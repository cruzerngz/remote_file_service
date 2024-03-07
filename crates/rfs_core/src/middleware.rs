//! This module contains the client and server side
//! objects that transmit the contents of method invocations
//! over the network.
// #![allow(unused)]

mod blob_trx;
mod callback;
mod context_manager;
mod dispatch;

use futures::FutureExt;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use std::{default, io, marker};
use std::{fmt::Debug, net::Ipv4Addr};
use tokio::net::{ToSocketAddrs, UdpSocket};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use context_manager::*;
pub use dispatch::*;

use crate::fsm::{self, TransitableState};
// define the serde method here once for use by submodules
use crate::ser_de::serialize_packed as serialize_primary;
use crate::ser_de::{deserialize_packed as deserialize_primary, ByteViewer};

/// Max payload size
const BYTE_BUF_SIZE: usize = 65535;

/// Method invocation errors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InvokeError {
    /// The remote is unable to find a handler for the given payload.
    ///
    /// This should be the most common error returned from an invocation.
    HandlerNotFound,

    /// The method signature of the response does not match
    /// the payload.
    SignatureNotMatched,

    /// The context manager is unable to get a response from the remote
    RequestTimedOut,

    /// Deserialization of the payload failed
    DeserializationFailed,

    /// Connection to the remote was unsuccessful
    RemoteConnectionFailed,

    /// Unable to send data to the remote
    DataTransmissionFailed,

    /// Remote received an error
    RemoteReceiveError,

    /// Invalid data
    InvalidData,
}

/// Middleware-specific data sent between the context manager and the dispatcher
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MiddlewareData {
    /// Send a message to the remote, expects an echo
    Ping,

    /// Remote method invocation payload, request or response
    #[serde(with = "serde_bytes")]
    Payload(Vec<u8>),

    /// Remote callback payload
    #[serde(with = "serde_bytes")]
    Callback(Vec<u8>),

    /// Err messages go here
    Error(InvokeError),

    /// An acknowledgement from either end that a message has been received.
    /// The value sent is arbitrary, but should be used in a way to
    /// verify the success of a request.
    ///
    /// A hash of the bytes is transmitted back when using [RequestAckProto].
    Ack(u64),

    /// A size transmission. This can represent anything really.
    Size(usize),
    /// A no-op.
    NoOp,
}

/// Handle middleware messages, either from the client or remote.
pub trait HandleMiddleware {
    fn handle_middleware(&self, data: MiddlewareData) -> Self;
}

impl std::error::Error for InvokeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

// temp, display is debug
impl std::fmt::Display for InvokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Route and handle the bytes of a remote method invocation.
///
/// The method proceseses the bytes of a remote method invocation,
/// routes the bytes to the appropriate method call, and returns the
/// result.
#[async_trait]
pub trait PayloadHandler {
    async fn handle_payload(&mut self, payload_bytes: &[u8]) -> Result<Vec<u8>, InvokeError>;
}

/// Route and handle the bytes of a remote callback.
///
/// A socket is passed to into the callback method. This is used by the callback method when
/// sending the result of the callback to the client.
#[async_trait]
pub trait CallbackHandler {
    async fn handle_callback(
        &mut self,
        callback_bytes: &[u8],
        sock: UdpSocket,
    ) -> Result<Vec<u8>, InvokeError>;
}

/// This trait is implemented for types that provide socket addresses to bind to.
///
/// Socket reuse logic can be implemented for certain types.
#[async_trait]
pub trait SocketProvider: core::marker::Send + core::marker::Sync {
    /// Construct an instance of `Self` from a given address
    fn from_addr(a: Ipv4Addr) -> Self;

    /// Creates a new socket address to bind to, or reuses an existing one.
    async fn new_bind_sock(&mut self) -> io::Result<Arc<UdpSocket>>;

    /// Free a socket address.
    ///
    /// In the default impl, this is a no-op
    #[allow(unused_variables)]
    async fn free_sock(&mut self, s: Arc<UdpSocket>) -> io::Result<()> {
        Ok(())
    }
}

/// Serve requests by binding to a port.
///
/// The default implementation does not cache requests.
// #[async_trait]
// pub trait RequestServer: PayloadHandler {
//     async fn serve(&mut self, addr: std::net::SocketAddrV4) {
//         todo!()
//     }
// }

