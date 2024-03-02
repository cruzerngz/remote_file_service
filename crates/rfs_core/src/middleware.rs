//! This module contains the client and server side
//! objects that transmit the contents of method invocations
//! over the network.
//!

mod context_manager;
mod dispatch;

use std::{fmt::Debug, net::Ipv4Addr};
use tokio::net::UdpSocket;

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
#[async_trait]
pub trait RequestServer: PayloadHandler {
    async fn serve(&mut self, addr: std::net::SocketAddrV4) {
        todo!()
    }
}

impl<T> RequestServer for T where T: PayloadHandler {}

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

                        log::info!("method: {}", std::str::from_utf8(<$payload_ty as rfs::RemoteMethodSignature>::remote_method_signature()).unwrap());

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

#[cfg(test)]
#[allow(unused)]
mod tests {

    use super::*;

    const MIDDLEWARE_PACKET_DATA: &[u8] = &[
        101, 115, 0, 0, 0, 0, 0, 0, 0, 7, 80, 97, 121, 108, 111, 97, 100, 118, 91, 110, 0, 0, 0, 0,
        0, 0, 0, 83, 110, 0, 0, 0, 0, 0, 0, 0, 105, 110, 0, 0, 0, 0, 0, 0, 0, 109, 110, 0, 0, 0, 0,
        0, 0, 0, 112, 110, 0, 0, 0, 0, 0, 0, 0, 108, 110, 0, 0, 0, 0, 0, 0, 0, 101, 110, 0, 0, 0,
        0, 0, 0, 0, 79, 110, 0, 0, 0, 0, 0, 0, 0, 112, 110, 0, 0, 0, 0, 0, 0, 0, 115, 110, 0, 0, 0,
        0, 0, 0, 0, 58, 110, 0, 0, 0, 0, 0, 0, 0, 58, 110, 0, 0, 0, 0, 0, 0, 0, 115, 110, 0, 0, 0,
        0, 0, 0, 0, 97, 110, 0, 0, 0, 0, 0, 0, 0, 121, 110, 0, 0, 0, 0, 0, 0, 0, 95, 110, 0, 0, 0,
        0, 0, 0, 0, 104, 110, 0, 0, 0, 0, 0, 0, 0, 101, 110, 0, 0, 0, 0, 0, 0, 0, 108, 110, 0, 0,
        0, 0, 0, 0, 0, 108, 110, 0, 0, 0, 0, 0, 0, 0, 111, 110, 0, 0, 0, 0, 0, 0, 0, 101, 110, 0,
        0, 0, 0, 0, 0, 0, 115, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0,
        0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0,
        0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 7, 110, 0, 0, 0, 0,
        0, 0, 0, 82, 110, 0, 0, 0, 0, 0, 0, 0, 101, 110, 0, 0, 0, 0, 0, 0, 0, 113, 110, 0, 0, 0, 0,
        0, 0, 0, 117, 110, 0, 0, 0, 0, 0, 0, 0, 101, 110, 0, 0, 0, 0, 0, 0, 0, 115, 110, 0, 0, 0,
        0, 0, 0, 0, 116, 110, 0, 0, 0, 0, 0, 0, 0, 109, 110, 0, 0, 0, 0, 0, 0, 0, 123, 110, 0, 0,
        0, 0, 0, 0, 0, 60, 110, 0, 0, 0, 0, 0, 0, 0, 115, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0,
        0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0,
        0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0,
        0, 0, 0, 7, 110, 0, 0, 0, 0, 0, 0, 0, 99, 110, 0, 0, 0, 0, 0, 0, 0, 111, 110, 0, 0, 0, 0,
        0, 0, 0, 110, 110, 0, 0, 0, 0, 0, 0, 0, 116, 110, 0, 0, 0, 0, 0, 0, 0, 101, 110, 0, 0, 0,
        0, 0, 0, 0, 110, 110, 0, 0, 0, 0, 0, 0, 0, 116, 110, 0, 0, 0, 0, 0, 0, 0, 45, 110, 0, 0, 0,
        0, 0, 0, 0, 115, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0,
        0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0,
        0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 18, 110, 0, 0, 0, 0, 0, 0,
        0, 110, 110, 0, 0, 0, 0, 0, 0, 0, 101, 110, 0, 0, 0, 0, 0, 0, 0, 119, 110, 0, 0, 0, 0, 0,
        0, 0, 32, 110, 0, 0, 0, 0, 0, 0, 0, 99, 110, 0, 0, 0, 0, 0, 0, 0, 111, 110, 0, 0, 0, 0, 0,
        0, 0, 110, 110, 0, 0, 0, 0, 0, 0, 0, 102, 110, 0, 0, 0, 0, 0, 0, 0, 105, 110, 0, 0, 0, 0,
        0, 0, 0, 103, 110, 0, 0, 0, 0, 0, 0, 0, 117, 110, 0, 0, 0, 0, 0, 0, 0, 114, 110, 0, 0, 0,
        0, 0, 0, 0, 116, 110, 0, 0, 0, 0, 0, 0, 0, 97, 110, 0, 0, 0, 0, 0, 0, 0, 116, 110, 0, 0, 0,
        0, 0, 0, 0, 105, 110, 0, 0, 0, 0, 0, 0, 0, 111, 110, 0, 0, 0, 0, 0, 0, 0, 110, 110, 0, 0,
        0, 0, 0, 0, 0, 62, 110, 0, 0, 0, 0, 0, 0, 0, 125, 93,
    ];

