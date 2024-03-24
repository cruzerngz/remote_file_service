//! This module contains the client and server side
//! objects that transmit the contents of method invocations
//! over the network.
// #![allow(unused)]

mod blob_trx;
mod callback;
mod context_manager;
mod dispatch;
mod handshake_proto;

use futures::FutureExt;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use std::{fmt::Debug, io, net::Ipv4Addr};
use tokio::net::UdpSocket;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use context_manager::*;
pub use dispatch::*;
pub use handshake_proto::{FaultyHandshakeProto, HandshakeProto};

use crate::ser_de::byte_packer::{pack_bytes, unpack_bytes};
// define the serde method here once for use by submodules
use crate::ser_de::deserialize_packed as deserialize_primary;
use crate::ser_de::serialize_packed as serialize_primary;

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

    /// The request is a duplicate
    DuplicateRequest,
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

/// Dispatcher context, injected into each remote implementation.
#[derive(Debug, Clone)]
pub struct DispatcherContext {
    source: SocketAddrV4,
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
/// ```ignore
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
pub trait TransmissionProtocol: Debug {
    /// Send bytes to the remote. Any fault-tolerant logic should be implemented here.
    async fn send_bytes(
        &self,
        sock: &UdpSocket,
        target: SocketAddrV4,
        payload: &[u8],
        timeout: Duration,
        retries: u8,
    ) -> io::Result<usize>;
    // where
    //     A: ToSocketAddrs + std::marker::Send + std::marker::Sync;

    /// Wait for a UDP packet. Returns the packet source and data.
    async fn recv_bytes(
        &self,
        sock: &UdpSocket,
        timeout: Duration,
        retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)>;
}

/// Converts a socket address to a V4 one.
/// V6 addresses will return an error.
pub fn sockaddr_to_v4(addr: SocketAddr) -> io::Result<SocketAddrV4> {
    match addr {
        SocketAddr::V4(a) => Ok(a),
        SocketAddr::V6(_) => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "IPv6 addresses are not supported",
        )),
    }
}

/// A simple version of [HandshakeProto]. This protocol is compatible with [FaultyRequestAckProto].
///
/// Every sent item needs an ack back.
#[derive(Clone, Debug, Default)]
pub struct RequestAckProto;

#[async_trait]
impl TransmissionProtocol for RequestAckProto {
    async fn send_bytes(
        &self,
        sock: &UdpSocket,
        target: SocketAddrV4,
        payload: &[u8],
        timeout: Duration,
        mut retries: u8,
    ) -> io::Result<usize>
// where
    //     A: ToSocketAddrs + std::marker::Send + std::marker::Sync,
    {
        let mut res: io::Result<usize> = Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "connection timed out",
        ));

        while retries != 0 {
            log::debug!("sending data to target");

            // occasionally err
            let send_size = sock.send_to(payload, &target).await?;

            let mut buf = [0_u8; 100];

            tokio::select! {
                biased;

                recv_res = async {
                    sock.recv(&mut buf).await
                }.fuse() => {
                    log::debug!("response received from target");

                    let recv_size = recv_res?;
                    let slice = &buf[..recv_size];

                    let de: TransmissionPacket = deserialize_primary(slice).unwrap();
                    let hash = if let TransmissionPacket::Ack(h) = de {
                        h
                    } else {
                        res = Err(io::Error::new(io::ErrorKind::InvalidData, "expected Ack"));
                        break;
                    };

                    if hash == hash_primary(&payload) {
                        res = Ok(send_size);
                    } else {
                        res = Err(io::Error::new(io::ErrorKind::InvalidData, "Ack does not match"));
                    }

                    break;
                },
                _ = async {
                    tokio::time::sleep(timeout).await;
                }.fuse() => {
                    retries -= 1;
                    log::debug!("response timed out. retries remaining: {}", retries);

                    continue;
                }
            }
        }

        res
    }

    async fn recv_bytes(
        &self,
        sock: &UdpSocket,
        _timeout: Duration,
        _retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)> {
        let mut recv_buf = [0_u8; BYTE_BUF_SIZE];

        let (size, addr) = sock.recv_from(&mut recv_buf).await?;

        let hash = hash_primary(&&recv_buf[..size]);
        let resp = TransmissionPacket::Ack(hash);

        let ser_resp = serialize_primary(&resp).expect("serialization should not fail");
        sock.send_to(&ser_resp, addr).await?;

        Ok((sockaddr_to_v4(addr)?, recv_buf[..size].to_vec()))
    }
}