// impl<T> RequestServer for T where T: PayloadHandler {}

/// This macro implements [`PayloadHandler`] with a specified number of routes.
///
/// ```no
/// /// Server definition (and any fields)
/// #[derive(Debug)]
/// pub struct Server;
///
/// // the remote interface implementation
/// #[async_trait::async_trait]
/// impl ImmutableFileOps for Server {
///     /// Read the contents of a file.
///     async fn read_file(&mut self, path: PathBuf, offset: Option<usize>) -> Vec<u8> {
///         // ... implementation
///         todo!()
///     }
/// }
///
///
/// payload_handler! {
///     Server,
///     // we use the '`method_name`_payload' method.
///      ImmutableFileOpsReadFile => ImmutableFileOps::read_file_payload
///     // an arbitrary number of paths can be added
/// }
/// ```
#[macro_export]
macro_rules! payload_handler {
    ($server_ty: ty,
        $($payload_ty: ty => $trait: ident :: $method: ident),+,
    ) => {
        #[async_trait::async_trait]
        impl PayloadHandler for $server_ty {
            async fn handle_payload(&mut self, payload_bytes: &[u8]) -> Result<Vec<u8>, rfs::middleware::InvokeError> {

                $(if payload_bytes.starts_with(
                        <$payload_ty as rfs::RemoteMethodSignature>::remote_method_signature(),
                    ) {

                        log::info!("{}", std::str::from_utf8(<$payload_ty as rfs::RemoteMethodSignature>::remote_method_signature()).unwrap());

                        let payload =
                            <$payload_ty as rfs::RemotelyInvocable>::process_invocation(payload_bytes)?;
                        let res = self.$method(payload).await;
                        let resp = <$payload_ty>::Response(res);
                        let export_payload = rfs::RemotelyInvocable::invoke_bytes(&resp);
                        return Ok(export_payload);
                    })+

                // no matches, error out
                Err(rfs::middleware::InvokeError::HandlerNotFound)
            }
        }
    };
}

/// Recommended payload to be sent between implementors of [`TransmissionProtocol`].
///
/// There is no requirement to use this data structure, or all it's variants/fields.
/// Each implementor is responsible for how data is transmitted.
///
/// Implementors can opt to send raw bytes as well.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransmissionPacket {
    /// Data payload
    Data {
        /// sequence number
        seq: u32,

        /// Hash value of bytes
        hash: u64,

        #[serde(with = "serde_bytes")]
        data: Vec<u8>,

        /// Indicates if this is the last packet
        last: bool,
    },

    /// For receipients of this packet, switch transmissions to this new target
    SwitchToAddress(SocketAddrV4),

    /// A request for a sequence number
    Seq(u64),

    /// An ack packet, along with a number.
    /// The meaning of the number sent within depends on the implementor of the protocol.
    Ack(u64),

    /// Signals the completion of the transfer
    Complete,
}

/// Types that implement this trait can be plugged into [`ContextManager`] and [`Dispatcher`].
#[async_trait]
pub trait TransmissionProtocol: core::marker::Send + core::marker::Sync {
    /// Send bytes to the remote. Any fault-tolerant logic should be implemented here.
    async fn send_bytes<A>(
        &mut self,
        sock: &UdpSocket,
        target: A,
        payload: &[u8],
        timeout: Duration,
        retries: u8,
    ) -> io::Result<usize>
    where
        A: ToSocketAddrs + std::marker::Send + std::marker::Sync;

    /// Wait for a UDP packet. Returns the packet source and data.
    async fn recv_bytes(
        &mut self,
        sock: &UdpSocket,
        timeout: Duration,
        retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)>;
}
/// Converts a socket address to a V4 one.
/// V6 addresses will return an error.
fn sockaddr_to_v4(addr: SocketAddr) -> io::Result<SocketAddrV4> {
    match addr {
        SocketAddr::V4(a) => Ok(a),
        SocketAddr::V6(_) => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "IPv6 addresses are not supported",
        )),
    }
}

/// A simple version of [HandshakeProto]. Every sent item needs an ack back.
#[derive(Clone, Debug, Default)]
pub struct SimpleHandshakeProto;

