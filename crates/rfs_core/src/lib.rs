//! This crate contains core implementations and traits for
//! both the server and client.

pub mod middleware;
pub mod ser_de;

pub use rfs_macros::*;
pub use ser_de::{deserialize, deserialize_packed, serialize, serialize_packed};

/// A type that satistifies these bounds will support
/// remote invocation.
pub trait RemotelyInvocable:
    RemoteMethodSignature + serde::Serialize + for<'a> serde::Deserialize<'a>
{
    /// Serializes the invocation.
    ///
    /// This method is automatically implemented and should not be overidden.
    fn invoke_bytes(&self) -> Vec<u8> {
        let serialized = crate::serialize_packed(&self).unwrap();

        let header = Self::remote_method_signature();

        [header, &serialized].concat()
    }

    /// Attempt to process and deserialize a set of bytes to `Self`.
    ///
    /// This method is automatically implemented and should not be overidden.
    fn process_invocation(bytes: &[u8]) -> Result<Self, ()> {
        let signature = Self::remote_method_signature();

        match bytes.starts_with(signature) {
            true => (),
            false => return Err(()),
        }

        let data = &bytes[(signature.len())..];

        crate::deserialize_packed(data).map_err(|_| ())
    }
}

// blanket implementation
impl<T> RemotelyInvocable for T where
    T: RemoteMethodSignature + serde::Serialize + for<'a> serde::Deserialize<'a>
{
}

/// This trait is automatically derived with the data structure generated from
/// the `#[remote_interface]` proc-macro.
pub trait RemoteRequest {
    /// Checks if the payload is a request
    fn is_request(&self) -> bool;
    /// Checks if the payload is a response
    fn is_response(&self) -> bool;
}

/// This trait is automatically derived from any interface that has the
/// `#[remote_interface]` proc-macro.
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
