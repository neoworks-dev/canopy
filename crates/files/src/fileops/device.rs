use std::os::unix::fs::MetadataExt;
use std::path::Path;

use crate::error::Result;

/// Identifies the filesystem a path lives on. The trash spec needs it to decide
/// between the home trash and a per-volume one.
pub(super) fn device_of(path: &Path) -> Result<u64> {
    Ok(std::fs::metadata(path)?.dev())
}

pub(super) fn on_same_volume(first_path: &Path, second_path: &Path) -> Result<bool> {
    Ok(device_of(first_path)? == device_of(second_path)?)
}
