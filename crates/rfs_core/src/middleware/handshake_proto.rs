//! Module for [HandshakeProto]

use std::backtrace::BacktraceStatus;
use std::net::Ipv4Addr;
use std::{io, net::SocketAddrV4, time::Duration};

use async_trait::async_trait;
use futures::FutureExt;
use tokio::net::{ToSocketAddrs, UdpSocket};

use crate::fsm::TransitableState;
use crate::ser_de::ByteViewer;
use crate::{fsm, middleware::sockaddr_to_v4};

use super::{deserialize_primary, serialize_primary, TransmissionProtocol};
use super::{hash_primary, TransmissionPacket};

/// This protocol ensures that every sent packet from the source must be acknowledged by the sink.
/// Timeouts and retries are fully implmented.
///
/// This protocol is not restricted by the UDP data limit.
/// In other words, it supports the transmission of an arbitrary number of bytes.
#[derive(Clone, Debug)]
pub struct HandshakeProto {
    // rx_state: HandshakeRx, // marker: marker::PhantomData<P>,
}

#[async_trait]
trait PerformStateAction {
    async fn perform_action(&self);
}

/// Transmitter states
#[derive(Clone, Copy, Debug, Default)]
enum HandshakeTx {
    #[default]
    SendAddressChange,

    /// tx sending packets, hands over control to rx
    Transmit,

    /// tx complete
    Complete,
}

/// State transition events for [HandshakeTx]
#[derive(Clone, Copy, Debug)]
enum HandshakeTxEvent {
    /// tx has sent the new address to rx
    SendNewAddr,

    /// tx has received a new destination address to send to
    ReceiveNewAddr,

    /// Last packet has been acknowledged
    AcknowledgeLast,
}

/// Receiver states
#[derive(Clone, Copy, Debug, Default)]
enum HandshakeRx {
    /// rx awaiting a change in address
    #[default]
    AwaitAddressChange,

    /// rx receiving packets, takes control over tx
    Receive,

    /// rx complete
    Complete,
}

/// State transition events for [HandshakeRx]
#[derive(Clone, Copy, Debug)]
enum HandshakeRxEvent {
    /// The new address is sent back to tx
    SendNewAddr,

    /// All packets received
    ReceivedAll,
}

// state transitions for the transmitter
fsm::state_transitions! {
    type State = HandshakeTx;
    type Event = HandshakeTxEvent;

    SendAddressChange + SendNewAddr => Transmit;
    Transmit + AcknowledgeLast => Complete;

    // on receiving a repeat request, go back to the previous state
    Transmit + ReceiveNewAddr => SendAddressChange;
}

// states transitions for the receiver
fsm::state_transitions! {
   type State = HandshakeRx;
   type Event = HandshakeRxEvent;

    AwaitAddressChange + SendNewAddr => Receive;
    Receive + ReceivedAll => Complete;
}

/// Generate a new new UDP socket bound to an OS-assigned port.
async fn new_socket_from_existing(sock: &UdpSocket) -> io::Result<UdpSocket> {
    let reference = sockaddr_to_v4(sock.local_addr()?)?;
    let addr = reference.ip();

    let sock = UdpSocket::bind(SocketAddrV4::new(addr.to_owned(), 0)).await?;

    Ok(sock)
}

impl HandshakeProto {
    /// This is a conservative limit on the max packet size
    const MAX_PACKET_PAYLOAD_SIZE: usize = 51_200;

