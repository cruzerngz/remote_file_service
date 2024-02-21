//! Virtual file module
//!

mod virt_objects;

use std::{io, path::Path};

use futures::AsyncReadExt;
pub use virt_objects::*;

/// Read the contents of a file to a string.
///
/// This can be used in place of opening a file, reading and then closing it.
pub async fn read_to_string<P: AsRef<Path>>(
    ctx: rfs_core::middleware::ContextManager,
    path: P,
) -> io::Result<String> {
    let mut file = VirtFile::open(ctx, path).await?;

    let mut buf = String::new();
    file.read_to_string(&mut buf).await?;

    Ok(buf)
}
