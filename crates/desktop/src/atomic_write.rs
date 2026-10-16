use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use tempfile::NamedTempFile;

/// Writes through a temporary file in the same directory and renames it into
/// place, so a reader sees the old contents or the new ones, never a mix.
pub(crate) fn write_atomically(path: &Path, temporary_prefix: &str, contents: &[u8], mode: u32) -> io::Result<()> {
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temporary = NamedTempFile::with_prefix_in(temporary_prefix, directory)?;
    temporary.write_all(contents)?;
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(mode))?;
    temporary.persist(path).map_err(|failure| failure.error)?;
    Ok(())
}