    /// Sends something repeatedly until a response is received.
    /// The max payload this method can accept is 65507 bytes.
    async fn send_and_recv<A: ToSocketAddrs>(
        sock: &UdpSocket,
        target: A,
        payload: &[u8],
        timeout: Duration,
        mut retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)> {
        if payload.len() > 65_507 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "payload exceeds max UDP data size",
            ));
        }

        loop {
            let _ = sock.send_to(&payload, &target).await?;

            // send the request until the remote acknowledges the update
            // or when the connection times out
            tokio::select! {
                biased;

                res = async {
                    let mut buf = [0_u8; 65535];
                    let s = sock.recv_from(&mut buf).await;

                    s.and_then(|(size, addr)| {

                        let bytes = &buf[..size];

                        let v4_addr = sockaddr_to_v4(addr)?;
                        Ok((v4_addr, bytes.to_vec()))

                        // if size != payload.len() {
                        //     Err(io::Error::new(io::ErrorKind::InvalidData, format!("data not sent completely. Have {}, sent {}", payload.len(), size)))
                        // } else  {

                        // }
                    })

                }.fuse() => {
                    match res {
                        Ok((addr, d)) => {
                            break Ok((addr, d))
                        },
                        Err(e) => {
                            log::error!("{}", e);
                            retries -= 1;
                        },
                    }
                },

                _ = async {
                    tokio::time::sleep(timeout).await
                }.fuse() => {

                    log::error!("connection timed out. retries left: {}", retries);

                    match retries {
                        0 => break Err(io::Error::new(io::ErrorKind::TimedOut, "connection timed out while waiting for response")),
                        _ => retries -= 1,
                    }
                    continue;
                }

            }
        }
    }

    /// Sends a packet out with a given sequence number.
    /// The same packet will be continuously sent until the receiver has acknowledged the sequence number
    /// and sent a reply. The last packet is handled differently than the others.
    ///
    /// The same restrictions from `send_and_recv` apply here.
    ///
    /// Retries apply for both sequence number.
    async fn send_and_recv_sequence<A: ToSocketAddrs>(
        sock: &UdpSocket,
        sequence_number: u32,
        last: bool,
        target: A,
        payload: &[u8],
        timeout: Duration,
        retries: u8,
    ) -> io::Result<()> {
        // Self::send_bytes(sock, target, payload, timeout, retries)

        let mut outer_retries = retries;

        loop {
            if outer_retries == 0 {
                break Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "max retries exceeded",
                ));
            }

            let hash = hash_primary(&payload);

            let packet = TransmissionPacket::Data {
                seq: sequence_number,
                hash,
                data: payload.to_vec(),
                last,
            };

            let ser_packet = serialize_primary(&packet).expect("serialization must not fail");

            let (_, resp) =
                Self::send_and_recv(sock, &target, &ser_packet, timeout, retries).await?;

            let payload: TransmissionPacket = deserialize_primary(&resp).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "deserialization failed")
            })?;

            match (last, payload) {
                // return after ack
                (false, TransmissionPacket::Seq(num)) => {
                    match num as i32 - sequence_number as i32 {
                        0 => (), // retry transmission
                        1 => break Ok(()),
                        other => {
                            break Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                format!(
                                    "invalid sequence number. expected {}, got {}",
                                    sequence_number + 1,
                                    other
                                ),
                            ))
                        }
                    }
                }

                // last packet acknowledged, final ack
                (true, TransmissionPacket::Complete) => {
                    break Self::transmit_final_ack(sock, target, timeout, retries).await
                }

                _ => (),
            }

            outer_retries -= 1;
        }
    }

    /// The final transmission in a request-ack cycle is special.
    ///
    /// This method implements the following logic:
    /// - transmit the [`TransmissionPacket::Complete`] variant
    /// - select: timeout to elapse or an incoming packet
    ///
    /// Handle each case
    async fn transmit_final_ack<A: ToSocketAddrs>(
        sock: &UdpSocket,
        target: A,
        timeout: Duration,
        retries: u8,
    ) -> io::Result<()> {
        const ACK_PACKET: TransmissionPacket = TransmissionPacket::Complete;
        let ack_payload = serialize_primary(&ACK_PACKET).expect("serialization must not fail");

        loop {
            log::debug!("transmitting final packet");
            let _ = sock.send_to(&ack_payload, &target).await?;

            tokio::select! {

                // if the timeout elapses and no further response is received, the packet is assumed to
                // be received
                _ = async {
                    tokio::time::sleep(timeout).await
                }.fuse() => {
                    break Ok(())
                }

                // if a `complete` packet is received first, send the packets again
                res = async {
                    let mut buf = [0_u8; 100];
                    let res = sock.recv_from(&mut buf).await;

                    res.and_then(|(size, _)| {
                        Ok(buf[..size].to_vec())
                    })

                }.fuse() => {
                    log::error!("received duplicate");

                    let data = res?;
                    let packet = deserialize_primary(&data).map_err(|_| io::Error::new (io::ErrorKind::InvalidData, "deserialization failed"))?;

                    match packet {
                        TransmissionPacket::Complete => (),
                        other => break Err(io::Error::new(io::ErrorKind::InvalidInput, format!("expected a transmission complete packet, got {:?}", other)))
                    }
                }

            }
        }
    }
}

