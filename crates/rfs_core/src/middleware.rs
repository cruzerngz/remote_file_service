//! This module contains the client and server side
//! objects that transmit the contents of method invocations
//! over the network.
//!
#![allow(unused)]

use std::fmt::Debug;

use crate::RemotelyInvocable;

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
}

/// The context manager for the client.
///
/// The context manager handles the transmission of data to its server-side counterpart,
/// the dispatcher.
///
/// Integrity checks, validation, etc. are performed here.
#[derive(Debug)]
pub struct ContextManager {
    /// The target address and port
    target: std::net::SocketAddrV4,
}

/// The dispatcher for remote invocations.
///
/// The dispatcher routes the contents of remote invocations to their
/// appropriate handlers.
///
#[derive(Debug)]
pub struct Dispatcher<H: Debug> {
    // Inner data structure that implements logic for remote interfaces
    handler: H,
}

/// Route and handle the bytes of a remote method invocation.
///
/// The method proceseses the bytes of a remote method invocation,
/// routes the bytes to the appropriate method call, and returns the
/// result.
pub trait DispatchHandler {
    fn dispatch(&mut self, payload_bytes: &[u8]) -> Result<Vec<u8>, InvokeError>;
}

/// This macro implements [`DispatchHandler`] with a specified number of routes.
///
/// ```no_run
/// /// Server definition (and any fields)
/// #[derive(Debug)]
/// pub struct Server;
///
/// // the remote interface implementation
/// impl ImmutableFileOps for Server {
///     /// Read the contents of a file.
///     async fn read_file(path: PathBuf, offset: Option<usize>) -> Vec<u8> {
///         // ... implementation
///         todo!()
///     }
/// }
///
///
/// dispatcher_handler!{
///     Server,
///      ImmutableFileOpsReadFile => ImmutableFileOps::read_file // this is one path
/// }
/// ```
#[macro_export]
macro_rules! dispatcher_handler {
    ($server_ty: ty,
        $($payload: ty => $trait: ident ::$method: ident),+
    ) => {
        impl DispatchHandler for $server_ty {
            fn dispatch(&mut self, payload_bytes: &[u8]) -> Result<Vec<u8>, InvokeError> {
                todo!()
            }
        }
    };
}

impl ContextManager {
    /// Create a new context manager, along with a target IP and port.
    pub async fn new(target: std::net::SocketAddrV4) -> Self {
        Self { target }
    }

    /// Send an invocation over the network, and returns the result.
    pub async fn invoke<P: RemotelyInvocable>(&self, payload: P) -> Result<P, InvokeError> {
        let data = payload.invoke_bytes();

        // send to server and wait for a reply

        todo!()
    }
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
