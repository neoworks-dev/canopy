use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use filetime::FileTime;

use super::operation::Operation;
use super::trash::remove_all;
use crate::error::Result;

const COPY_BUFFER_SIZE: usize = 256 * 1024;

/// Copies `source` into `destination_directory` under its own base name, asking
/// the operation what to do when the target is already taken.
fn copy_into(source: &Path, destination_directory: &Path, operation: &Operation) -> Result<()> {
    let name = source.file_name().unwrap_or_default();
    let target = operation.resolve_target(source, &destination_directory.join(name))?;
    copy_resolved(source, &target, operation)
}

pub(super) fn copy_resolved(source: &Path, target: &Path, operation: &Operation) -> Result<()> {
    operation.wait_while_paused()?;
    operation.set_current(source);
    let metadata = fs::symlink_metadata(source)?;
    if metadata.is_symlink() {
        return copy_symlink(source, target);
    }
    if metadata.is_dir() {
        return copy_directory(source, target, &metadata, operation);
    }
    copy_file(source, target, &metadata, operation)
}

fn copy_symlink(source: &Path, target: &Path) -> Result<()> {
    let link_target = fs::read_link(source)?;
    remove_all(target)?;
    std::os::unix::fs::symlink(link_target, target)?;
    Ok(())
}

/// Merges into an existing target rather than replacing it, so a conflict on a
/// directory never destroys entries the source does not carry.
fn copy_directory(source: &Path, target: &Path, metadata: &Metadata, operation: &Operation) -> Result<()> {
    fs::DirBuilder::new().recursive(true).mode(metadata.permissions().mode() & 0o777).create(target)?;
    for child in sorted_children(source)? {
        copy_into(&child, target, operation)?;
    }
    apply_metadata(target, metadata)
}

/// The paths directly inside a directory, in name order.
pub(super) fn sorted_children(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut children = Vec::new();
    for child in fs::read_dir(directory)? {
        children.push(child?.path());
    }
    children.sort();
    Ok(children)
}

fn copy_file(source: &Path, target: &Path, metadata: &Metadata, operation: &Operation) -> Result<()> {
    let mut source_file = File::open(source)?;
    let mut target_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(metadata.permissions().mode() & 0o777)
        .open(target)?;
    if let Err(error) = write_contents(&mut target_file, &mut source_file, operation) {
        return Err(abandon_target(target_file, target, error));
    }
    drop(target_file);
    apply_metadata(target, metadata)
}

/// Removes a half-written file so a cancelled or failed copy does not leave a
/// truncated one behind.
pub(super) fn abandon_target(target_file: File, target: &Path, cause: crate::Error) -> crate::Error {
    drop(target_file);
    let _ = fs::remove_file(target);
    cause
}

/// Copies in chunks so pause and cancel take effect during a large file rather
/// than only between files, and so the byte counter advances inside one file
/// instead of jumping when it completes.
pub(super) fn write_contents(
    target: &mut (impl Write + ?Sized),
    source: &mut (impl Read + ?Sized),
    operation: &Operation,
) -> Result<()> {
    let mut buffer = vec![0u8; COPY_BUFFER_SIZE];
    loop {
        operation.wait_while_paused()?;
        let read_count = read_chunk(source, &mut buffer)?;
        if read_count == 0 {
            return Ok(());
        }
        target.write_all(&buffer[..read_count])?;
        operation.add_bytes(read_count as i64);
    }
}

fn read_chunk(source: &mut (impl Read + ?Sized), buffer: &mut [u8]) -> std::io::Result<usize> {
    loop {
        match source.read(buffer) {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            outcome => return outcome,
        }
    }
}

fn apply_metadata(path: &Path, metadata: &Metadata) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(metadata.permissions().mode() & 0o777))?;
    let modified_at = FileTime::from_last_modification_time(metadata);
    filetime::set_file_times(path, modified_at, modified_at)?;
    Ok(())
}