/// A faulty version that is compatible with [RequestAckProto].
///
/// This protocol may drop packets on transmission.
/// The packet drop probabilty is specified in the const generic.
///
/// The proto will fail to transmit every 1 in `FRAC` invocations on average.
#[derive(Clone, Debug)]
pub struct FaultyRequestAckProto {
    frac: u32,
}

impl FaultyRequestAckProto {
    pub fn from_frac(frac: u32) -> Self {
        Self { frac }
    }
}

#[async_trait]
impl TransmissionProtocol for FaultyRequestAckProto {
    async fn send_bytes(
        &self,
        sock: &UdpSocket,
        target: SocketAddrV4,
        payload: &[u8],
        timeout: Duration,
        mut retries: u8,
    ) -> io::Result<usize>
// where
    //     A: ToSocketAddrs + std::marker::Send + std::marker::Sync,
    {
        let mut res: io::Result<usize> = Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "connection timed out",
        ));

        while retries != 0 {
            log::debug!("sending data to target");

            // occasionally err
            let send_size = match probability_frac(self.frac) {
                true => {
                    log::error!("simulated packet drop");
                    payload.len()
                }
                false => sock.send_to(payload, &target).await?,
            };

            let mut buf = [0_u8; 100];

            tokio::select! {
                biased;

                recv_res = async {
                    sock.recv(&mut buf).await
                }.fuse() => {
                    log::debug!("response received from target");

                    let recv_size = recv_res?;
                    let slice = &buf[..recv_size];

                    let de: TransmissionPacket = deserialize_primary(slice).unwrap();
                    let hash = if let TransmissionPacket::Ack(h) = de {
                        h
                    } else {
                        res = Err(io::Error::new(io::ErrorKind::InvalidData, "expected Ack"));
                        break;
                    };

                    if hash == hash_primary(&payload) {
                        res = Ok(send_size);
                    } else {
                        res = Err(io::Error::new(io::ErrorKind::InvalidData, "Ack does not match"));
                    }

                    break;
                },
                _ = async {
                    tokio::time::sleep(timeout).await;
                }.fuse() => {
                    retries -= 1;
                    log::debug!("response timed out. retries remaining: {}", retries);

                    continue;
                }
            }
        }

        res
    }

    async fn recv_bytes(
        &self,
        sock: &UdpSocket,
        _timeout: Duration,
        _retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)> {
        let mut recv_buf = [0_u8; BYTE_BUF_SIZE];

        let (size, addr) = sock.recv_from(&mut recv_buf).await?;

        let hash = hash_primary(&&recv_buf[..size]);
        let resp = TransmissionPacket::Ack(hash);

        let ser_resp = serialize_primary(&resp).expect("serialization should not fail");
        sock.send_to(&ser_resp, addr).await?;

        Ok((sockaddr_to_v4(addr)?, recv_buf[..size].to_vec()))
    }
}

/// Returns the outcome of the probability of getting `1` in `frac`.
fn probability_frac(frac: u32) -> bool {
    let rand_num: u64 = rand::random();
    let threshold = u64::MAX / frac as u64;

    rand_num < threshold
}

/// Packets are sent to the destination without checking if they have been received.
///
/// This protocol is compatible only with itself.
///
/// As this sends all data in a single UDP packet, the max payload size is `65507` bytes.
#[derive(Clone, Debug)]
pub struct DefaultProto;

#[async_trait]
impl TransmissionProtocol for DefaultProto {
    async fn send_bytes(
        &self,
        sock: &UdpSocket,
        target: SocketAddrV4,
        payload: &[u8],
        _timeout: Duration,
        _retries: u8,
    ) -> io::Result<usize> {
        let packed = pack_bytes(payload);
        sock.send_to(&packed, target).await?;

        Ok(payload.len())
    }

    async fn recv_bytes(
        &self,
        sock: &UdpSocket,
        _timeout: Duration,
        _retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)> {
        let mut buf = [0_u8; 65535];

        let (size, addr) = sock.recv_from(&mut buf).await?;

        let addr = sockaddr_to_v4(addr)?;
        let unpacked = unpack_bytes(&buf[..size]);

        Ok((addr, unpacked))
    }
}

/// A faulty version of [DefaultProto].
#[derive(Debug)]
pub struct FaultyDefaultProto {
    frac: u32,
}

