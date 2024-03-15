//! Module for [HandshakeProto]
#![allow(unused)]

use std::fmt::Debug;
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
    // / tx has sent the new address to rx
    // SendNewAddr,
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

    SendAddressChange + ReceiveNewAddr => Transmit;
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

        log::debug!("tx sending new tx address ({})", new_addr);

        let (_, bytes) = Self::send_and_recv(sock, target, &ser_payload, timeout, retries).await?;

        let resp: TransmissionPacket = deserialize_primary(&bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "deserialization failed"))?;

        if let TransmissionPacket::SwitchToAddress(n_target) = resp {
            log::debug!("tx received new rx address: {}", n_target);

            // modify state
            state.ingest(HandshakeTxEvent::ReceiveNewAddr);

            *new_target = Some(n_target);
        } else {
            log::debug!("tx received incorrect packet: {:?}", resp);
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
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "tx deserialization failed of Transmission packet",
                )
            })?;

            match packet {
                TransmissionPacket::SwitchToAddress(_) => {
                    // switch to the previous state and exit
                    state.ingest(HandshakeTxEvent::ReceiveNewAddr);
                    return Ok(());
                }
                TransmissionPacket::Seq(seq_num) => {
                    log::debug!("tx sending sequence {}", seq_num);
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
    /// await the new address and send back a new address.
    ///
    /// The original address of the tx is returned.
    async fn await_address_change(
        &mut self,
        state: &mut HandshakeRx,
        sock: &UdpSocket,
        new_target: &mut Option<SocketAddrV4>,
        new_address: SocketAddrV4,
    ) -> io::Result<SocketAddrV4> {
        let mut recv_buf = [0_u8; 1000];

        let addr = loop {
            let (size, addr) = sock.recv_from(&mut recv_buf).await?;

            let packet: TransmissionPacket =
                deserialize_primary(&recv_buf[..size]).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "deserialization failed of TransmissionPacket when awaiting new address",
                    )
                })?;

            match packet {
                TransmissionPacket::SwitchToAddress(new_addr) => {
                    log::debug!("rx changing target addresss to {}", new_addr);
                    *new_target = Some(new_addr);
                    break addr;
                }

                // continue listening
                _ => (),
            }
        };

        let packet = TransmissionPacket::SwitchToAddress(new_address);
        let ser_packet = serialize_primary(&packet).expect("serialization must not fail");

        sock.send_to(&ser_packet, addr).await?;

        state.ingest(HandshakeRxEvent::SendNewAddr);

        Ok(sockaddr_to_v4(addr)?)
    }

    // receive loop
    async fn receive(
        &mut self,
        state: &mut HandshakeRx,
        sock: &UdpSocket,
        target: SocketAddrV4,
        rx_data: &mut Vec<u8>,
        timeout: Duration,
    ) -> io::Result<()> {
        let mut sequence_num = 0;

        loop {
            let mut seq_buf = [0_u8; 65535];

            // send out ack for packet number
            let packet = TransmissionPacket::Seq(sequence_num);
            let ser_packet = serialize_primary(&packet).expect("serialization must not fail");

            log::debug!("rx requesting sequence {}", sequence_num);
            let _ = sock.send_to(&ser_packet, target).await?;

            // receive with timeout
            let size = tokio::select! {
                biased;

                res = async {
                    sock.recv_from(&mut seq_buf).await
                }.fuse() => {
                    res.and_then(|(size, _)| {
                        Ok(size)
                    })
                },

                _ = async {
                    tokio::time::sleep(timeout).await
                }.fuse() => {
                    log::error!("timeout elapsed");
                    continue;
                }
            }?;

            let packet: TransmissionPacket =
                deserialize_primary(&seq_buf[..size]).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "rx deserialization failed of TransmissionPacket",
                    )
                })?;

            // {
            //     Ok(res) => res,
            //     Err(e) => {
            //         log::error!("deserialization failed: {:?}", e);
            //         // retry
            //         continue;
            //     },
            // };
            match packet {
                TransmissionPacket::Data {
                    seq,
                    hash,
                    data,
                    last,
                } => {
                    match (seq == sequence_num as u32, hash == hash_primary(&data)) {
                        (true, true) => {
                            log::debug!("rx received sequence {}", sequence_num);
                            rx_data.extend(data);
                            sequence_num += 1;
                        }
                        // re-transmit packet
                        _ => {
                            log::debug!("rx requires re-transmitting sequence {}", sequence_num);
                            continue;
                        }
                    }

                    // correctly received last packet and exit
                    if last {
                        log::debug!("rx received last sequence");
                        state.ingest(HandshakeRxEvent::ReceivedAll);
                        break;
                    }
                }
                TransmissionPacket::SwitchToAddress(_) => {
                    state.ingest(HandshakeRxEvent::SendNewAddr);
                    break;
                }
                TransmissionPacket::Complete => {
                    state.ingest(HandshakeRxEvent::ReceivedAll);
                    break;
                }

                TransmissionPacket::Ack(_) | TransmissionPacket::Seq(_) => {
                    unimplemented!("cases are never handled by rx")
                }
            }
        }

        Ok(())
    }

    async fn complete(&mut self, sock: &UdpSocket, target: SocketAddrV4) -> io::Result<()> {
        let packet = TransmissionPacket::Complete;
        let ser_packet = serialize_primary(&packet).expect("serialization must not fail");

        // send w/o expecting a response
        sock.send_to(&ser_packet, target).await?;

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
            log::debug!("tx state: {:?}", tx_state);

            match tx_state {
                HandshakeTx::SendAddressChange => {
                    self.send_address_change(
                        &mut tx_state,
                        &sock,
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

                HandshakeTx::Complete => {
                    break;
                }
            }
        }

        // temp break point
        return Ok(payload.len());
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

        // this is the original address of tx
        let mut rx_source: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0);

        let mut rx_data = Vec::new();

        loop {
            log::debug!("rx state: {:?}", rx_state);

            match rx_state {
                HandshakeRx::AwaitAddressChange => {
                    rx_source = self
                        .await_address_change(
                            &mut rx_state,
                            sock, // we need to use the existing socket when listening for these changes
                            &mut rx_target,
                            sockaddr_to_v4(rx_sock.local_addr()?)?,
                        )
                        .await?
                }
                HandshakeRx::Receive => {
                    self.receive(
                        &mut rx_state,
                        &rx_sock,
                        rx_target.expect("no target to receive from"),
                        &mut rx_data,
                        timeout,
                    )
                    .await?
                }
                HandshakeRx::Complete => {
                    self.complete(&sock, rx_target.expect("no target to receive from"))
                        .await?;
                    break;
                }
            }
        }

        // let dummy_addr = rx_target.expect("no target to receive from");
        return Ok((rx_source, rx_data));
    }
}
