//! This module contains the client and server side
//! objects that transmit the contents of method invocations
//! over the network.
//!
#![allow(unused)]

mod context_manager;

use std::{
    fmt::Debug,
    net::{SocketAddrV4, UdpSocket},
};
// use tokio::net::UdpSocket;

use async_trait::async_trait;

use crate::RemotelyInvocable;
pub use context_manager::ContextManager;

/// Method invocation errors
#[derive(Debug)]
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
}

/// The dispatcher for remote invocations.
///
/// The dispatcher routes the contents of remote invocations to their
/// appropriate handlers.
///
#[derive(Debug)]
pub struct Dispatcher<H: Debug + PayloadHandler> {
    socket: UdpSocket,

    // Inner data structure that implements logic for remote interfaces
    handler: H,
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

impl<H> Dispatcher<H>
where
    H: Debug + PayloadHandler,
{
    /// Create a new dispatcher from the handler and a listening IP
    pub fn new(addr: SocketAddrV4, handler: H) -> Self {
        let socket = UdpSocket::bind(addr).expect("failed to bind to specified address");

        Self { socket, handler }
    }

    /// Runs the dispatcher indefinitely.
    pub async fn dispatch(&mut self) {
        let mut buf = [0; 10_000];

        loop {
            // buf.clear();

            match self.socket.recv_from(&mut buf) {
                Ok((bytes, addr)) => {
                    log::debug!("received {} bytes from {}", bytes, addr);

                    // connection packets have zero length
                    if bytes == 0 {
                        continue;
                    }

                    log::debug!("packet has stuff");

                    let copy = &buf[..bytes];

                    // to be spawned as a separate task
                    let bytes = match self.handler.handle_payload(copy).await {
                        Ok(res) => {
                            log::debug!("payload header: {:?}", &res[..20]);

                            self.socket.send_to(&res, addr).unwrap()
                        }
                        Err(e) => {
                            log::error!("Invoke error: {:?}", e);
                            continue;
                        }
                    };

                    log::debug!("sent {} bytes to {}", bytes, addr);
                }
                // log the error
                Err(e) => log::error!("Receive error: {}", e),
            }
        }
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
/// ```no_run
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
/// handle_payloads! {
///     Server,
///     // we use the '`method_name`_payload' method.
///      ImmutableFileOpsReadFile => ImmutableFileOps::read_file_payload
///     // an arbitrary number of paths can be added
/// }
/// ```
#[macro_export]
macro_rules! handle_payloads {
    ($server_ty: ty,
        $($payload_ty: ty => $trait: ident :: $method: ident),+
    ) => {
        #[async_trait::async_trait]
        impl PayloadHandler for $server_ty {
            async fn handle_payload(&mut self, payload_bytes: &[u8]) -> Result<Vec<u8>, rfs::middleware::InvokeError> {

                $(if let Ok(payload) = <$payload_ty as rfs::RemotelyInvocable>::process_invocation(payload_bytes) {

                    log::debug!("processing {}", stringify!($payload_ty));

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
mod tests {

    use super::*;

    macro_rules! ident_manip {
        ($first: ident, $second: ident) => {};
    }

    #[derive(Debug)]

    struct S;
    #[test]
    fn test_func_macro_stuff() {
        // dispatcher_handler! {
        //     S,
        //     ImmutableFileOpsReadFile => ImmutableFileOps::read_file,
        //     MutableFileOpsCreateFile => MutableFileOps::create_file
        // }
    }
}
