//! Remote methods between server and client are defined here.

use std::path::PathBuf;

use rfs_macros::remote_interface;

use crate::RemoteMethodSignature;

/// Immutable file operations are defined in this interface.
#[remote_interface]
pub trait ImmutableFileOps {
    /// Read the contents of a file.
    async fn read_file(path: PathBuf, offset: Option<usize>) -> Vec<u8>;
}

/// Mutable file operations are defined in this interface.
#[remote_interface]
pub trait MutableFileOps {
    async fn create_file(path: PathBuf) -> Result<(), ()>;
}

#[cfg(test)]
mod tests {

    use std::{fs::File, io::Write};

    use crate::RemotelyInvocable;

    use super::*;

    /// Test the fully integrated ser/de of the payload of a remote invocation.
    #[test]
    fn test_remote_serde() {
        let message = ImmutableFileOpsReadFile::Request {
            path: Default::default(),
            offset: None,
        };

        let ser = message.invoke_bytes();

        println!("{:?}", ser);

        let des = ImmutableFileOpsReadFile::process_invocation(&ser).unwrap();

        println!("{:?}", des);

        let mut x = File::create("serialized").unwrap();
        x.write_all(&ser).unwrap();
    }
}
