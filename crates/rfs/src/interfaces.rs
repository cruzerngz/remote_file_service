//! Virtual method definitions.
//!
//! All traits have [`remote_interface`] attribute and only contain async functions.

use std::net::SocketAddrV4;
use std::path::PathBuf;

use rfs_core::remote_interface;
use rfs_core::RemoteMethodSignature;
use serde::Deserialize;
use serde::Serialize;

use crate::fs::VirtDirEntry;
use crate::fs::VirtIOErr;

/// Immutable file operations are defined in this interface.
#[remote_interface]
pub trait ImmutableFileOps {
    /// Read the contents of a file.
    async fn read_file(path: PathBuf, offset: Option<usize>) -> Vec<u8>;

    /// List all files in the current directory
    async fn ls(path: PathBuf) -> Vec<String>;
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
    /// Read the entire file
    async fn read_all(path: String) -> Vec<u8>;

    /// Read a portion of the file
    async fn read_bytes(path: String, offset: usize, len: usize) -> Vec<u8>;

    /// Write a vector of bytes to a file. The file will be created if it does not exist.
    ///
    /// If the file exists, the contents of the file will be replaced by the payload.
    /// This is a convenience method and is equivalent to calling [PrimitiveFsOps::write_bytes]
    /// with [`FileWriteMode::Truncate`].
    async fn write_all(path: String, contents: Vec<u8>) -> bool;

    /// Writes some bytes into a file path, returning the number of bytes written.
    ///
    /// Use the `mode` parameter to specify the write mode.
    async fn write_bytes(path: String, bytes: FileUpdate) -> Result<usize, VirtIOErr>;

    /// Writes some bytes into a file path, returning the number of bytes written.
    ///
    /// If the file exists, the contents will be overwritten.
    // async fn write_truncate_bytes(path: String, bytes: Vec<u8>) -> usize;

    /// Create a file at a specified path.
    ///
    /// This will truncate any data if the file already exists.
    /// Returns the result of the operation.
    async fn create(path: String) -> Result<(), VirtIOErr>;

    /// Remove a file at a specified path. Returns the result of the operation.
    async fn remove(path: String) -> Result<(), VirtIOErr>;

    /// Rename a file or directory at a specified path. Returns the result of the operation.
    async fn rename(path: String, from: String, to: String) -> Result<(), VirtIOErr>;

    /// Create a directory.
    async fn mkdir(path: String) -> Result<(), VirtIOErr>;

    /// Remove a directory and all of its contents.
    async fn rmdir(path: String) -> Result<(), VirtIOErr>;

    /// Read the contents of a directory
    async fn read_dir(path: String) -> Vec<VirtDirEntry>;

    /// Returns the size of the file in bytes.
    async fn file_size(path: String) -> Result<usize, VirtIOErr>;
}

/// File write modes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FileWriteMode {
    /// Append to the end of the file
    Append,

    /// Overwrite the file
    Truncate,

    /// Insert data at a specified offset.
    Insert(usize),
}

/// File update types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FileUpdate {
    /// New data that is appended to the file.
    Append(Vec<u8>),

    /// Data that is inserted at a specified offset.
    Insert((usize, Vec<u8>)),

    /// Data that completely replaces the file
    Overwrite(Vec<u8>),
}

/// Identifier for a file registered with the remote.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileId(pub(crate) u64);

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

/// Methods that register a callback are defined here.
///
/// These methods should not be invoked directly!
#[remote_interface]
pub trait CallbackOps {
    /// Registers a path to be watched for updates.
    ///
    /// Upon a write update, a [FileUpdate] will be sent to the return address.
    async fn register_file_update(path: String, return_addr: SocketAddrV4)
        -> Result<(), VirtIOErr>;
}

/// These methods are used for testing invocation semantics (various transmission protocols).
///
/// Stuff like transmission failures, the correctness of the return value, are tested here.
#[remote_interface]
pub trait TestOps {
    /// Get the stringified name of the protocol used by the remote.
    async fn get_remote_protocol() -> String;