// tx impls
impl HandshakeProto {
    /// Sends the new address over to rx
    async fn send_address_change<A: ToSocketAddrs>(
        &mut self,
        state: &mut HandshakeTx,
        sock: &UdpSocket,
        target: A,
        new_addr: SocketAddrV4,
        new_target: &mut Option<SocketAddrV4>,
        timeout: Duration,
        retries: u8,
    ) -> io::Result<()> {
        let payload = TransmissionPacket::SwitchToAddress(new_addr);
        let ser_payload = serialize_primary(&payload).expect("serialization must not fail");

        let (_, bytes) = Self::send_and_recv(sock, target, &ser_payload, timeout, retries).await?;

        let resp: TransmissionPacket = deserialize_primary(&bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "deserialization failed"))?;

        if let TransmissionPacket::SwitchToAddress(n_target) = resp {
            // modify state
            state.ingest(HandshakeTxEvent::ReceiveNewAddr);

            *new_target = Some(n_target);
        } else {
        }

        Ok(())
    }

    /// Waits for rx to send over the new address. Error handling is done by rx.
    /// Modifies the given target to this new address.
    async fn transmit_data(
        &mut self,
        state: &mut HandshakeTx,
        sock: &UdpSocket,
        target: SocketAddrV4,
        payload: &[u8],
    ) -> io::Result<()> {
        let num_segments = payload.len().div_ceil(Self::MAX_PACKET_PAYLOAD_SIZE);

        // wait for a sequence number and send that packet out
        loop {
            let mut seq_buf = [0_u8; 1_000];

            let (size, _) = sock.recv_from(&mut seq_buf).await?;

            let data = &seq_buf[..size];
            let packet: TransmissionPacket = deserialize_primary(&data).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "deserialization failed")
            })?;

            match packet {
                TransmissionPacket::SwitchToAddress(_) => {
                    // switch to the previous state and exit
                    state.ingest(HandshakeTxEvent::ReceiveNewAddr);
                    return Ok(());
                }
                TransmissionPacket::Seq(seq_num) => {
                    let start = seq_num as usize * Self::MAX_PACKET_PAYLOAD_SIZE;
                    let packet = match seq_num as usize + 1 == num_segments {
                        true => {
                            let packet_data = &payload[start..];

                            TransmissionPacket::Data {
                                seq: seq_num as u32,
                                hash: hash_primary(&packet_data),
                                data: packet_data.to_vec(),
                                last: true,
                            }
                        }
                        false => {
                            let packet_data =
                                &payload[start..(start + Self::MAX_PACKET_PAYLOAD_SIZE)];

                            TransmissionPacket::Data {
                                seq: seq_num as u32,
                                hash: hash_primary(&packet_data),
                                data: packet_data.to_vec(),
                                last: false,
                            }
                        }
                    };

                    let ser_packet =
                        serialize_primary(&packet).expect("serialization must not fail");

                    sock.send_to(&ser_packet, target).await?;
                }
                // update state and exit
                TransmissionPacket::Complete => {
                    state.ingest(HandshakeTxEvent::AcknowledgeLast);
                    return Ok(());
                }
                // do nothing for the rest
                _ => (),
            }
        }
    }
}

