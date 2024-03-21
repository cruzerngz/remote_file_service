//! Server definition and implementations
#![allow(unused)]

mod callbacks;

use futures::{channel::mpsc, SinkExt, StreamExt};
// use crate::server::middleware::PayloadHandler;
use rfs::{
    fs::{VirtDirEntry, VirtIOErr},
    middleware::{InvokeError, MiddlewareData, PayloadHandler},
    payload_handler, RemoteMethodSignature, RemotelyInvocable,
};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    net::SocketAddrV4,
    num::NonZeroU8,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
pub use callbacks::*;
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

    /// Registered callbacks watching for file updates.
    ///
    /// A path can have multiple callbacks.
    pub file_upd_callbacks: HashMap<String, Vec<FileUpdateCallback>>,
}

#[derive(Debug)]
pub struct FileUpdateCallback {
    addr: SocketAddrV4,
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
            file_upd_callbacks: Default::default(),
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
            file_upd_callbacks: Default::default(),
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
    /// Paths with 'backdirs' `./../` will not be resolved, and will return `None`.
    fn resolve_path<P: AsRef<Path>>(&self, path: P) -> Option<PathBuf> {
        let mut full_path = self.base.clone();
        full_path.push(path);

        match Self::contains_backdir(&full_path) {
            true => None,
            false => Some(full_path),
        }
    }

    /// Resolve the given relative path. The path must exist for method to function.
    ///
    /// All links are resolved. If the path contains backdirs, this will return `None`.
    fn resolve_relative_path<P: AsRef<Path>>(&self, path: P) -> Option<PathBuf> {
        let resolved = self.resolve_path(path)?;

        let full = resolved.canonicalize().ok()?;

        let relative = full.strip_prefix(&self.base).ok()?;

        Some(relative.to_path_buf())
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
        full_path.push(&path);

        let relative_path = full_path
            .strip_prefix(&self.base)
            .unwrap()
            .to_str()
            .unwrap();

        let mut lock = FILE_UPDATE_CALLBACKS
            .get()
            .expect("must be initialized")
            .lock()
            .await;

        let num_triggered = lock
            .trigger_file_update(&relative_path, FileUpdate::Overwrite(contents.clone()))
            .await;

        if let Some(num) = num_triggered {
            log::info!("triggered callbacks: {:?} ", num);
        }

        match fs::write(full_path, contents) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    async fn write_bytes(&mut self, path: String, data: FileUpdate) -> Result<usize, VirtIOErr> {
        let full_path = match self.resolve_path(&path) {
            Some(p) => p,
            None => return Err(VirtIOErr::NotFound),
        };

        // for simplicity, every write triggers a complete file update
        // impl details are in [FileUpdate]
        let existing_contents = fs::read(&full_path).map_err(|e| VirtIOErr::from(e))?;
        let overwritten_contents = data.to_owned().update_file(&existing_contents);

        fs::write(&full_path, overwritten_contents).map_err(|e| VirtIOErr::from(e))?;

        let relative_path = full_path
            .strip_prefix(&self.base)
            .unwrap()
            .to_str()
            .unwrap();

        let mut lock = FILE_UPDATE_CALLBACKS
            .get()
            .expect("must be initialized")
            .lock()
            .await;

        let size = data.len();
        let num_triggered = lock.trigger_file_update(&path, data).await;

        if let Some(num) = num_triggered {
            log::info!("triggered callbacks: {:?} ", num);
        }

        Ok(size)
    }

    async fn create(&mut self, path: String) -> Result<(), VirtIOErr> {
        let full_path = match self.resolve_path(&path) {
            Some(p) => p,
            None => return Err(VirtIOErr::NotFound),
        };

        log::debug!("creating file at {:?}", full_path);

        match std::fs::File::create(full_path) {
            Ok(_) => Ok(()),
            Err(e) => {
                log::error!("failed to create file: {}", e);
                Err(e.into())
            }
        }
    }

    async fn remove(&mut self, path: String) -> Result<(), VirtIOErr> {
        let full_path = match self.resolve_path(&path) {
            Some(p) => p,
            None => return Err(VirtIOErr::NotFound),
        };

        match std::fs::remove_file(full_path) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn rename(&mut self, path: String, from: String, to: String) -> Result<(), VirtIOErr> {
        todo!()
    }

    async fn mkdir(&mut self, path: String) -> Result<(), VirtIOErr> {
        todo!()
    }
    async fn rmdir(&mut self, path: String) -> Result<(), VirtIOErr> {
        todo!()
    }

    async fn read_dir(&mut self, path: String) -> Vec<VirtDirEntry> {
        let full_path = match self.resolve_path(&path) {
            Some(p) => p,
            None => return vec![],
        };

        let entries = match fs::read_dir(full_path) {
            Ok(e) => e,
            Err(_) => return vec![],
        };

        let virt = entries
            .into_iter()
            .filter_map(|entry| Some(entry.ok()?))
            .filter_map(|entry| VirtDirEntry::from_dir_entry(entry, self.base.clone()))
            .collect();

        virt
    }

    async fn file_size(&mut self, path: String) -> Result<usize, VirtIOErr> {
        todo!();
        Ok(0)
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

#[async_trait]
impl CallbackOps for RfsServer {
    async fn register_file_update(
        &mut self,
        path: String,
        return_addr: SocketAddrV4,
    ) -> Result<(), VirtIOErr> {
        let (send, mut recv) = mpsc::channel::<Arc<FileUpdate>>(1);

        let handle = FileUpdateCallback { addr: return_addr };

        // get the relative path to the server's base
        let relative_path = self
            .resolve_relative_path(&path)
            .ok_or(VirtIOErr::NotFound)?
            .to_str()
            .expect("path must be valid")
            .to_string();

        let mut lock = FILE_UPDATE_CALLBACKS
            .get()
            .expect("should be initialized")
            .lock()
            .await;

        log::debug!("registering callback for {}", relative_path);

        // create a receiver and push the channel to the callback list
        match lock.lookup.get_mut(&relative_path) {
            Some(callbacks) => callbacks.push(handle),
            None => {
                lock.lookup.insert(relative_path.clone(), vec![handle]);
            }
        };

        Ok(())
    }
}

// assign dispatch paths to the server.
payload_handler! {
    RfsServer,
    // sanity check interface
    SimpleOpsSayHello => SimpleOps::say_hello_payload,
    SimpleOpsComputeFib => SimpleOps::compute_fib_payload,

    // primitive ops
    PrimitiveFsOpsReadAll => PrimitiveFsOps::read_all_payload,
    PrimitiveFsOpsWriteAll => PrimitiveFsOps::write_all_payload,
    PrimitiveFsOpsCreate => PrimitiveFsOps::create_payload,
    PrimitiveFsOpsRename => PrimitiveFsOps::rename_payload,
    PrimitiveFsOpsRemove => PrimitiveFsOps::remove_payload,
    PrimitiveFsOpsReadBytes => PrimitiveFsOps::read_bytes_payload,
    PrimitiveFsOpsWriteBytes => PrimitiveFsOps::write_bytes_payload,

    // primitive ops (continued)
    PrimitiveFsOpsMkdir => PrimitiveFsOps::mkdir_payload,
    PrimitiveFsOpsRmdir => PrimitiveFsOps::rmdir_payload,
    PrimitiveFsOpsReadDir => PrimitiveFsOps::read_dir_payload,

    // callbacks
    CallbackOpsRegisterFileUpdate => CallbackOps::register_file_update_payload,
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
