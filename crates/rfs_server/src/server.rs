//! Server definition and implementations
#![allow(unused)]

// use crate::server::middleware::PayloadHandler;
use rfs::{
    fs::VirtIOErr,
    middleware::{InvokeError, PayloadHandler},
    payload_handler, RemoteMethodSignature, RemotelyInvocable,
};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use async_trait::async_trait;
use rfs::interfaces::*;

#[derive(Debug)]
pub struct RfsServer {
    /// Starting directory for the server.
    ///
    /// The server has access to all files seen within this directory.
    /// The server deos not have access to paths outside this directory.
    pub base: PathBuf,

    /// File read cache, pending transmission to clients.
    /// This cache contains the entire contents of a file.
    pub read_cache: HashMap<String, Vec<u8>>,
}

impl Default for RfsServer {
    fn default() -> Self {
        let exe_dir = std::env::current_dir().expect("failed to get executable dir");

        log::debug!(
            "server base file path: {:?}",
            std::fs::canonicalize(&exe_dir)
        );

        Self {
            base: PathBuf::from(exe_dir),
            read_cache: Default::default(),
        }
    }
}

impl RfsServer {
    pub fn from_path<P: AsRef<Path>>(p: P) -> Self {
        Self {
            base: p
                .as_ref()
                .to_path_buf()
                .canonicalize()
                .expect("path must be valid"),
            read_cache: Default::default(),
        }
    }

    /// Checks if a provided path contains prev-dir path segments `..`.
    /// Paths are not resolved at the OS-level, as they might not exist yet.
    ///
    /// This prevents out-of-dir accesses, such as when the path is `../../some_path`.
    fn contains_backdir<P: AsRef<Path>>(path: P) -> bool {
        let pb = path.as_ref().to_path_buf();

        pb.into_iter().any(|segment| segment == "..")
    }

    /// Resolve the given relative path to a full path.
    ///
    /// Paths with 'backdirs' will not be resolved, and will return `None`.
    fn resolve_path<P: AsRef<Path>>(&self, path: P) -> Option<PathBuf> {
        let mut full_path = self.base.clone();
        full_path.push(path);

        match Self::contains_backdir(&full_path) {
            true => None,
            false => Some(full_path),
        }
    }
}

#[async_trait]
impl ImmutableFileOps for RfsServer {
    async fn read_file(&mut self, path: PathBuf, offset: Option<usize>) -> Vec<u8> {
        todo!()
    }

    async fn ls(&mut self, path: PathBuf) -> Vec<String> {
        todo!()
    }
}

#[async_trait]
impl MutableFileOps for RfsServer {
    async fn create_file(&mut self, path: PathBuf, truncate: bool) -> Result<(bool, i32), ()> {
        todo!()
    }
}

#[async_trait]
impl PrimitiveFsOps for RfsServer {
    async fn read_all(&mut self, path: String) -> Vec<u8> {
        let full_path = match self.resolve_path(&path) {
            Some(p) => p,
            None => return vec![],
        };

        log::debug!("reading file path: {:?}", full_path);

        let file = match std::fs::read(full_path) {
            Ok(s) => s,
            Err(e) => {
                log::error!("read error: {}", e);

                vec![]
            }
        };

        log::debug!("file contents: {:?}", file);

        file
    }

    async fn read_bytes(&mut self, path: String, offset: usize, len: usize) -> Vec<u8> {
        let data = match self.read_cache.get(&path) {
            Some(contents) => {
                let slice = &contents[offset..(offset + len)];

                slice.to_vec()
            }
            None => {
                let mut full_path = self.base.clone();
                full_path.push(&path);

                let file_data = match fs::read(full_path) {
                    Ok(d) => d,
                    Err(_) => return vec![],
                };

                let slice = &file_data[offset..(offset + len)];
                let res = slice.to_vec();
                self.read_cache.insert(path.clone(), file_data);

                res
            }
        };

        data
    }