// rx impls
impl HandshakeProto {
    // await the new address and send back a new address
    async fn await_address_change(
        &mut self,
        state: &mut HandshakeRx,
        sock: &UdpSocket,
        new_target: &mut Option<SocketAddrV4>,
        new_address: SocketAddrV4,
    ) -> io::Result<()> {
        let mut recv_buf = [0_u8; 1000];

        loop {
            let (size, _) = sock.recv_from(&mut recv_buf).await?;

            let packet: TransmissionPacket =
                deserialize_primary(&recv_buf[..size]).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "deserialization faield")
                })?;

            match packet {
                TransmissionPacket::SwitchToAddress(new_addr) => {
                    *new_target = Some(new_addr);
                    break;
                }
                // continue listening
                _ => (),
            }
        }

        let packet = TransmissionPacket::SwitchToAddress(new_address);
        let ser_packet = serialize_primary(&packet).expect("serialization must not fail");

        sock.send_to(
            &ser_packet,
            new_target.expect("target address must be valid"),
        )
        .await?;

        state.ingest(HandshakeRxEvent::SendNewAddr);

        Ok(())
    }
}

#[async_trait]
impl TransmissionProtocol for HandshakeProto {
    async fn send_bytes<A>(
        &mut self,
        sock: &UdpSocket,
        target: A,
        payload: &[u8],
        timeout: Duration,
        retries: u8,
    ) -> io::Result<usize>
    where
        A: ToSocketAddrs + std::marker::Send + std::marker::Sync,
    {
        // first we will switch target sockets so that we don't block the main process
        // from receiving requests

        // state control variable
        let mut tx_state = HandshakeTx::default();
        let mut tx_target: Option<SocketAddrV4> = None;

        let tx_sock = new_socket_from_existing(sock).await?;

        loop {
            match tx_state {
                HandshakeTx::SendAddressChange => {
                    self.send_address_change(
                        &mut tx_state,
                        &tx_sock,
                        &target, // address changes are sent to the existing address
                        sockaddr_to_v4(tx_sock.local_addr()?)?,
                        &mut tx_target,
                        timeout,
                        retries,
                    )
                    .await?
                }
                HandshakeTx::Transmit => {
                    self.transmit_data(
                        &mut tx_state,
                        &tx_sock,
                        tx_target.expect("tx target not set"),
                        payload,
                    )
                    .await?
                }

                HandshakeTx::Complete => break,
            }
        }

        // temp break point
        return Ok(0);

        let template_addr = {
            let a = sock.local_addr()?;
            let mut addr = sockaddr_to_v4(a)?;

            addr.set_port(0);

            addr
        };

        // create a new socket and establish a new connecion
        let (tx_sock, tx_target) = {
            let new_sock = UdpSocket::bind(template_addr).await?;
            let new_bind_addr = sockaddr_to_v4(new_sock.local_addr()?)?;

            let ser_payload =
                serialize_primary(&TransmissionPacket::SwitchToAddress(new_bind_addr))
                    .expect("serialization must not fail");

            // get the new remote address
            let remote_addr = loop {
                let (_, data) =
                    Self::send_and_recv(&new_sock, &target, &ser_payload, timeout, retries).await?;

                let recv_packet: TransmissionPacket = deserialize_primary(&data).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "deserialization failed")
                })?;

