//! This crate contains core implementations and traits for
//! both the server and client.

pub mod middleware;
pub mod ser_de;

use async_trait::async_trait;
use middleware::InvokeError;
pub use rfs_macros::*;
pub use ser_de::{
    deserialize, deserialize_packed, deserialize_packed_with_header, deserialize_with_header,
    serialize, serialize_packed, serialize_packed_with_header, serialize_with_header,
};

/// A type that is remotely invocable.
///
/// Traits with the [`remote_interface`] proc-macro automatically generate payloads
/// that fulfill these trait bounds.
pub trait RemotelyInvocable:
    RemoteMethodSignature + serde::Serialize + for<'a> serde::Deserialize<'a>
{
    /// Serializes the invocation.
    ///
    /// This method is automatically implemented and should not be overidden.
    fn invoke_bytes(&self) -> Vec<u8> {
        crate::serialize_with_header(self, Self::remote_method_signature())
            .expect("serialization should not fail")
    }

    /// Attempt to process and deserialize a set of bytes to `Self`.
    ///
    /// This method is automatically implemented and should not be overidden.
    fn process_invocation(bytes: &[u8]) -> Result<Self, InvokeError> {
        let signature = Self::remote_method_signature();

        println!("signature: {:?}", signature);
        println!("compare  : {:?}", &bytes[..signature.len()]);

        match bytes.starts_with(signature) {
            true => (),
            false => return Err(InvokeError::SignatureNotMatched),
        }

        crate::deserialize_with_header(bytes, Self::remote_method_signature())
            .map_err(|_| InvokeError::DeserializationFailed)
    }
}

// blanket implementation
impl<T> RemotelyInvocable for T where
    T: RemoteMethodSignature + serde::Serialize + for<'a> serde::Deserialize<'a>
{
}

/// This trait is used for differentiating the variant of a payload.
///
/// This trait is automatically derived from any interface that has the
/// [`remote_interface`] proc-macro. (not yet)
pub trait RemoteRequest {
    /// Checks if the payload is a request
    fn is_request(&self) -> bool;
    /// Checks if the payload is a response
    fn is_response(&self) -> bool;
}

/// This trait is used for derived payloads that call their parent
/// interfaces.
///
/// The given function signature must match with the parent interface.
///
/// This trait is automatically derived from any interface that has the
/// [`remote_interface`] proc-macro. (not yet)
#[async_trait]
trait RemoteCall {
    type Function;

    async fn call(&self, func: Self::Function) -> Self;
}

/// The signature of a method call, used for routing remote invocations
/// to their respective methods.
///
/// This trait is automatically derived from any interface that has the
/// [`remote_interface`] or [`remote_callback`] proc-macro.
pub trait RemoteMethodSignature {
    /// Returns the method signature of a remote interface method.
    ///
    /// Used for routing method calls on the server side.
    fn remote_method_signature() -> &'static [u8];
}

/// Macro testing mod
mod derive_tests {
    // use rfs_macros::remote_interface;

    // use crate::RemoteMethodSignature;
    // use crate::middleware::ContextManager;
    // #[remote_interface]
    // pub trait FileOperations {
    //     fn get_file_info(path: String, offset: Option<usize>) -> String;

    //     fn create_file(path: String) -> bool;
    // }
}

// #[cfg(test)]
// #[allow(unused)]
// mod tests {
//     use super::*;
//     use derive_tests::FileOperationsGetFileInfoMessage;
//     use rfs_macros::remote_interface;

//     #[remote_interface]
//     pub trait ASD {
//         async fn get_file_info(path: String, offset: Option<usize>) -> String;

//         async fn create_file(path: PathBuf) -> bool;
//     }

//     #[test]
//     fn test_remote_interface_expansion() {
//         let s = S::remote_method_signature();

//         let res = ASDCreateFileMessage::remote_method_signature();
//         let res = std::str::from_utf8(res).unwrap();

//         println!("{:?}", s);

//         println!("{:?}", res);
//     }
// }