#[async_trait]
impl TransmissionProtocol for SimpleHandshakeProto {
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
        todo!()
    }

    /// Wait for a UDP packet. Returns the packet source and data.
    async fn recv_bytes(
        &mut self,
        sock: &UdpSocket,
        timeout: Duration,
        retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)> {
        todo!()
    }
}

/// This protocol ensures that every sent packet from the source must be acknowledged by the sink.
/// Timeouts and retries are fully implmented.
///
/// This protocol is not restricted by the UDP data limit.
/// In other words, it supports the transmission of an arbitrary number of bytes.
#[derive(Clone, Debug)]
pub struct HandshakeProto {
    state: HandshakeStates, // marker: marker::PhantomData<P>,
}

#[derive(Clone, Debug, Default)]
enum HandshakeStates {
    /// When waiting, a request to switch target addresses can be performed.
    #[default]
    Waiting,

    Sending,

    Receiving,

    Complete,
}

#[derive(Clone, Debug)]
enum HandshakeEvent {
    SomeEvent,
    OtherEvent,
}

fsm::state_transitions! {
    state: HandshakeStates;
    event: HandshakeEvent;


}

// impl TransitableState for HandshakeStates {
//     type Event = HandshakeEvent;

//     fn ingest(&mut self, transition: Self::Event) {
//         // fsm::state_transition! {
//         //     self: self;
//         //     transition: transition;

//         //     Complete + SomeEvent | OtherEvent => Waiting

//         // }
//     }
// }

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

/// A faulty version of [RequestAckProto].
///
/// This protocol may drop packets on transmission.
/// The packet drop probabilty is specified in the const generic.
///
/// The proto will fail to transmit every 1 in `FRAC` invocations on average.
#[derive(Clone, Debug)]
pub struct FaultyRequestAckProto<const FRAC: u32>;

#[async_trait]
impl<const FRAC: u32> TransmissionProtocol for FaultyRequestAckProto<FRAC> {
    async fn send_bytes<A>(
        &mut self,
        sock: &UdpSocket,
        target: A,
        payload: &[u8],
        timeout: Duration,
        mut retries: u8,
    ) -> io::Result<usize>
    where
        A: ToSocketAddrs + std::marker::Send + std::marker::Sync,
    {
        // let mut res: io::Result<usize> = Err(io::Error::new(
        //     io::ErrorKind::TimedOut,
        //     "connection timed out",
        // ));

        // while retries != 0 {
        //     log::debug!("sending data to target");

        //     // occasionally err
        //     let send_size = match probability_frac(FRAC) {
        //         true => {
        //             log::error!("simulated packet drop");
        //             payload.len()
        //         }
        //         false => sock.send_to(payload, &target).await?,
        //     };

        //     let mut buf = [0_u8; 100];

        //     tokio::select! {
        //         biased;

        //         recv_res = async {
        //             sock.recv(&mut buf).await
        //         }.fuse() => {
        //             log::debug!("response received from target");

        //             let recv_size = recv_res?;
        //             let slice = &buf[..recv_size];

        //             let de: MiddlewareData = deserialize_primary(slice).unwrap();
        //             let hash = if let MiddlewareData::Ack(h) = de {
        //                 h
        //             } else {
        //                 res = Err(io::Error::new(io::ErrorKind::InvalidData, "expected Ack"));
        //                 break;
        //             };

        //             if hash == hash_primary(&payload) {
        //                 res = Ok(send_size);
        //             } else {
        //                 res = Err(io::Error::new(io::ErrorKind::InvalidData, "Ack does not match"));
        //             }

        //             break;
        //         },
        //         _ = async {
        //             tokio::time::sleep(timeout).await;
        //         }.fuse() => {
        //             retries -= 1;
        //             log::debug!("response timed out. retries remaining: {}", retries);

        //             continue;
        //         }
        //     }
        // }

        // res

        todo!()
    }

    async fn recv_bytes(
        &mut self,
        sock: &UdpSocket,
        timeout: Duration,
        retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)> {
        todo!()
    }
}

/// Returns the outcome of the probability of getting `1` in `frac`.
fn probability_frac(frac: u32) -> bool {
    let rand_num: u64 = rand::random();
    let threshold = u64::MAX / frac as u64;

    rand_num < threshold
}

