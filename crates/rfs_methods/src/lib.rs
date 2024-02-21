//! Remote methods, data structures between server and client are defined here.

pub mod fs;

use std::path::PathBuf;

use rfs_core::remote_interface;

use rfs_core::RemoteMethodSignature;

/// Immutable file operations are defined in this interface.
#[remote_interface]
pub trait ImmutableFileOps {
    /// Read the contents of a file.
    async fn read_file(path: PathBuf, offset: Option<usize>) -> Vec<u8>;

    /// List all files in the current directory
    async fn ls(path: PathBuf) -> Vec<String>;

    // this is implemented by remote-interface
    // async fn read_file_payload(payload: ImmutableFileOpsReadFile) -> Vec<u8> {
    //      Self::read_file(
    //          .. params
    //      ).await
    // }
    // type X  = bool;
}

/// Mutable file operations are defined in this interface.
#[remote_interface]
pub trait MutableFileOps {
    /// Create a new file at the new path
    async fn create_file(path: PathBuf, truncate: bool) -> Result<(bool, i32), ()>;
}

/// Remotely invoked primitives, platform agnostic.
///
/// These are not meant to be invoked directly.
#[remote_interface]
pub trait PrimitiveFsOps {
    /// Read some bytes from a file
    async fn read(path: String) -> Vec<u8>;

    async fn write(path: String) -> bool;

    /// Create a file at a specified path. Returns the result of the operation.
    async fn create(path: String) -> bool;

    async fn remove(path: String) -> bool;
    async fn rename(path: String, from: String, to: String) -> bool;

    async fn mkdir(path: String) -> bool;
    async fn rmdir(path: String) -> bool;
    async fn read_dir(path: String) -> bool;
}

/// Sanity check interface
#[remote_interface]
pub trait SimpleOps {
    /// Pass something to the remote to log.
    async fn say_hello(content: String) -> bool;

    /// Compute the Nth fibonacci number and return the result.
    ///
    /// This is supposed to simulate an expensive computation.
    async fn compute_fib(fib_num: u8) -> u64;
}

#[cfg(test)]
#[allow(unused)]
mod tests {

    use rfs_core::RemotelyInvocable;

    use super::*;

    /// Test the fully integrated ser/de of the payload of a remote invocation.
    #[test]
    fn test_remote_serde() {
        type X = ImmutableFileOpsClient;
        let x = ImmutableFileOpsClient::read_file(todo!(), todo!(), todo!());

        let message = ImmutableFileOpsReadFile::Request {
            path: Default::default(),
            offset: None,
        };

        let ser = message.invoke_bytes();

        println!("{:?}", ser);

        let des = ImmutableFileOpsReadFile::process_invocation(&ser).unwrap();

        println!("{:?}", des);

        // let mut x = std::fs::File::create("serialized").unwrap();
        // x.write_all(&ser).unwrap();
    }
}
