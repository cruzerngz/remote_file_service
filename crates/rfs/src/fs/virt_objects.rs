//! Virtual (remote) object and related definitions

#![allow(unused)]
use std::{
    fmt::Debug,
    fs::FileTimes,
    io::{self, Read},
    path::{Path, PathBuf},
    time::SystemTime,
};

use futures::{AsyncRead, AsyncWrite, FutureExt};
use rfs_core::middleware::{ContextManager, TransmissionProtocol};
use serde::{Deserialize, Serialize};

use crate::interfaces::PrimitiveFsOpsClient;

/// Errors for virtual IO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VirtIOErr {
    /// The
    InvalidPath,
}

/// A file that resides over the network in the remote.
///
/// This struct aims to duplicate some of the most common file operations
/// available in [std::fs::File].
///
/// For simplicity, symlinks residing on the remote will not be treated as files
/// and they will be ignored.
#[derive(Clone, Debug)]
pub struct VirtFile<T: TransmissionProtocol> {
    ctx: ContextManager<T>,
    path: PathBuf,

    /// The local byte buffer of the file
    local_buf: Vec<u8>,
}

/// Open a virtual file and specify some options.
///
/// Attempts to mirror [std::fs::OpenOptions].
#[derive(Clone, Debug)]
pub struct VirtOpenOptions<T: TransmissionProtocol> {
    ctx: ContextManager<T>,
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
pub struct VirtDirEntry {
    /// Converted from the `path()` on the remote,
    /// because PathBuf does not implement serialize/deserialize.
    ///
    /// This path is relative to the remote's base path.
    path: String,
}

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

impl<T: TransmissionProtocol> Unpin for VirtFile<T> {}

impl<T> VirtFile<T>
where
    T: TransmissionProtocol,
{
    /// Create a new file on the remote.
    ///
    /// Attempts to mirror [std::fs::File::create]
    pub async fn create<P: AsRef<Path>>(
        mut ctx: ContextManager<T>,
        path: P,
    ) -> std::io::Result<Self> {
        let res = PrimitiveFsOpsClient::create(
            &mut ctx,
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
    pub async fn open<P: AsRef<Path>>(ctx: ContextManager<T>, path: P) -> std::io::Result<Self> {
        // let res = PrimitiveFsOpsClient

        todo!()
    }

    /// Return metadata from the file
    pub async fn metadata(&self) -> std::io::Result<VirtMetadata> {
        todo!()
    }

    /// Returns the virtual file path as a string
    fn as_path(&self) -> String {
        self.path
            .to_str()
            .and_then(|s| Some(s.to_owned()))
            .unwrap_or_default()
    }

    /// Eagerly executes a read of the file contents, if any.
    fn load_contents(&mut self) -> io::Result<usize> {
        todo!()
    }
}

impl<T> AsyncRead for VirtFile<T>
where
    T: TransmissionProtocol,
{
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let path = self.as_path();

        let mut read_bytes = Box::pin(PrimitiveFsOpsClient::read_bytes(&mut self.ctx, path));

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

impl<T> AsyncWrite for VirtFile<T>
where
    T: TransmissionProtocol,
{
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let path = self.as_path();

        let mut task = Box::pin(PrimitiveFsOpsClient::write_append_bytes(
            &mut self.ctx,
            path,
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

impl<T> VirtOpenOptions<T>
where
    T: TransmissionProtocol,
{
    pub fn new(ctx: ContextManager<T>) -> Self {
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

    pub fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<VirtFile<T>> {
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
    use std::{fs, path::PathBuf};

    fn test() {
        let stuff = fs::read_dir("path").unwrap();

        for item in stuff {
            let i = item.unwrap();
        }
    }

    #[test]
    fn test_pathbuf_stuff() {
        let mut p = PathBuf::new();
        p.push("..");
        p.push("..");

        p.push("first_dir");
        p.push("next_dir");

        println!("{:?}", p.as_path());
    }
}
