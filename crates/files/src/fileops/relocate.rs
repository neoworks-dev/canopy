use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::copy::{copy_resolved, sorted_children};
use super::names::is_directory;
use super::operation::Operation;
use super::trash::remove_all;
use crate::error::Result;

/// Moves `source` into `destination_directory` and reports where it landed,
/// which is what the undo step needs to put it back.
fn move_into(source: &Path, destination_directory: &Path, operation: &Operation) -> Result<PathBuf> {
    let name = source.file_name().unwrap_or_default();
    let target = operation.resolve_target(source, &destination_directory.join(name))?;
    move_resolved(source, &target, operation)?;
    Ok(target)
}

pub(super) fn move_resolved(source: &Path, target: &Path, operation: &Operation) -> Result<()> {
    operation.wait_while_paused()?;
    operation.set_current(source);
    if is_directory(source) && is_directory(target) {
        return merge_directory(source, target, operation);
    }
    clear_directory_target(target)?;
    rename_or_copy(source, target, operation)
}

/// Removes a directory standing where a non-directory is about to land, a
/// combination rename refuses.
fn clear_directory_target(target: &Path) -> Result<()> {
    if !is_directory(target) {
        return Ok(());
    }
    remove_all(target)
}

/// Falls back to copy-then-delete when source and target live on different
/// filesystems, where rename cannot work.
fn rename_or_copy(source: &Path, target: &Path, operation: &Operation) -> Result<()> {
    let Err(error) = fs::rename(source, target) else {
        return Ok(());
    };
    if error.kind() != ErrorKind::CrossesDevices {
        return Err(error.into());
    }
    copy_resolved(source, target, operation)?;
    remove_all(source)
}

fn merge_directory(source: &Path, target: &Path, operation: &Operation) -> Result<()> {
    for child in sorted_children(source)? {
        move_into(&child, target, operation)?;
    }
    remove_emptied(source)
}

/// Tolerates leftovers: a skipped child legitimately keeps the source directory
/// alive after a merge.
fn remove_emptied(source: &Path) -> Result<()> {
    match fs::remove_dir(source) {
        Err(error) if error.kind() == ErrorKind::DirectoryNotEmpty => Ok(()),
        outcome => Ok(outcome?),
    }
}
