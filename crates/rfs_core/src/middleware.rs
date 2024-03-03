//! This module contains the client and server side
//! objects that transmit the contents of method invocations
//! over the network.
//!

mod callback;
mod context_manager;
mod dispatch;

use futures::FutureExt;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::time::Duration;
use std::{fmt::Debug, net::Ipv4Addr};
use tokio::net::{ToSocketAddrs, UdpSocket};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use context_manager::ContextManager;

pub use dispatch::*;

// define the serde method here once for use by submodules
use crate::ser_de::deserialize_packed as deserialize_primary;
use crate::ser_de::serialize_packed as serialize_primary;

/// Error header send between the dispatcher and context manager
const ERROR_HEADER: &[u8] = "ERROR_ERROR_ERROR_ERROR_HEADER".as_bytes();
/// Header used when communicating directly between the context manager
/// and the dispatcher.
const MIDDLWARE_HEADER: &[u8] = "MIDDLEWARE_HEADER".as_bytes();

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
    /// The value sent
    Ack(u64),
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

/// This trait is used by the dispatcher to determine the address that the server has bound itself to.
///
/// It
pub trait Addressable {
    /// Returns the address the server is bound to. The port number cannot be returned.
    fn bind_address(&self) -> Ipv4Addr;
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

/// Types that implement this trait can be plugged into [`Dispatcher`].
///
/// All methods are associated methods; no `self` is required.
#[async_trait]
pub trait TransmissionProtocol {
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
}