    async fn write_all(&mut self, path: String, contents: Vec<u8>) -> bool {
        let mut full_path = self.base.clone();
        full_path.push(path);

        match fs::write(full_path, contents) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    async fn write_bytes(
        &mut self,
        path: String,
        bytes: Vec<u8>,
        mode: FileWriteMode,
    ) -> Result<usize, VirtIOErr> {
        let full_path = match self.resolve_path(&path) {
            Some(p) => p,
            None => return Err(VirtIOErr::NotFound),
        };

        let res = match mode {
            FileWriteMode::Append => (OpenOptions::new().append(true).open(&full_path), bytes),
            FileWriteMode::Truncate => (
                OpenOptions::new()
                    .truncate(true)
                    .write(true)
                    .open(&full_path),
                bytes,
            ),
            FileWriteMode::Insert(offset) => {
                let curr_contents = fs::read(&full_path).map_err(|e| VirtIOErr::from(e))?;
                let (left, right) = curr_contents.split_at(offset);
                let mut new_contents = Vec::with_capacity(curr_contents.len() + bytes.len());
                new_contents.extend_from_slice(left);
                new_contents.extend_from_slice(&bytes);
                new_contents.extend_from_slice(right);

                (
                    OpenOptions::new()
                        .truncate(true)
                        .write(true)
                        .open(&full_path),
                    new_contents,
                )
            }
        };

        let (mut f, data) = { (res.0.map_err(|e| VirtIOErr::from(e))?, res.1) };
        f.write_all(&data).map_err(|e| VirtIOErr::from(e))?;

        Ok(data.len())
    }

    async fn create(&mut self, path: String) -> bool {
        let mut start = self.base.clone();
        start.push(path);

        log::debug!("creating file at {:?}", start);

        match std::fs::File::create(start) {
            Ok(_) => true,
            Err(e) => {
                log::error!("failed to create file: {}", e);
                false
            }
        }
    }

    async fn remove(&mut self, path: String) -> bool {
        match std::fs::remove_file(path) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    async fn rename(&mut self, path: String, from: String, to: String) -> bool {
        todo!()
    }

    async fn mkdir(&mut self, path: String) -> bool {
        todo!()
    }
    async fn rmdir(&mut self, path: String) -> bool {
        todo!()
    }
    async fn read_dir(&mut self, path: String) -> bool {
        todo!()
    }

    async fn file_size(&mut self, path: String) -> usize {
        0
    }
}

#[async_trait]
impl SimpleOps for RfsServer {
    async fn say_hello(&mut self, content: String) -> bool {
        println!("Hello, {}!", content);

        true
    }

    async fn compute_fib(&mut self, fib_num: u8) -> u64 {
        // pretend that some expensive computation is taking place
        tokio::time::sleep(Duration::from_secs(5)).await;

        match fib_num {
            0 => 0,
            1 => 1,
            other => {
                let mut sml = 0;
                let mut big = 1;

                for _ in 2..=other {
                    let next = sml + big;
                    sml = big;
                    big = next;
                }

                big
            }
        }
    }
}

// assign dispatch paths to the server.
payload_handler! {
    RfsServer,
    // ImmutableFileOpsReadFile => ImmutableFileOps::read_file_payload,
    // MutableFileOpsCreateFile => MutableFileOps::create_file_payload,

    // sanity check interface
    SimpleOpsSayHello => SimpleOps::say_hello_payload,
    SimpleOpsComputeFib => SimpleOps::compute_fib_payload,

    // primitive ops
    PrimitiveFsOpsReadAll => PrimitiveFsOps::read_all_payload,
    PrimitiveFsOpsWriteAll => PrimitiveFsOps::write_all_payload,
    PrimitiveFsOpsCreate => PrimitiveFsOps::create_payload,
    PrimitiveFsOpsRemove => PrimitiveFsOps::remove_payload,
    PrimitiveFsOpsReadBytes => PrimitiveFsOps::read_bytes_payload,
    PrimitiveFsOpsWriteBytes => PrimitiveFsOps::write_bytes_payload,
}

// #[async_trait]
// impl PayloadHandler for RfsServer {
//     async fn handle_payload(&mut self, payload_bytes: &[u8]) -> Result<Vec<u8>, InvokeError> {
//         log::debug!("incoming payload: {:?}", payload_bytes);

//         // if sig does not match, continue
//         if payload_bytes.starts_with(
//             <SimpleOpsComputeFib as rfs::RemoteMethodSignature>::remote_method_signature(),
//         ) {
//             let payload =
//                 <SimpleOpsComputeFib as rfs::RemotelyInvocable>::process_invocation(payload_bytes)?;
//             let res = self.compute_fib_payload(payload).await;
//             let resp = SimpleOpsComputeFib::Response(res);
//             let export_payload = rfs::RemotelyInvocable::invoke_bytes(&resp);
//             return Ok(export_payload);
//         }

//         Err(InvokeError::HandlerNotFound)
//     }
// }

// this is a sample of what the macro implements
// #[async_trait]
// impl DispatchHandler for RfsServer {
//     async fn dispatch(
//         &mut self,
//         payload_bytes: &[u8],
//     ) -> Result<Vec<u8>, rfs_core::middleware::InvokeError> {
//         // each method call has one block like this
//         if let Ok(payload) = MutableFileOpsCreateFile::process_invocation(payload_bytes) {
//             let res = Self::create_file_payload(payload).await;
//             let resp = MutableFileOpsCreateFile::Response(res);
//             let export_payload = resp.invoke_bytes();
//             return Ok(export_payload);
//         }

//         Err(InvokeError::HandlerNotFound)
//     }
// }

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_contains_backdir() {
        let server = RfsServer::from_path(".");
        assert!(RfsServer::contains_backdir(PathBuf::from(
            "../this/is/invalid"
        )));
        assert!(RfsServer::contains_backdir(PathBuf::from(
            "this/../../is/also/invalid"
        )));
        assert!(!RfsServer::contains_backdir(PathBuf::from(
            "./this/is/valid"
        )));
    }
}
