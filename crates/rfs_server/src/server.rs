//! Server definition and implementations
#![allow(unused)]

use std::{path::PathBuf, time::Duration};

use async_trait::async_trait;
use rfs_core::{handle_payloads, middleware::PayloadHandler, RemotelyInvocable};
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
    ImmutableFileOpsReadFile => ImmutableFileOps::read_file_payload,
    MutableFileOpsCreateFile => MutableFileOps::create_file_payload,

    SimpleOpsSayHello => SimpleOps::say_hello_payload,
    SimpleOpsComputeFib => SimpleOps::compute_fib_payload
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
