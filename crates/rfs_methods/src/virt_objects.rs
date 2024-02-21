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

/// Virtual directory
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtDirEntry;

/// Iterator over [VirtReadDir] entries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtReadDir;

/// Virtual file metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtMetadata;
