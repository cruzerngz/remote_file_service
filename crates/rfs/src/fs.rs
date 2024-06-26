//! Virtual file module

mod virt_objects;

use std::{io, path::Path};

pub use virt_objects::*;

use crate::interfaces::PrimitiveFsOpsClient;

/// Read the contents of a file to a string.
///
/// Note that the contents of the file need to be valids UTF-8!
///
/// This can be used in place of opening a file, reading and then closing it.
///
/// This function uses the primitive method [PrimitiveFsOpsClient::read_bytes] and does not
/// create a virtual file.
pub async fn read_to_string<P>(
    mut ctx: rfs_core::middleware::ContextManager,
    path: P,
) -> io::Result<String>
where
    P: AsRef<Path>,
    // T: TransmissionProtocol,
{
    let contents = PrimitiveFsOpsClient::read_all(
        &mut ctx,
        path.as_ref()
            .to_str()
            .and_then(|s| Some(s.to_owned()))
            .unwrap_or_default(),
    )
    .await
    .map_err(|e| io::Error::from(e))?;

    let x = std::str::from_utf8(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}", e)))?;

    Ok(x.to_owned())
}

/// Returns an iterator over the entries of a directory.
pub async fn read_dir<P: AsRef<Path>>(
    mut ctx: rfs_core::middleware::ContextManager,
    path: P,
) -> io::Result<VirtReadDir> {
    let entries = PrimitiveFsOpsClient::read_dir(
        &mut ctx,
        path.as_ref()
            .to_str()
            .and_then(|s| Some(s.to_owned()))
            .unwrap_or_default(),
    )
    .await
    .map_err(|e| io::Error::from(e))?;

    Ok(VirtReadDir::from(entries))
}

/// Create a new directory at the specified path.
pub async fn create_dir<P: AsRef<Path>>(
    mut ctx: rfs_core::middleware::ContextManager,
    path: P,
) -> io::Result<()> {
    PrimitiveFsOpsClient::mkdir(
        &mut ctx,
        path.as_ref()
            .to_str()
            .and_then(|s| Some(s.to_owned()))
            .unwrap_or_default(),
    )
    .await
    .map_err(|e| io::Error::from(e))?
    .map_err(|e| io::Error::from(e))
}

/// Delete a directory and all of its contents.
pub async fn remove_dir<P: AsRef<Path>>(
    mut ctx: rfs_core::middleware::ContextManager,
    path: P,
) -> io::Result<()> {
    PrimitiveFsOpsClient::rmdir(
        &mut ctx,
        path.as_ref()
            .to_str()
            .and_then(|s| Some(s.to_owned()))
            .unwrap_or_default(),
    )
    .await
    .map_err(|e| io::Error::from(e))?
    .map_err(|e| io::Error::from(e))
}

/// Delete a file
pub async fn remove_file<P: AsRef<Path>>(
    mut ctx: rfs_core::middleware::ContextManager,
    path: P,
) -> io::Result<()> {
    PrimitiveFsOpsClient::remove(
        &mut ctx,
        path.as_ref()
            .to_str()
            .and_then(|s| Some(s.to_owned()))
            .unwrap_or_default(),
    )
    .await
    .map_err(|e| io::Error::from(e))?
    .map_err(|e| io::Error::from(e))
}

mod testing {}