/// UDP-like protocol, packets are sent to the destination without checking if they have been received.
///
/// As this sends all data in a single UDP packet, the max payload size is `65507` bytes.
#[derive(Clone, Debug)]
pub struct SimpleProto;

#[async_trait]
impl TransmissionProtocol for SimpleProto {
    async fn send_bytes<A>(
        &mut self,
        sock: &UdpSocket,
        target: A,
        payload: &[u8],
        _timeout: Duration,
        _retries: u8,
    ) -> io::Result<usize>
    where
        A: ToSocketAddrs + std::marker::Send + std::marker::Sync,
    {
        sock.send_to(payload, target).await
    }

    async fn recv_bytes(
        &mut self,
        sock: &UdpSocket,
        timeout: Duration,
        retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)> {
        let mut buf = [0_u8; 65535];

        let (size, addr) = sock.recv_from(&mut buf).await?;

        let addr = sockaddr_to_v4(addr)?;

        Ok((addr, buf[..size].to_vec()))
    }
}

/// The primary hash method used for verifying the integrity of data
fn hash_primary<T: Hash>(item: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    item.hash(&mut hasher);

    hasher.finish()
}

impl From<io::Error> for InvokeError {
    fn from(value: io::Error) -> Self {
        log::error!("error kind: {:?}", value.kind());

        match value.kind() {
            io::ErrorKind::NotFound => todo!(),
            io::ErrorKind::PermissionDenied => todo!(),
            io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::NotConnected
            | io::ErrorKind::AddrInUse
            | io::ErrorKind::AddrNotAvailable
            | io::ErrorKind::BrokenPipe => InvokeError::DataTransmissionFailed,
            io::ErrorKind::AlreadyExists => todo!(),
            io::ErrorKind::WouldBlock => todo!(),
            io::ErrorKind::InvalidInput | io::ErrorKind::InvalidData => InvokeError::InvalidData,
            io::ErrorKind::TimedOut => InvokeError::RequestTimedOut,
            io::ErrorKind::WriteZero => todo!(),
            io::ErrorKind::Interrupted => todo!(),
            io::ErrorKind::Unsupported => todo!(),
            io::ErrorKind::UnexpectedEof => todo!(),
            io::ErrorKind::OutOfMemory => todo!(),
            io::ErrorKind::Other => todo!(),
            _ => InvokeError::RequestTimedOut,
        }
    }
}

impl From<InvokeError> for io::Error {
    fn from(value: InvokeError) -> Self {
        match value {
            InvokeError::HandlerNotFound => todo!(),
            InvokeError::SignatureNotMatched => todo!(),
            InvokeError::RequestTimedOut => todo!(),
            InvokeError::DeserializationFailed => todo!(),
            InvokeError::RemoteConnectionFailed => todo!(),
            InvokeError::DataTransmissionFailed => todo!(),
            InvokeError::RemoteReceiveError => todo!(),
            InvokeError::InvalidData => todo!(),
        }
    }
}

/// Basic socket provider impl, no socket reuse
#[derive(Debug)]
pub struct BasicSockProvider {
    addr: Ipv4Addr,
}

#[async_trait]
impl SocketProvider for BasicSockProvider {
    fn from_addr(a: Ipv4Addr) -> Self {
        Self { addr: a }
    }

    async fn new_bind_sock(&mut self) -> io::Result<Arc<UdpSocket>> {
        Ok(Arc::new(
            UdpSocket::bind(SocketAddrV4::new(self.addr, 0)).await?,
        ))
    }
}

/// Maintains an internal pool of bound sockets
#[derive(Debug)]
pub struct SocketPool {
    addr: Ipv4Addr,

    /// The boolean field indicates if the current socket is in use
    sockets: HashMap<SocketAddrV4, (bool, Arc<UdpSocket>)>,
}

impl SocketPool {
    async fn create_new_sock(&mut self) -> io::Result<UdpSocket> {
        let sock = UdpSocket::bind(SocketAddrV4::new(self.addr, 0)).await?;

        Ok(sock)
    }