impl FaultyDefaultProto {
    pub fn from_frac(frac: u32) -> Self {
        Self { frac }
    }
}

#[async_trait]
impl TransmissionProtocol for FaultyDefaultProto {
    async fn send_bytes(
        &self,
        sock: &UdpSocket,
        target: SocketAddrV4,
        payload: &[u8],
        _timeout: Duration,
        _retries: u8,
    ) -> io::Result<usize> {
        match probability_frac(self.frac) {
            true => {
                log::error!("simulated packet drop");
                Ok(payload.len())
            }
            false => {
                let packed = pack_bytes(payload);
                sock.send_to(&packed, target).await?;

                Ok(payload.len())
            }
        }
    }

    async fn recv_bytes(
        &self,
        sock: &UdpSocket,
        _timeout: Duration,
        _retries: u8,
    ) -> io::Result<(SocketAddrV4, Vec<u8>)> {
        let mut buf = [0_u8; 65535];

        let (size, addr) = sock.recv_from(&mut buf).await?;

        let addr = sockaddr_to_v4(addr)?;
        let unpacked = unpack_bytes(&buf[..size]);

        Ok((addr, unpacked))
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
            InvokeError::HandlerNotFound => {
                io::Error::new(io::ErrorKind::NotFound, "handler not found")
            }
            InvokeError::SignatureNotMatched => {
                io::Error::new(io::ErrorKind::InvalidData, "signature not matched")
            }
            InvokeError::RequestTimedOut => {
                io::Error::new(io::ErrorKind::TimedOut, "request timed out")
            }
            InvokeError::DeserializationFailed => {
                io::Error::new(io::ErrorKind::InvalidData, "deserialization failed")
            }
            InvokeError::RemoteConnectionFailed => {
                io::Error::new(io::ErrorKind::ConnectionRefused, "remote connection failed")
            }
            InvokeError::DataTransmissionFailed => {
                io::Error::new(io::ErrorKind::BrokenPipe, "data transmission failed")
            }
            InvokeError::RemoteReceiveError => {
                io::Error::new(io::ErrorKind::BrokenPipe, "remote receive error")
            }
            InvokeError::InvalidData => io::Error::new(io::ErrorKind::InvalidData, "invalid data"),
            InvokeError::DuplicateRequest => {
                io::Error::new(io::ErrorKind::Interrupted, "duplicate request")
            }
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
    async fn tx_rx(
        proto: Arc<dyn TransmissionProtocol + Send + Sync>,
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

        log::debug!("tx_sock: {:?}", tx_sock);
        log::debug!("rx_sock: {:?}", rx_sock);

        let tx_target = rx_sock.local_addr().unwrap();
        let rx_target = tx_sock.local_addr().unwrap();

        log::debug!("tx_target: {:?}", tx_target);
        log::debug!("rx_target: {:?}", rx_target);

        let mut tx_proto = proto.clone();
        let mut rx_proto = proto.clone();

        let payload_clone = data_payload.clone();

        let rx_handle =
            tokio::spawn(async move { rx_proto.recv_bytes(&rx_sock, timeout, retries).await });

        let tx_handle = tokio::spawn(async move {
            tx_proto
                .send_bytes(
                    &tx_sock,
                    sockaddr_to_v4(tx_target)?,
                    &payload_clone,
                    timeout,
                    retries,
                )
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
        pretty_env_logger::formatted_timed_builder()
            .parse_filters("DEBUG")
            .init();

        let handshake_proto = HandshakeProto {};
        let proto_arc = Arc::new(handshake_proto);

        log::info!("testing HandshakeProto large");
        tx_rx(proto_arc.clone(), true, Duration::from_millis(750), 5).await;

        log::info!("testing HandshakeProto small");
        tx_rx(proto_arc.clone(), false, Duration::from_millis(750), 5).await;

        log::info!("testing DefaultProto small");
        tx_rx(Arc::new(DefaultProto), false, Duration::from_millis(400), 2).await;

        log::info!("testing RequestAckProto small");
        tx_rx(
            Arc::new(RequestAckProto),
            false,
            Duration::from_millis(400),
            3,
        )
        .await;

        log::info!("testing FaultyRequestAckProto small");
        tx_rx(
            Arc::new(FaultyRequestAckProto::from_frac(10)),
            false,
            Duration::from_millis(400),
            3,
        )
        .await;

        return;
    }
}
