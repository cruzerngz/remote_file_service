//! Server definition and implementations
#![allow(unused)]

use std::path::PathBuf;

use async_trait::async_trait;
use rfs_core::{dispatcher_handler, middleware::DispatchHandler, RemotelyInvocable};
use rfs_methods::*;

#[derive(Debug)]
pub struct RfsServer {
    /// Starting directory for the server.
    pub home: PathBuf,
}

#[async_trait]
impl ImmutableFileOps for RfsServer {
    async fn read_file(&mut self, path: PathBuf, offset: Option<usize>) -> Vec<u8> {
        todo!()
    }
}

#[async_trait]
impl MutableFileOps for RfsServer {
    async fn create_file(&mut self, path: PathBuf, truncate: bool) -> Result<(bool, i32), ()> {
        todo!()
    }
}

// assign dispatch paths to the server.
dispatcher_handler! {
    RfsServer,
    ImmutableFileOpsReadFile => ImmutableFileOps::read_file_payload,
    MutableFileOpsCreateFile => MutableFileOps::create_file_payload
}

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
