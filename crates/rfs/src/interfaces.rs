//! Virtual method definitions.
//!
//! All traits have [`remote_interface`] attribute and only contain async functions.

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

    /// Writes some bytes into a file path, returning the number of bytes written
    async fn write_file_bytes(path: String, bytes: Vec<u8>) -> usize;

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