    /// Create a new socket and inserts it into the pool with use condition `cond`.
    async fn create_insert_new_sock(&mut self, in_use: bool) -> io::Result<Arc<UdpSocket>> {
        let sock = Arc::new(self.create_new_sock().await?);

        let a = match sock.local_addr()? {
            std::net::SocketAddr::V4(a) => a,
            std::net::SocketAddr::V6(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    "IPv6 addresses are not supported",
                ))
            }
        };

        self.sockets.insert(a, (in_use, sock.clone()));

        return Ok(sock);
    }
}

#[async_trait]
impl SocketProvider for SocketPool {
    fn from_addr(a: Ipv4Addr) -> Self {
        Self {
            addr: a,
            sockets: Default::default(),
        }
    }

    async fn new_bind_sock(&mut self) -> io::Result<Arc<UdpSocket>> {
        if self.sockets.len() == 0 {
            return Ok(self.create_insert_new_sock(true).await?);
        }

        let unused_sock = self
            .sockets
            .iter()
            .find_map(|(_, (in_use, sock))| match in_use {
                true => None,
                false => Some(sock.clone()),
            });

        match unused_sock {
            Some(s) => Ok(s),
            None => Ok(self.create_insert_new_sock(true).await?),
        }
    }

    async fn free_sock(&mut self, s: Arc<UdpSocket>) -> io::Result<()> {
        let addr = match s.local_addr()? {
            std::net::SocketAddr::V4(a) => a,
            std::net::SocketAddr::V6(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    "IPv6 addresses are not supported",
                ))
            }
        };

        let entry = self.sockets.get_mut(&addr);

        match entry {
            Some((in_use, _)) => {
                *in_use = false;
                Ok(())
            }
            // we are ok with an entry not existing
            None => Ok(()),
        }
    }
}

#[cfg(test)]
#[allow(unused)]
mod tests {

    use std::net::SocketAddrV4;

    use super::*;

    /// Tests the happy path for types that implement [TransmissionProtocol]
    #[tokio::test]
    async fn test_send_timeout() {
        std::env::set_var("RUST_LOG", "DEBUG");
        pretty_env_logger::init();

        // let sock = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        //     .await
        //     .unwrap();

        // let res = send_timeout(
        //     &sock,
        //     SocketAddrV4::new(Ipv4Addr::LOCALHOST, 10000),
        //     &[10, 10, 10, 10],
        //     Duration::from_secs(3),
        //     3,
        // )
        // .await;

        // assert!(matches!(res, Err(_)));
    }

    #[test]
    fn test_prob() {
        let frac = 10;

        let probs = (0..64)
            .into_iter()
            .map(|_| match probability_frac(frac) {
                true => 1,
                false => 0,
            })
            .collect::<Vec<_>>();

        let s: i32 = probs.iter().sum();

        println!("1 in {} yields {}", frac, s);
    }

    /// Transmit and receive some stuff
    async fn tx_rx<T: TransmissionProtocol + Clone + 'static>(
        mut proto: T,
        large: bool,
        timeout: Duration,
        retries: u8,
    ) {
        let data_size = match large {
            true => 60_000 * 10,
            false => 51_200,
        };

        let data_payload = (0..data_size)
            .into_iter()
            .map(|num| (num & 0b1) as u8)
            .collect::<Vec<_>>();

        let tx_sock = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();

        let rx_sock = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();

        let tx_target = rx_sock.local_addr().unwrap();
        let rx_target = tx_sock.local_addr().unwrap();

        let mut tx_proto = proto.clone();
        let mut rx_proto = proto.clone();

        let payload_clone = data_payload.clone();

        let rx_handle =
            tokio::spawn(async move { rx_proto.recv_bytes(&rx_sock, timeout, retries).await });

        let tx_handle = tokio::spawn(async move {
            tx_proto
                .send_bytes(&tx_sock, tx_target, &payload_clone, timeout, retries)
                .await
        });

        let tx_result = tx_handle
            .await
            .expect("unable to join task")
            .expect("transmission failed");

        let rx_result = rx_handle
            .await
            .expect("unable to join task")
            .expect("receive failed");

        assert_eq!(rx_result.1, data_payload);
    }

    #[tokio::test]
    async fn test_transmission_protocols() {
        std::env::set_var("RUST_LOG", "DEBUG");
        pretty_env_logger::init();

        let handshake_proto = HandshakeProto {
            state: Default::default(),
        };

        tx_rx(handshake_proto, true, Duration::from_millis(750), 5).await
    }
}
