//! Virtual (remote) object and related definitions

use std::{
    borrow::Cow,
    error::Error,
    fmt::{Debug, Display},
    fs::{self, DirEntry},
    io::{self},
    net::{SocketAddr, SocketAddrV4},
    ops::Deref,
    path::{Path, PathBuf},
    time::SystemTime,
};

use rfs_core::{deserialize_packed, middleware::ContextManager};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::interfaces::{CallbackOpsClient, FileUpdate, PrimitiveFsOpsClient};

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
    Other(String),
}

/// A file that resides over the network in the remote.
///
/// This struct aims to duplicate some of the most common file operations
/// available in [std::fs::File].
///
/// For simplicity, symlinks residing on the remote will not be treated as files
/// and they will be ignored.
#[derive(Clone, Debug)]
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtDirEntry {
    /// Converted from the `path()` on the remote,
    /// because PathBuf does not implement serialize/deserialize.
    ///
    /// This path is relative to the remote's base path.
    pub path: String,

    /// Marker for if the entry is for a file or directory
    pub file: bool,
}

/// Iterator over [VirtDirEntry] items.
#[derive(Clone, Debug)]
pub struct VirtReadDir {
    pub entries: Vec<VirtDirEntry>,
}

/// Virtual file metadata
#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct VirtMetadata {
    /// Last file access time
    accessed: Option<SystemTime>,

    /// Last file mutation time
    modified: Option<SystemTime>,

    permissions: VirtPermissions,
}

/// File permissions (rwx)
#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct VirtPermissions {
    read: (bool, bool, bool),
    write: (bool, bool, bool),
    execute: (bool, bool, bool),
}

impl Unpin for VirtFile {}

impl Display for VirtIOErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err_msg: Cow<str> = match self {
            VirtIOErr::NotFound => "the requested item was not found".into(),
            VirtIOErr::PermissionDenied => "insufficient permissions".into(),
            VirtIOErr::ConnectionRefused => "connection refused".into(),
            VirtIOErr::ConnectionReset => "connection reset".into(),
            VirtIOErr::ConnectionAborted => "connection aborted".into(),
            VirtIOErr::NotConnected => "not connected".into(),
            VirtIOErr::AddrInUse => "address in use".into(),
            VirtIOErr::AddrNotAvailable => "address not available".into(),
            VirtIOErr::BrokenPipe => "broken pipe".into(),
            VirtIOErr::AlreadyExists => "item already exists".into(),
            VirtIOErr::WouldBlock => "operation would block".into(),
            VirtIOErr::InvalidInput => "invalid input".into(),
            VirtIOErr::InvalidData => "invalid data".into(),
            VirtIOErr::TimedOut => "request timed out".into(),
            VirtIOErr::WriteZero => "write zero bytes".into(),
            VirtIOErr::Interrupted => "operation interrupted".into(),
            VirtIOErr::Unsupported => "operation unsupported".into(),
            VirtIOErr::UnexpectedEof => "unexpected end of file".into(),
            VirtIOErr::OutOfMemory => "out of memory".into(),
            VirtIOErr::Other(msg) => format!("other error: {}", msg).into(),
        };

        write!(f, "{}", err_msg)
    }
}

impl std::error::Error for VirtIOErr {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl VirtDirEntry {
    /// Create a new virtual directory entry from a local directory entry and the server's base path.
    ///
    /// If base is not a prefix of the entry's path, this method will return None.
    /// This is used instead of `From<DirEntry>` because of the additional base path requirement.
    pub fn from_dir_entry<P: AsRef<Path>>(value: DirEntry, base: P) -> Option<Self> {
        let path = value.path();
        let rel = path.strip_prefix(base.as_ref()).ok()?;

        Some(Self {
            path: rel
                .to_str()
                .and_then(|s| Some(s.to_owned()))
                .unwrap_or_default(),
            file: path.is_file(),
        })
    }

    /// Returns the path of the entry
    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    /// Returns true if the entry is a file
    pub fn is_file(&self) -> bool {
        self.file
    }

