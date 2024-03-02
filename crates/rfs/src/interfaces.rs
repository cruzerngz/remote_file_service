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
    async fn read_bytes(path: String) -> Vec<u8>;

    /// Write a vector of bytes to a file. The file will be created if it does not exist.
    ///
    /// If the file exists, the contents of the file will be replaced by the payload.
    async fn write_bytes(path: String, contents: Vec<u8>) -> bool;

    /// Writes some bytes into a file path, returning the number of bytes written.
    ///
    /// If the file exists, the contents will be appended to the end.
    async fn write_append_bytes(path: String, bytes: Vec<u8>) -> usize;

    /// Create a file at a specified path. Returns the result of the operation.
    async fn create(path: String) -> bool;

    /// Remove a file at a specified path. Returns the result of the operation.
    async fn remove(path: String) -> bool;

    /// Rename a file or directory at a specified path. Returns the result of the operation.
    async fn rename(path: String, from: String, to: String) -> bool;

    /// Create a directory.
    async fn mkdir(path: String) -> bool;

    /// Remove a directory.
    async fn rmdir(path: String) -> bool;

    /// Read the contents of a directory
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

/// Methods that register a callback are defined here.
///
/// These methods should not be invoked directly!
#[remote_interface]
pub trait CallbackOps {
    async fn register_file_update(path: String) -> bool;
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
            PrimitiveFsOpsReadBytes,
            PrimitiveFsOpsWriteBytes,
            PrimitiveFsOpsCreate,
            PrimitiveFsOpsWriteAppendBytes,
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
}
