//! Virtual (remote) object and related definitions

#![allow(unused)]
use std::{
    clone,
    fmt::Debug,
    fs::{self, FileTimes},
    io::{self, Read},
    path::{Path, PathBuf},
    time::SystemTime,
};

use futures::{ready, AsyncRead, AsyncWrite, FutureExt};
use rfs_core::middleware::{ContextManager, TransmissionProtocol};
use serde::{Deserialize, Serialize};

use crate::interfaces::{FileWriteMode, PrimitiveFsOpsClient};

/// Errors for virtual IO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VirtIOErr {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    Other,
}

/// A file that resides over the network in the remote.
///
/// This struct aims to duplicate some of the most common file operations
/// available in [std::fs::File].
///
/// For simplicity, symlinks residing on the remote will not be treated as files
/// and they will be ignored.
#[derive(Clone, Debug)]
pub struct VirtFile {
    ctx: ContextManager,
    path: PathBuf,

    /// Local metadata. May differ from the remote
    metadata_local: VirtMetadata,

    /// The local byte buffer of the file
    local_buf: Vec<u8>,

    /// Information regarding reads
    read_info: FileReadMeta,
}

#[derive(Clone, Debug, Default)]
struct FileReadMeta {
    /// Current byte position
    pos: usize,

    /// Size of data in file
    len: usize,
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
pub struct VirtDirEntry {
    /// Converted from the `path()` on the remote,
    /// because PathBuf does not implement serialize/deserialize.
    ///
    /// This path is relative to the remote's base path.
    path: String,

    /// Marker for if the entry is for a file or directory
    file: bool,
}

/// Iterator over [VirtDirEntry] items.
#[derive(Clone, Debug)]
pub struct VirtReadDir {
    entries: Vec<VirtDirEntry>,
}

/// Virtual file metadata
#[derive(Clone, Debug, Default)]
pub struct VirtMetadata {
    /// Last file access time
    accessed: Option<SystemTime>,

    /// Last file mutation time
    modified: Option<SystemTime>,

    permissions: VirtPermissions,
}

/// File permissions (rwx)
#[derive(Clone, Debug, Default)]
pub struct VirtPermissions {
    read: (bool, bool, bool),
    write: (bool, bool, bool),
    execute: (bool, bool, bool),
}

impl Unpin for VirtFile {}

impl VirtFile
// where
//     T: TransmissionProtocol,
{
    /// Create a new file on the remote.
    ///
    /// Attempts to mirror [std::fs::File::create]
    pub async fn create<P: AsRef<Path>>(mut ctx: ContextManager, path: P) -> std::io::Result<Self> {
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
            metadata_local: todo!(),
            path: PathBuf::from(path.as_ref()),
            local_buf: Default::default(),
            read_info: Default::default(),
        })
    }

    /// Open an existing file in read-only mode.
    ///
    /// Attempts to mirror [std::fs::File::open]
    pub async fn open<P: AsRef<Path>>(ctx: ContextManager, path: P) -> std::io::Result<Self> {
        // let res = PrimitiveFsOpsClient

        Ok(Self {
            ctx,
            path: path.as_ref().to_path_buf(),
            metadata_local: VirtMetadata::default(),
            local_buf: Default::default(),
            read_info: Default::default(), // this needs to contain file info
        })
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

    /// Read the entire file into a vector.
    pub async fn read_bytes(&mut self) -> io::Result<Vec<u8>> {
        let path = self.as_path();

        let res = PrimitiveFsOpsClient::read_all(&mut self.ctx, path)
            .await
            .map_err(|e| io::Error::from(e));

        res
    }

    /// Write to the file from a vector of bytes.
    pub async fn write_bytes(&mut self, bytes: &[u8], mode: FileWriteMode) -> io::Result<usize> {
        let path = self.as_path();

        let res = PrimitiveFsOpsClient::write_bytes(&mut self.ctx, path, bytes.to_vec(), mode)
            .await
            .map_err(|e| io::Error::from(e))?;

        Ok(bytes.len())
    }
}

impl VirtOpenOptions
// where
//     T: TransmissionProtocol,
{
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

impl From<fs::Metadata> for VirtMetadata {
    fn from(value: fs::Metadata) -> Self {
        Self {
            accessed: value.accessed().ok(),
            modified: value.modified().ok(),
            permissions: value.permissions().into(),
        }
    }
}

impl From<fs::Permissions> for VirtPermissions {
    fn from(value: fs::Permissions) -> Self {
        match value.readonly() {
            true => Self {
                read: (true, true, true),
                write: Default::default(),
                execute: Default::default(),
            },
            false => Self {
                read: Default::default(),
                write: Default::default(),
                execute: Default::default(),
            },
        }
    }
}

impl From<io::Error> for VirtIOErr {
    fn from(value: io::Error) -> Self {
        match value.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            io::ErrorKind::ConnectionRefused => Self::ConnectionRefused,
            io::ErrorKind::ConnectionReset => Self::ConnectionReset,
            io::ErrorKind::ConnectionAborted => Self::ConnectionAborted,
            io::ErrorKind::NotConnected => Self::NotConnected,
            io::ErrorKind::AddrInUse => Self::AddrInUse,
            io::ErrorKind::AddrNotAvailable => Self::AddrNotAvailable,
            io::ErrorKind::BrokenPipe => Self::BrokenPipe,
            io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            io::ErrorKind::WouldBlock => Self::WouldBlock,
            io::ErrorKind::InvalidInput => Self::InvalidInput,
            io::ErrorKind::InvalidData => Self::InvalidData,
            io::ErrorKind::TimedOut => Self::TimedOut,
            io::ErrorKind::WriteZero => Self::WriteZero,
            io::ErrorKind::Interrupted => Self::Interrupted,
            io::ErrorKind::Unsupported => Self::Unsupported,
            io::ErrorKind::UnexpectedEof => Self::UnexpectedEof,
            io::ErrorKind::OutOfMemory => Self::OutOfMemory,
            io::ErrorKind::Other => Self::Other,

            _ => unimplemented!("unstable library variants not handled"),
        }
    }
}

mod testing {
    use std::{fs, io, path::PathBuf};

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
