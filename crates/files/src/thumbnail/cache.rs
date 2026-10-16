//! The shared thumbnail directory every desktop file manager reads, so an image
//! rendered here is one Dolphin and Nautilus do not have to render again.

use std::fs::Metadata;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use md5::{Digest, Md5};

use super::png_text::thumbnail_metadata;
use crate::url_path::escape_path;

const CACHE_DIRECTORY_NAME: &str = "thumbnails";
const SIZE_DIRECTORY_NAME: &str = "large";

/// Keeps the staging files of concurrent writers apart.
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

/// The identity a cached thumbnail is filed under, and the value stored in the
/// image so a hash collision can be told apart from a hit.
pub(super) fn source_uri(absolute_path: &Path) -> String {
    format!("file://{}", escape_path(&absolute_path.to_string_lossy()))
}

pub(super) fn modified_stamp(metadata: &Metadata) -> String {
    metadata.mtime().to_string()
}

/// The standard location for one source: the MD5 of its URI, which is a naming
/// rule rather than a security claim. `directory` overrides the user's cache.
pub(super) fn cache_path(directory: Option<&Path>, uri: &str) -> Option<PathBuf> {
    let directory = match directory {
        Some(directory) => directory.to_path_buf(),
        None => user_cache_directory()?.join(CACHE_DIRECTORY_NAME).join(SIZE_DIRECTORY_NAME),
    };
    let sum = Md5::digest(uri.as_bytes());
    Some(directory.join(format!("{}.png", crate::hex::encode(&sum))))
}

/// `$XDG_CACHE_HOME`, or `~/.cache` when it is not set.
fn user_cache_directory() -> Option<PathBuf> {
    match std::env::var_os("XDG_CACHE_HOME").filter(|value| !value.is_empty()) {
        Some(directory) => Some(PathBuf::from(directory)).filter(|directory| directory.is_absolute()),
        None => std::env::home_dir().map(|home| home.join(".cache")),
    }
}

/// A stored thumbnail that was made from this exact source at this exact
/// modification time, and nothing otherwise.
pub(super) fn read_cached(path: &Path, uri: &str, modified_at: &str) -> Option<Vec<u8>> {
    let stored = std::fs::read(path).ok()?;
    let (stored_uri, stored_modified_at) = thumbnail_metadata(&stored);
    if stored_uri != uri || stored_modified_at != modified_at {
        return None;
    }
    Some(stored)
}

/// Stages the image through a temporary file, so a reader that arrives
/// mid-write sees either the old thumbnail or the new one. A cache that cannot
/// be written is not worth failing a render over.
pub(super) fn write_cached(path: &Path, image: &[u8]) {
    let Some(directory) = path.parent() else {
        return;
    };
    if create_private_directory(directory).is_err() {
        return;
    }
    let temporary =
        directory.join(format!(".thumbnail-{}-{}", std::process::id(), NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)));
    if write_private_file(&temporary, image).is_err() || std::fs::rename(&temporary, path).is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
}

fn create_private_directory(directory: &Path) -> std::io::Result<()> {
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(directory)
}

fn write_private_file(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    file.write_all(contents)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_uri_escapes_the_path() {
        assert_eq!(source_uri(Path::new("/photos/a b.png")), "file:///photos/a%20b.png");
    }

    #[test]
    fn the_cache_name_is_the_md5_of_the_uri() {
        let path = cache_path(Some(Path::new("/cache")), "file:///a").unwrap();
        assert_eq!(path, PathBuf::from("/cache/015d15b77423dd0879dafd0916b42aae.png"));
    }
}
