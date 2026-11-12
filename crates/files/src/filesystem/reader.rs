use std::fs::DirEntry;
use std::path::Path;

use canopy_model::{Entry, Listing, Order, entry_from_metadata, sort_entries};

use crate::error::Result;
use crate::filesystem::path::{parent_path, resolve_path};

/// Lists one directory in the given order. Sorting stays on this side so it
/// holds for the whole directory, not just the entries the listing currently
/// has mounted.
pub fn read_directory(requested_path: &str, order: &Order) -> Result<Listing> {
    let absolute_path = resolve_path(requested_path)?;
    let directory_entries = std::fs::read_dir(&absolute_path)?;
    let mut entries = Vec::new();
    for directory_entry in directory_entries {
        entries.push(to_entry(&absolute_path, &directory_entry?));
    }
    sort_entries(&mut entries, order);
    Ok(Listing {
        path: absolute_path.to_string_lossy().into_owned(),
        parent_path: parent_path(&absolute_path),
        entries,
    })
}

/// An entry whose metadata cannot be read (a race with deletion) still shows
/// up with its name and path. `DirEntry::metadata` does not follow symlinks;
/// `entry_from_metadata` follows them itself.
fn to_entry(parent_directory: &Path, directory_entry: &DirEntry) -> Entry {
    let name = directory_entry.file_name().to_string_lossy().into_owned();
    let path = parent_directory.join(directory_entry.file_name()).to_string_lossy().into_owned();
    match directory_entry.metadata() {
        Ok(metadata) => entry_from_metadata(name, path, &metadata),
        Err(_) => {
            let is_dir = directory_entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            Entry { name, path, is_dir, ..Entry::default() }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn names(listing: &Listing) -> Vec<&str> {
        listing.entries.iter().map(|entry| entry.name.as_str()).collect()
    }

    #[test]
    fn directories_come_first_then_names() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("banana.txt"), "x").unwrap();
        fs::write(root.path().join("Apple.txt"), "x").unwrap();
        fs::create_dir(root.path().join("zebra")).unwrap();
        fs::create_dir(root.path().join("Alpha")).unwrap();

        let listing = read_directory(root.path().to_str().unwrap(), &Order::default()).unwrap();

        assert_eq!(names(&listing), ["Alpha", "zebra", "Apple.txt", "banana.txt"]);
    }

    #[test]
    fn size_parent_and_time_are_reported() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("data.bin"), "0123456789").unwrap();

        let listing = read_directory(root.path().to_str().unwrap(), &Order::default()).unwrap();

        assert_eq!(listing.parent_path, root.path().parent().unwrap().to_str().unwrap());
        assert_eq!(listing.entries.len(), 1);
        assert_eq!(listing.entries[0].size, 10);
        assert!(!listing.entries[0].mod_time.is_empty());
    }

    #[test]
    fn permissions_are_reported() {
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("script.sh");
        fs::write(&script, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o750)).unwrap();
        fs::create_dir(root.path().join("nested")).unwrap();

        let listing = read_directory(root.path().to_str().unwrap(), &Order::default()).unwrap();

        let mode_of = |name: &str| listing.entries.iter().find(|entry| entry.name == name).unwrap().mode.clone();
        assert_eq!(mode_of("script.sh"), "-rwxr-x---");
        assert!(mode_of("nested").starts_with('d'));
    }

    #[test]
    fn a_missing_directory_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("does-not-exist");
        assert!(read_directory(missing.to_str().unwrap(), &Order::default()).is_err());
    }

    #[test]
    fn a_link_to_a_directory_lists_among_the_directories() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("target")).unwrap();
        std::os::unix::fs::symlink("target", root.path().join("b-link")).unwrap();
        std::os::unix::fs::symlink("missing", root.path().join("a-broken")).unwrap();
        fs::write(root.path().join("a-file"), "x").unwrap();

        let listing = read_directory(root.path().to_str().unwrap(), &Order::default()).unwrap();

        assert_eq!(names(&listing), ["b-link", "target", "a-broken", "a-file"]);
        let link = &listing.entries[0];
        assert!(link.is_dir && link.is_symlink);
        assert_eq!(link.link_target, "target");
        let broken = &listing.entries[2];
        assert!(!broken.is_dir && broken.is_symlink);
    }

    #[test]
    fn hidden_entries_are_included() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(".hidden"), "x").unwrap();
        fs::write(root.path().join("visible"), "x").unwrap();

        let listing = read_directory(root.path().to_str().unwrap(), &Order::default()).unwrap();

        assert_eq!(names(&listing), [".hidden", "visible"]);
    }
}
