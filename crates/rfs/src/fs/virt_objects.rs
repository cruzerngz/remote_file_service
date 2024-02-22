//! Virtual (remote) object definitions
#![allow(unused)]
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use futures::{AsyncRead, AsyncWrite, FutureExt};
use rfs_core::middleware::ContextManager;
use serde::{Deserialize, Serialize};

use crate::interfaces::PrimitiveFsOpsClient;

/// A file that resides over the network in the remote.
///
/// This struct aims to duplicate some of the most common file operations
/// available in [std::fs::File].
#[derive(Clone, Debug)]
pub struct VirtFile {
    ctx: ContextManager,
    path: PathBuf,

    /// The local byte buffer of the file
    local_buf: Vec<u8>,
}

/// Open a virtual file and specify some options.
///
/// Attempts to mirror [std::fs::OpenOptions].
#[derive(Clone, Debug)]
pub struct VirtOpenOptions {}

impl VirtFile {
    /// Create a new file on the remote.
    ///
    /// Attempts to mirror [std::fs::File::create]
    pub async fn create<P: AsRef<Path>>(ctx: ContextManager, path: P) -> std::io::Result<Self> {
        let res = PrimitiveFsOpsClient::create(
            &ctx,
            path.as_ref()
                .to_str()
                .and_then(|s| Some(s.to_string()))
                .unwrap_or_default(),
        )
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "File creation error"))?;

        if !res {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to create file",
            ));
        }

        Ok(Self {
            ctx,
            path: PathBuf::from(path.as_ref()),
            local_buf: Default::default(),
        })
    }

    /// Open an existing file in read-only mode.
    ///
    /// Attempts to mirror [std::fs::File::open]
    pub async fn open<P: AsRef<Path>>(ctx: ContextManager, path: P) -> std::io::Result<Self> {
        todo!()
    }

    /// Return metadata from the file
    pub async fn metadata(&self) -> std::io::Result<VirtMetadata> {
        todo!()
    }

    /// Returns the virtual file path as a string
    fn path_as_string(&self) -> String {
        self.path
            .to_str()
            .and_then(|s| Some(s.to_owned()))
            .unwrap_or_default()
    }
}

impl AsyncRead for VirtFile {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let mut read_bytes = Box::pin(PrimitiveFsOpsClient::read(&self.ctx, self.path_as_string()));

        match read_bytes.poll_unpin(cx) {
            std::task::Poll::Ready(res) => match res {
                Ok(data) => {
                    let buffer_size = buf.len();
                    let source_size = data.len();

                    let (dest_slice, source_slice) = match buffer_size.cmp(&source_size) {
                        std::cmp::Ordering::Less => (buf, &data[..buffer_size]),
                        std::cmp::Ordering::Equal => (buf, data.as_slice()),
                        std::cmp::Ordering::Greater => (&mut buf[..source_size], data.as_slice()),
                    };

                    assert_eq!(
                        dest_slice.len(),
                        source_slice.len(),
                        "slices must be of the same length"
                    );

                    dest_slice.copy_from_slice(source_slice);

                    std::task::Poll::Ready(Ok(dest_slice.len()))
                }
                Err(_e) => std::task::Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    "read failed lmao",
                ))),
            },
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

impl AsyncWrite for VirtFile {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let mut task = Box::pin(PrimitiveFsOpsClient::write_file_bytes(
            &self.ctx,
            self.path_as_string(),
            buf.to_vec(),
        ));

        match task.poll_unpin(cx) {
            std::task::Poll::Ready(res) => {
                let output = res.map_err(|e| io::Error::new(io::ErrorKind::Other, e));

                std::task::Poll::Ready(output)
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

/// Virtual directory
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtDirEntry;

/// Iterator over [VirtReadDir] entries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtReadDir;

/// Virtual file metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtMetadata;