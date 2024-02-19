//! Server definition and implementations
#![allow(unused)]

use std::path::PathBuf;

use async_trait::async_trait;
use rfs_methods::*;

#[derive(Debug)]
pub struct RfsServer {
    /// Starting directory for the server.
    home: PathBuf,
}

#[async_trait]
impl ImmutableFileOps for RfsServer {
    async fn read_file(path: PathBuf, offset: Option<usize>) -> Vec<u8> {
        todo!()
    }
}

#[async_trait]
impl MutableFileOps for RfsServer {
    async fn create_file(path: PathBuf, truncate: bool) -> Result<(bool, i32), ()> {
        todo!()
    }
}