    /// Simulate an idempotent operation.
    ///
    /// The same result is returned regardless of the number of invocations.
    async fn test_idempotent(uuid: u64) -> u64;

    /// Simulate a non-idempotent operation. (transaction updates, etc.)
    ///
    /// The result can differ based on the number of invocations.
    /// This method returns the number of invocations made for the same `uuid`.
    async fn test_non_idempotent(uuid: u64) -> usize;

    /// Reset the state of the non-idempotent operation.
    async fn reset_non_idempotent() -> ();
}

/// Data streaming operations.
///
/// These methods should not be invoked directly!
/// NOT USED
#[remote_interface]
pub trait StreamingOps {
    /// Signal to the remote to open a blob transmitter and return the network address.
    ///
    /// The path to the file is expected to be valid.
    async fn open_blob_file_tx(path: String) -> SocketAddrV4;

    /// Signal to the remote to open a blob receiver and return the network address.
    ///
    /// The path to the file may or may not be valid.
    /// File contents can be overridden or appended by setting `overwrite` to `true` or `false`.
    async fn open_blob_file_rx(path: String, overwrite: bool) -> SocketAddrV4;
}

impl FileUpdate {
    /// Perform the file update based on the previous file contents
    pub fn update_file(self, prev: &[u8]) -> Vec<u8> {
        match self {
            FileUpdate::Append(data) => [prev, data.as_slice()].concat(),
            FileUpdate::Insert((offset, data)) => match prev.len() <= offset {
                true => {
                    let (left, right) = prev.split_at(prev.len());
                    [left, data.as_slice(), right].concat()
                }
                false => {
                    let (left, right) = prev.split_at(offset);
                    [left, data.as_slice(), right].concat()
                }
            },
            FileUpdate::Overwrite(data) => data.to_owned(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            FileUpdate::Append(data) => data.len(),
            FileUpdate::Insert((_, data)) => data.len(),
            FileUpdate::Overwrite(data) => data.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Check for signature collisions between every method defined
    /// in a particular trait.
    ///
    /// Takes advantage of the fact that a vector slices is lexicographically sorted
    macro_rules! check_signature_collision {
        ($($sig: ty),*,) => {
            let mut vec = Vec::new();

            $(
                vec.push(<$sig>::remote_method_signature());
            )*

            vec.sort();

            // let words = vec.iter().map(|bytes| std::str::from_utf8(bytes).unwrap()).collect::<Vec<_>>();
            // println!("{:#?}", words);

            for i in 0..vec.len() - 1 {
                if vec[i].starts_with(&vec[i + 1]) {
                    panic!(
                        "signature prefix collision: {} and {}",
                        std::str::from_utf8(vec[i]).unwrap(),
                        std::str::from_utf8(vec[i + 1]).unwrap()
                    );
                }
            }
        };
    }

    /// Signature test for [PrimitiveFsOps]
    #[test]
    fn test_method_signature_collision_primitive_fs_ops() {
        check_signature_collision! {
            PrimitiveFsOpsReadAll,
            PrimitiveFsOpsWriteAll,
            PrimitiveFsOpsCreate,
            PrimitiveFsOpsReadBytes,
            PrimitiveFsOpsRemove,
            PrimitiveFsOpsRename,
            PrimitiveFsOpsMkdir,
            PrimitiveFsOpsRmdir,
            PrimitiveFsOpsReadDir,
        }
    }

    /// Signature test for [SimpleOps]
    #[test]
    fn test_method_signature_collision_simple_ops() {
        check_signature_collision! {
            SimpleOpsSayHello,
            SimpleOpsComputeFib,
        }
    }

    #[test]
    fn test_method_signature_collision_callback_ops() {
        check_signature_collision! {CallbackOpsRegisterFileUpdate,}
    }

    #[test]
    fn test_method_signature_collision_streaming_ops() {
        check_signature_collision! {StreamingOpsOpenBlobFileRx, StreamingOpsOpenBlobFileTx,}
    }
}
