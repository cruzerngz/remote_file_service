//! Virtual (remote) object definitions
#![allow(unused)]
use std::{
    fmt::Debug,
    fs::FileTimes,
    io::{self, Read},
    path::{Path, PathBuf},
    time::SystemTime,
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
pub struct VirtOpenOptions {
    ctx: ContextManager,
    create: bool,
    read: bool,
    write: bool,
    truncate: bool,
    append: bool,
}

/// An item inside a directory.
///
/// This item can be a file, or a directory.
#[derive(Clone, Debug)]
pub struct VirtDirEntry;

/// Iterator over [VirtDirEntry] items.
#[derive(Clone, Debug)]
pub struct VirtReadDir {
    entries: Vec<VirtDirEntry>,
}

/// Virtual file metadata
#[derive(Clone, Debug)]
pub struct VirtMetadata {
    /// Last file access time
    accessed: SystemTime,

    /// Last file mutation time
    modified: SystemTime,

    permissions: VirtPermissions,
}

/// File permissions (rwx)
#[derive(Clone, Debug)]
pub struct VirtPermissions {
    read: (bool, bool, bool),
    write: (bool, bool, bool),
    execute: (bool, bool, bool),
}

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
        // let res = PrimitiveFsOpsClient

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
        let mut read_bytes = Box::pin(PrimitiveFsOpsClient::read_bytes(
            &self.ctx,
            self.path_as_string(),
        ));

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
        let mut task = Box::pin(PrimitiveFsOpsClient::write_append_bytes(
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

impl VirtOpenOptions {
    pub fn new(ctx: ContextManager) -> Self {
        Self {
            ctx,
            // target: todo!(),
            create: false,
            read: false,
            write: false,
            // open: false,
            truncate: false,
            append: false,
        }
    }

    pub fn read(&mut self, read: bool) -> &mut Self {
        self.read = read;

        self
    }

    pub fn write(&mut self, write: bool) -> &mut Self {
        self.write = write;

        self
    }

    pub fn append(&mut self, append: bool) -> &mut Self {
        self.append = append;

        self
    }

    pub fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.truncate = truncate;

        self
    }

    pub fn create(&mut self, create: bool) -> &mut Self {
        self.create = create;

        self
    }

    pub fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<VirtFile> {
        match (
            self.read,
            self.write,
            self.create,
            self.append,
            self.truncate,
        ) {
            // cannot create and read at the same time
            // (true, _, true, _, _) => {
            //     return Err(io::Error::new(
            //         io::ErrorKind::InvalidData,
            //         "cannot create and ",
            //     ))
            // }

            // cannot append and truncate at the same time
            (_, _, _, true, true) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "cannot append and truncate at the same time",
                ))
            }

            // cannot truncate without write
            (_, false, _, _, true) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "cannot truncate file without writing",
                ))
            }

            // passed checks
            (r, w, c, a, t) => {
                todo!()
            }

            _ => todo!(),
        }

        todo!()
    }
}

impl VirtDirEntry {
    pub fn path(&self) -> PathBuf {
        todo!()
    }

    pub fn metadata(&self) -> VirtMetadata {
        todo!()
    }
}

mod testing {
    use std::fs;

    fn test() {
        let stuff = fs::read_dir("path").unwrap();

        for item in stuff {
            let i = item.unwrap();
        }
    }
}