    pub fn metadata(&self) -> VirtMetadata {
        todo!()
    }
}

// deref inner vector
impl Deref for VirtReadDir {
    type Target = Vec<VirtDirEntry>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl<E: AsRef<[VirtDirEntry]>> From<E> for VirtReadDir {
    fn from(value: E) -> Self {
        Self {
            entries: value.as_ref().iter().map(|entry| entry.clone()).collect(),
        }
    }
}

impl VirtFile {
    /// Create a new file on the remote.
    ///
    /// Attempts to mirror [std::fs::File::create]
    pub async fn create<P: AsRef<Path>>(mut ctx: ContextManager, path: P) -> std::io::Result<Self> {
        let _res = PrimitiveFsOpsClient::create(
            &mut ctx,
            path.as_ref()
                .to_str()
                .and_then(|s| Some(s.to_string()))
                .unwrap_or_default(),
        )
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "invocation error"))?
        .map_err(|e| io::Error::from(e))?;

        Ok(Self {
            ctx,
            metadata_local: Default::default(),
            path: PathBuf::from(path.as_ref()),
            local_buf: Default::default(),
            read_info: Default::default(),
        })
    }

    /// Open an existing file in read-only mode.
    ///
    /// Attempts to mirror [std::fs::File::open]
    pub async fn open<P: AsRef<Path>>(mut ctx: ContextManager, path: P) -> std::io::Result<Self> {
        // let res = PrimitiveFsOpsClient

        let contents =
            PrimitiveFsOpsClient::read_all(&mut ctx, path.as_ref().to_str().unwrap().to_string())
                .await
                .map_err(|e| io::Error::from(e))?;

        // load contents into local buffer
        Ok(Self {
            ctx,
            path: path.as_ref().to_path_buf(),
            metadata_local: VirtMetadata::default(),
            local_buf: contents,
            read_info: Default::default(), // this needs to contain file info
        })
    }

    /// Return metadata from the file
    pub async fn metadata(&self) -> std::io::Result<VirtMetadata> {
        todo!()
    }

    /// Returns the virtual file path as a string
    pub fn as_path(&self) -> String {
        self.path
            .to_str()
            .and_then(|s| Some(s.to_owned()))
            .unwrap_or_default()
    }

    /// Returns the locally cached file contents
    pub fn local_cache(&self) -> &[u8] {
        &self.local_buf
    }

    /// Read the entire file into a vector.
    pub async fn read_bytes(&mut self) -> io::Result<Vec<u8>> {
        let path = self.as_path();

        let res = PrimitiveFsOpsClient::read_all(&mut self.ctx, path)
            .await
            .map_err(|e| io::Error::from(e))?;

        self.local_buf = res.clone();

        Ok(res)
    }

    /// Write to the file from a vector of bytes.
    pub async fn write_bytes(&mut self, data: FileUpdate) -> io::Result<usize> {
        let path = self.as_path();

        let _res = PrimitiveFsOpsClient::write_bytes(&mut self.ctx, path, data.clone())
            .await
            .map_err(|e| io::Error::from(e))?;

        let size = data.len();
        // update local buf only after write request completes
        self.local_buf = data.update_file(&self.local_buf);

        Ok(size)
    }

    /// Blocks until the file is updated. The new file contents are returned,
    /// as well as the update information.
    pub async fn watch(&mut self) -> io::Result<(Vec<u8>, FileUpdate)> {
        // this is the return socket the remote will send callbacks to
        let ret_sock = self.ctx.generate_socket().await?;

        let _ = CallbackOpsClient::register_file_update(
            &mut self.ctx,
            self.path
                .to_str()
                .ok_or(io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?
                .to_string(),
            sockaddr_to_v4(ret_sock.local_addr()?)?,
        )
        .await?
        .map_err(|e| io::Error::from(e))?;

        let resp = self.ctx.listen(&ret_sock).await?;
        log::debug!("watch triggered");

        let update: FileUpdate = deserialize_packed(&resp)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "deserialization failed"))?;

        self.local_buf = update.clone().update_file(&self.local_buf);