    const MIDDLEWARE_PACKET_DATA_PACKED: &[u8] = &[
        101, 115, 58, 7, 58, 7, 80, 97, 121, 108, 111, 97, 100, 118, 91, 110, 58, 7, 58, 83, 110,
        58, 7, 58, 105, 110, 58, 7, 58, 109, 110, 58, 7, 58, 112, 110, 58, 7, 58, 108, 110, 58, 7,
        58, 101, 110, 58, 7, 58, 79, 110, 58, 7, 58, 112, 110, 58, 7, 58, 115, 110, 58, 7, 58, 58,
        110, 58, 7, 58, 58, 110, 58, 7, 58, 115, 110, 58, 7, 58, 97, 110, 58, 7, 58, 121, 110, 58,
        7, 58, 95, 110, 58, 7, 58, 104, 110, 58, 7, 58, 101, 110, 58, 7, 58, 108, 110, 58, 7, 58,
        108, 110, 58, 7, 58, 111, 110, 58, 7, 58, 101, 110, 58, 7, 58, 115, 110, 58, 8, 58, 110,
        58, 8, 58, 110, 58, 8, 58, 110, 58, 8, 58, 110, 58, 8, 58, 110, 58, 8, 58, 110, 58, 8, 58,
        110, 58, 7, 58, 7, 110, 58, 7, 58, 82, 110, 58, 7, 58, 101, 110, 58, 7, 58, 113, 110, 58,
        7, 58, 117, 110, 58, 7, 58, 101, 110, 58, 7, 58, 115, 110, 58, 7, 58, 116, 110, 58, 7, 58,
        109, 110, 58, 7, 58, 123, 110, 58, 7, 58, 60, 110, 58, 7, 58, 115, 110, 58, 8, 58, 110, 58,
        8, 58, 110, 58, 8, 58, 110, 58, 8, 58, 110, 58, 8, 58, 110, 58, 8, 58, 110, 58, 8, 58, 110,
        58, 7, 58, 7, 110, 58, 7, 58, 99, 110, 58, 7, 58, 111, 110, 58, 7, 58, 110, 110, 58, 7, 58,
        116, 110, 58, 7, 58, 101, 110, 58, 7, 58, 110, 110, 58, 7, 58, 116, 110, 58, 7, 58, 45,
        110, 58, 7, 58, 115, 110, 58, 8, 58, 110, 58, 8, 58, 110, 58, 8, 58, 110, 58, 8, 58, 110,
        58, 8, 58, 110, 58, 8, 58, 110, 58, 8, 58, 110, 58, 7, 58, 18, 110, 58, 7, 58, 110, 110,
        58, 7, 58, 101, 110, 58, 7, 58, 119, 110, 58, 7, 58, 32, 110, 58, 7, 58, 99, 110, 58, 7,
        58, 111, 110, 58, 7, 58, 110, 110, 58, 7, 58, 102, 110, 58, 7, 58, 105, 110, 58, 7, 58,
        103, 110, 58, 7, 58, 117, 110, 58, 7, 58, 114, 110, 58, 7, 58, 116, 110, 58, 7, 58, 97,
        110, 58, 7, 58, 116, 110, 58, 7, 58, 105, 110, 58, 7, 58, 111, 110, 58, 7, 58, 110, 110,
        58, 7, 58, 62, 110, 58, 7, 58, 125, 93,
    ];
}