                if let TransmissionPacket::SwitchToAddress(a) = recv_packet {
                    break a;
                }
            };

            log::debug!("tx changing bind addresss to {}", new_bind_addr);
            log::debug!("tx new target {}", remote_addr);

            new_sock.connect(remote_addr).await?;
            (new_sock, remote_addr)
        };

        let mut curr_chunk = 0;

        let mut payload_view = ByteViewer::from_slice(payload);

        // send an ack to recv to know that we are ready to transmit
        loop {
            break;
        }

        // all packets except last
        // we'll send a reasonably large amount of data in each go
        while payload_view.distance_to_end() >= Self::MAX_PACKET_PAYLOAD_SIZE {
            let payload = payload_view.next_bytes(Self::MAX_PACKET_PAYLOAD_SIZE, true);

            Self::send_and_recv_sequence(
                &tx_sock,
                curr_chunk as u32,
                false,
                &tx_target,
                payload,
                timeout,
                retries,
            )
            .await?;

            curr_chunk += 1;
        }

        // last packet is always sent
        let last_data = payload_view.next_bytes(payload_view.distance_to_end(), true);

        Self::send_and_recv_sequence(
            &tx_sock,
            curr_chunk as u32,
            true,
            &tx_target,
            last_data,
            timeout,
            retries,
        )
        .await?;

        Ok(payload.len())
    }

    async fn recv_bytes(
        &mut self,
        sock: &UdpSocket,
        timeout: Duration,
        retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)> {
        // state control
        let mut rx_state = HandshakeRx::default();
        let mut rx_target: Option<SocketAddrV4> = None;

        let rx_sock = new_socket_from_existing(sock).await?;

        loop {
            match rx_state {
                HandshakeRx::AwaitAddressChange => {
                    self.await_address_change(
                        &mut rx_state,
                        sock, // we need to use the existing socket when listening for these changes
                        &mut rx_target,
                        sockaddr_to_v4(rx_sock.local_addr()?)?,
                    )
                    .await?
                }
                HandshakeRx::Receive => todo!(),
                HandshakeRx::Complete => todo!(),
            }
        }

        // temp break point
        return Ok((todo!(), todo!()));

        let mut buf = [0_u8; 65535];
        let (size, original_target) = sock.recv_from(&mut buf).await?;

        let payload: TransmissionPacket = deserialize_primary(&buf[..size])
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Deserializtion failed"))?;

        // send the new address and await an ack
        let (tx_sock, tx_target) = if let TransmissionPacket::SwitchToAddress(remote_addr) = payload
        {
            let template_addr = {
                let a = sock.local_addr()?;
                let mut addr = sockaddr_to_v4(a)?;

                addr.set_port(0);

                addr
            };
            let new_sock = UdpSocket::bind(template_addr).await?;
            let new_bind_addr = sockaddr_to_v4(new_sock.local_addr()?)?;

            log::debug!("rx changing bind addresss to {}", new_bind_addr);
            log::debug!("rx new target {}", remote_addr);

            new_sock.connect(remote_addr).await?;

            let packet = TransmissionPacket::SwitchToAddress(new_bind_addr);
            let ser_packet = serialize_primary(&packet).expect("serialization must not fail");

            log::debug!("sending new address to tx");
            let (_, ack) =
                Self::send_and_recv(&new_sock, remote_addr, &ser_packet, timeout, retries).await?;

            let ack_packet: TransmissionPacket = deserialize_primary(&ack).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "deserialization failed")
            })?;

            if let TransmissionPacket::Ack(_) = ack_packet {
                (new_sock, remote_addr)
            } else {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected ack"));
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected address swtich",
            ));
        };

        // actual data received
        let mut data_vec = Vec::<u8>::new();

        // receive and reconstruct data
        let mut seq_num = 0;
        loop {
            log::debug!("requesting sequence {}", 0);

            let seq_packet = TransmissionPacket::Seq(seq_num);
            let ser_seq = serialize_primary(&seq_packet).expect("serialization must not fail");
            let (_, data) =
                Self::send_and_recv(&tx_sock, &tx_target, &ser_seq, timeout, retries).await?;

            let recv_packet: TransmissionPacket = deserialize_primary(&data).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "deserialization failed")
            })?;

            let (data, last) = if let TransmissionPacket::Data {
                seq,
                hash,
                data,
                last,
            } = recv_packet
            {
                match (seq == seq_num as u32, hash == hash_primary(&data)) {
                    (true, true) => (),
                    // re-transmit packet
                    _ => continue,
                }

                (data, last)
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "expected data packet",
                ));
            };

            data_vec.extend(data);

            if last {
                log::debug!("sequence rx complete");
                break;
            }

            seq_num += 1;
        }

        Ok((sockaddr_to_v4(original_target)?, data_vec))
    }
}