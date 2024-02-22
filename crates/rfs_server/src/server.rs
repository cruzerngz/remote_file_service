//! Server definition and implementations
#![allow(unused)]

// use crate::server::middleware::PayloadHandler;
use rfs::{
    handle_payloads,
    middleware::{InvokeError, PayloadHandler},
    RemoteMethodSignature, RemotelyInvocable,
};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use async_trait::async_trait;
use rfs::interfaces::*;

#[derive(Debug)]
pub struct RfsServer {
    /// Starting directory for the server.
    pub home: PathBuf,
}

impl Default for RfsServer {
    fn default() -> Self {
        let exe_dir = std::env::current_dir().expect("failed to get executable dir");

        log::debug!(
            "server base file path: {:?}",
            std::fs::canonicalize(&exe_dir)
        );

        Self {
            home: PathBuf::from(exe_dir),
        }
    }
}

impl RfsServer {
    pub fn from_path(p: PathBuf) -> Self {
        Self { home: p }
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
    async fn read(&mut self, path: String) -> Vec<u8> {
        let file = match std::fs::read(path) {
            Ok(s) => s,
            Err(_) => vec![],
        };

        file
    }

    async fn write(&mut self, path: String) -> bool {
        todo!()
    }

    async fn write_file_bytes(&mut self, path: String, bytes: Vec<u8>) -> usize {
        let mut start = self.home.clone();
        start.push(path);

        match OpenOptions::new().append(true).open(start) {
            Ok(mut f) => match f.write_all(&bytes) {
                Ok(num) => bytes.len(),
                Err(e) => {
                    log::error!("failed to write to file: {}", e);
                    0
                }
            },
            Err(e) => {
                log::error!("file open failed: {}", e);

                0
            }
        }
    }

    async fn create(&mut self, path: String) -> bool {
        let mut start = self.home.clone();
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
handle_payloads! {
    RfsServer,
    // ImmutableFileOpsReadFile => ImmutableFileOps::read_file_payload,
    // MutableFileOpsCreateFile => MutableFileOps::create_file_payload,

    SimpleOpsSayHello => SimpleOps::say_hello_payload,
    SimpleOpsComputeFib => SimpleOps::compute_fib_payload,

    // primitive ops
    PrimitiveFsOpsRead => PrimitiveFsOps::read_payload,
    PrimitiveFsOpsWrite => PrimitiveFsOps::write_payload,
    PrimitiveFsOpsCreate => PrimitiveFsOps::create_payload,
    PrimitiveFsOpsRemove => PrimitiveFsOps::remove_payload,
    PrimitiveFsOpsWriteFileBytes => PrimitiveFsOps::write_file_bytes_payload,
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
