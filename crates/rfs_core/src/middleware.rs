//! This module contains the client and server side
//! objects that transmit the contents of method invocations
//! over the network.
//!

mod blob_trx;
mod callback;
mod context_manager;
mod dispatch;

use futures::FutureExt;
use std::collections::HashMap;
use std::hash::{BuildHasher, DefaultHasher, Hash, Hasher, RandomState};
use std::net::SocketAddrV4;
use std::sync::Arc;
use std::time::Duration;
use std::{clone, io};
use std::{fmt::Debug, net::Ipv4Addr};
use tokio::net::{ToSocketAddrs, UdpSocket};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use context_manager::*;
pub use dispatch::*;

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
pub trait SocketProvider {
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

/// Types that implement this trait can be plugged into [`ContextManager`] and [`Dispatcher`].
///
/// All methods are associated methods; no `self` is required.
#[async_trait]
pub trait TransmissionProtocol: core::marker::Send + core::marker::Sync {
    /// Send bytes to the remote. Any fault-tolerant logic should be implemented here.
    async fn send_bytes<A>(
        sock: &UdpSocket,
        target: A,
        payload: &[u8],
        timeout: Duration,
        retries: u8,
    ) -> io::Result<usize>
    where
        A: ToSocketAddrs + std::marker::Send + std::marker::Sync;

    /// Send an acknowledgement to the sender.
    ///
    /// If this method is not overridden, it is a no-op.
    #[allow(unused_variables)]
    async fn send_ack<A>(sock: &UdpSocket, target: A, payload: &[u8]) -> io::Result<()>
    where
        A: ToSocketAddrs + std::marker::Send + std::marker::Sync,
    {
        Ok(())
    }

    /// Send bytes to the remote and waits for a response.
    ///
    /// The default implmentation uses `send_bytes` and `send_ack` naively.
    ///
    /// If more advanced logic is required, this method should be overridden.
    async fn send_with_response<A>(
        sock: &UdpSocket,
        target: A,
        payload: &[u8],
        timeout: Duration,
        retries: u8,
    ) -> io::Result<Vec<u8>>
    where
        A: ToSocketAddrs + Clone + std::marker::Send + std::marker::Sync,
    {
        let mut buf = [0_u8; BYTE_BUF_SIZE];

        sock.connect(&target).await?;

        let _ = Self::send_bytes(sock, &target, payload, timeout, retries).await?;
        let num_bytes = sock.recv(&mut buf).await?;
        let slice = &buf[..num_bytes];

        Self::send_ack(sock, &target, slice).await?;

        Ok(slice.to_vec())
    }
}

/// This protocol ensures that every sent packet from the source must be acknowledged by the sink.
/// Timeouts and retries are fully implmented.
#[derive(Clone, Debug)]
pub struct RequestAckProto;

#[async_trait]
impl TransmissionProtocol for RequestAckProto {
    async fn send_bytes<A>(
        sock: &UdpSocket,
        target: A,
        payload: &[u8],
        timeout: Duration,
        mut retries: u8,
    ) -> io::Result<usize>
    where
        A: ToSocketAddrs + std::marker::Send + std::marker::Sync,
    {
        let mut res: io::Result<usize> = Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "connection timed out",
        ));

        while retries != 0 {
            log::debug!("sending data to target");
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

                    let de: MiddlewareData = deserialize_primary(slice).unwrap();
                    let hash = if let MiddlewareData::Ack(h) = de {
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

    async fn send_ack<A>(sock: &UdpSocket, target: A, payload: &[u8]) -> io::Result<()>
    where
        A: ToSocketAddrs + std::marker::Send + std::marker::Sync,
    {
        let ack = MiddlewareData::Ack(hash_primary(&payload));
        let ack_bytes = serialize_primary(&ack).expect("serialization should not fail");

        sock.send_to(&ack_bytes, target).await?;

        Ok(())
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
        sock: &UdpSocket,
        target: A,
        payload: &[u8],
        timeout: Duration,
        retries: u8,
    ) -> io::Result<usize>
    where
        A: ToSocketAddrs + std::marker::Send + std::marker::Sync,
    {
        // drop packets every now and then
        match probability_frac(FRAC) {
            true => {
                log::error!("faulty packet transmission");
                Ok(payload.len())
            }
            false => RequestAckProto::send_bytes(sock, target, payload, timeout, retries).await,
        }
    }

    async fn send_ack<A>(sock: &UdpSocket, target: A, payload: &[u8]) -> io::Result<()>
    where
        A: ToSocketAddrs + std::marker::Send + std::marker::Sync,
    {
        // drop packets every now and then
        match probability_frac(FRAC) {
            true => {
                log::error!("faulty packet transmission");
                Ok(())
            }
            false => RequestAckProto::send_ack(sock, target, payload).await,
        }
    }
}

/// Returns the outcome of the probability of getting `1` in `frac`.
fn probability_frac(frac: u32) -> bool {
    let rand_num: u64 = rand::random();
    let threshold = u64::MAX / frac as u64;

    rand_num < threshold
}

/// UDP-like protocol, packets are sent to the destination without checking if they have been received.
#[derive(Clone, Debug)]
pub struct SimpleProto;

#[async_trait]
impl TransmissionProtocol for SimpleProto {
    async fn send_bytes<A>(
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
}

/// The primary hash method used for verifying the integrity of data
fn hash_primary<T: Hash>(item: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    item.hash(&mut hasher);

    hasher.finish()
}

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
}
