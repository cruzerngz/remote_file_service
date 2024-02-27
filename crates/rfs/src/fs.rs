//! Virtual file module
//!

mod virt_objects;

use std::{io, path::Path};

use futures::AsyncReadExt;
pub use virt_objects::*;

use crate::interfaces::PrimitiveFsOpsClient;

/// Read the contents of a file to a string.
///
/// This can be used in place of opening a file, reading and then closing it.
///
/// This function uses the primitive method [PrimitiveFsOpsClient::read] and does not
/// create a virtual file.
pub async fn read_to_string<P: AsRef<Path>>(
    ctx: rfs_core::middleware::ContextManager,
    path: P,
) -> io::Result<String> {
    let contents = PrimitiveFsOpsClient::read_bytes(
        &ctx,
        path.as_ref()
            .to_str()
            .and_then(|s| Some(s.to_owned()))
            .unwrap_or_default(),
    )
    .await
    .map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("unable to open remote file: {}", e),
        )
    })?;

    let x = std::str::from_utf8(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}", e)))?;

    Ok(x.to_owned())
}