        Ok((self.local_buf.clone(), update))
    }

    /// Watch for file updates on the returned channel.
    ///
    /// The local file buffer will need to be manually updated.
    /// The updated file contents are: file path and update info.
    pub async fn watch_chan(&self) -> io::Result<mpsc::Receiver<io::Result<(String, FileUpdate)>>> {
        // this is the return socket the remote will send callbacks to
        let ret_sock = self.ctx.generate_socket().await?;

        let _ = CallbackOpsClient::register_file_update(
            &mut self.ctx.clone(),
            self.path
                .to_str()
                .ok_or(io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?
                .to_string(),
            sockaddr_to_v4(ret_sock.local_addr()?)?,
        )
        .await?
        .map_err(|e| io::Error::from(e))?;

        let (tx, rx) = mpsc::channel(3);

        let mut ctx_clone = self.ctx.clone();
        let file_path = self.as_path();

        tokio::spawn(async move {
            let resp = match ctx_clone.listen(&ret_sock).await {
                Ok(r) => r,
                Err(e) => {
                    log::error!("watch callback failed: {:?}", e);
                    tx.send(Err(io::Error::from(e)))
                        .await
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                        .unwrap();
                    return;
                }
            };

            let update: FileUpdate = match deserialize_packed(&resp)
                .map_err(|_e| io::Error::new(io::ErrorKind::InvalidData, "deserialization failed"))
            {
                Ok(upd) => upd,
                Err(e) => {
                    tx.send(Err(e))
                        .await
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                        .unwrap();
                    return;
                }
            };

            tx.send(Ok((file_path, update))).await.unwrap();
        });

        Ok(rx)
    }

    /// Update the local contents of the file.
    ///
    /// If the remote file needs to be updated, use `write_bytes` instead.
    pub fn update_bytes(&mut self, upd: FileUpdate) {
        self.local_buf = upd.update_file(&self.local_buf);
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

    #[allow(unused_variables)]
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

            #[allow(unreachable_patterns)]
            _ => todo!(),
        }
    }
}

/// Converts a socket address to a V4 one.
/// V6 addresses will return an error.
fn sockaddr_to_v4(addr: SocketAddr) -> io::Result<SocketAddrV4> {
    match addr {
        SocketAddr::V4(a) => Ok(a),
        SocketAddr::V6(_) => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "IPv6 addresses are not supported",
        )),
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
            io::ErrorKind::Other => Self::Other(
                value
                    .source()
                    .and_then(|s| Some(s.to_string()))
                    .unwrap_or_default(),
            ),

            _ => unimplemented!("unstable library variants not handled"),
        }
    }
}

impl From<VirtIOErr> for io::Error {
    fn from(value: VirtIOErr) -> Self {
        match value {
            VirtIOErr::NotFound => io::Error::new(io::ErrorKind::NotFound, ""),
            VirtIOErr::PermissionDenied => io::Error::new(io::ErrorKind::PermissionDenied, ""),
            VirtIOErr::ConnectionRefused => io::Error::new(io::ErrorKind::ConnectionRefused, ""),
            VirtIOErr::ConnectionReset => io::Error::new(io::ErrorKind::ConnectionReset, ""),
            VirtIOErr::ConnectionAborted => io::Error::new(io::ErrorKind::ConnectionAborted, ""),
            VirtIOErr::NotConnected => io::Error::new(io::ErrorKind::NotConnected, ""),
            VirtIOErr::AddrInUse => io::Error::new(io::ErrorKind::AddrInUse, ""),
            VirtIOErr::AddrNotAvailable => io::Error::new(io::ErrorKind::AddrNotAvailable, ""),
            VirtIOErr::BrokenPipe => io::Error::new(io::ErrorKind::BrokenPipe, ""),
            VirtIOErr::AlreadyExists => io::Error::new(io::ErrorKind::AlreadyExists, ""),
            VirtIOErr::WouldBlock => io::Error::new(io::ErrorKind::WouldBlock, ""),
            VirtIOErr::InvalidInput => io::Error::new(io::ErrorKind::InvalidInput, ""),
            VirtIOErr::InvalidData => io::Error::new(io::ErrorKind::InvalidData, ""),
            VirtIOErr::TimedOut => io::Error::new(io::ErrorKind::TimedOut, ""),
            VirtIOErr::WriteZero => io::Error::new(io::ErrorKind::WriteZero, ""),
            VirtIOErr::Interrupted => io::Error::new(io::ErrorKind::Interrupted, ""),
            VirtIOErr::Unsupported => io::Error::new(io::ErrorKind::Unsupported, ""),
            VirtIOErr::UnexpectedEof => io::Error::new(io::ErrorKind::UnexpectedEof, ""),
            VirtIOErr::OutOfMemory => io::Error::new(io::ErrorKind::OutOfMemory, ""),
            VirtIOErr::Other(msg) => io::Error::new(io::ErrorKind::Other, msg),
        }
    }
}

#[allow(unused)]
mod testing {

    use super::*;
    use std::fs;

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

    #[test]
    fn test_virt_dir_entry() {
        let dir_entry = VirtDirEntry {
            path: "top_dir/next_dir".to_string(),
            file: todo!(),
        };
    }
}
