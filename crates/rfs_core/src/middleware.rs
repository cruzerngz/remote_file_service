//! This module contains the client and server side
//! objects that transmit the contents of method invocations
//! over the network.
//!
#![allow(unused)]

mod context_manager;

use std::fmt::Debug;

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
    // Inner data structure that implements logic for remote interfaces
    handler: H,
}

impl<H> Dispatcher<H>
where
    H: Debug + PayloadHandler,
{
    /// Create a new dispatcher from the handler
    pub fn from_handler(handler: H) -> Self {
        Self { handler }
    }

    /// Runs the dispatcher indefinitely.
    pub async fn dispatch(&mut self) {
        loop {}

        todo!()
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
            async fn handle_payload(&mut self, payload_bytes: &[u8]) -> Result<Vec<u8>, rfs_core::middleware::InvokeError> {

                $(if let Ok(payload) = <$payload_ty>::process_invocation(payload_bytes) {
                    let res = self.$method(payload).await;
                    let resp = <$payload_ty>::Response(res);
                    let export_payload = resp.invoke_bytes();
                    return Ok(export_payload);
                })+

                // no matches, error out
                Err(rfs_core::middleware::InvokeError::HandlerNotFound)
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
