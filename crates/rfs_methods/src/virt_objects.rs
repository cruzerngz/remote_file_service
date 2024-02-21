//! Virtual (remote) object definitions

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A file that resides over the network in the remote.
///
/// This struct aims to duplicate some of the most common file operations
/// available in [std::fs::File].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtFile {
    path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtDirEntry;
