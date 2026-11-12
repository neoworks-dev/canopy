use std::fs::Metadata;
use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Local, SecondsFormat};
use serde::{Deserialize, Serialize};

use crate::mode::mode_string;
use crate::owner::owner_string;

/// A single item inside a directory, in the shape the UI reads.
///
/// A symlink reports the kind, size and times of what it points to, so a link
/// to a folder opens like a folder; its mode and owner are the link's own.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: i64,
    pub mod_time: String,
    /// Empty when the filesystem does not record when a file was created.
    pub created_time: String,
    pub mode: String,
    /// `user:group`, by name where the system knows one.
    pub owner: String,
    pub is_symlink: bool,
    /// The link's target as stored, unresolved. Empty for anything but a symlink.
    pub link_target: String,
    /// The instant behind `mod_time`, so sorting compares times rather than strings.
    #[serde(skip)]
    pub modified_at: Option<SystemTime>,
    #[serde(skip)]
    pub created_at: Option<SystemTime>,
}

/// The result of reading one directory.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub path: String,
    pub parent_path: String,
    pub entries: Vec<Entry>,
}

/// The entry for one path. A path whose metadata cannot be read still has its
/// name and path.
pub fn describe(path: &str) -> Entry {
    let mut entry = Entry { name: base_name(path), path: path.to_string(), ..Entry::default() };
    let Ok(link_metadata) = std::fs::symlink_metadata(path) else {
        return entry;
    };
    fill_from_metadata(&mut entry, &link_metadata);
    entry
}

/// The entry for a child of a directory whose metadata, read without following
/// a final symlink, is already known.
pub fn entry_from_metadata(name: String, path: String, link_metadata: &Metadata) -> Entry {
    let mut entry = Entry { name, path, ..Entry::default() };
    fill_from_metadata(&mut entry, link_metadata);
    entry
}

/// A broken symlink keeps the link's own size and times.
fn fill_from_metadata(entry: &mut Entry, link_metadata: &Metadata) {
    entry.mode = mode_string(link_metadata);
    entry.owner = owner_string(link_metadata);
    if !link_metadata.is_symlink() {
        fill_from_target(entry, link_metadata);
        return;
    }
    entry.is_symlink = true;
    if let Ok(target) = std::fs::read_link(&entry.path) {
        entry.link_target = target.to_string_lossy().into_owned();
    }
    match std::fs::metadata(&entry.path) {
        Ok(target_metadata) => fill_from_target(entry, &target_metadata),
        Err(_) => fill_from_target(entry, link_metadata),
    }
}

fn fill_from_target(entry: &mut Entry, metadata: &Metadata) {
    entry.is_dir = metadata.is_dir();
    entry.size = metadata.len() as i64;
    if let Ok(modified) = metadata.modified() {
        entry.modified_at = Some(modified);
        entry.mod_time = format_time(modified);
    }
    if let Ok(created) = metadata.created() {
        entry.created_at = Some(created);
        entry.created_time = format_time(created);
    }
}

/// RFC 3339 in local time, to the second.
pub fn format_time(time: SystemTime) -> String {
    let local: DateTime<Local> = time.into();
    local.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn base_name(path: &str) -> String {
    match Path::new(path).file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;

    use super::*;

    fn text(path: &Path) -> &str {
        path.to_str().unwrap()
    }

    #[test]
    fn a_plain_file_is_not_a_link() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("note.txt");
        fs::write(&file, "abc").unwrap();

        let entry = describe(text(&file));

        assert!(!entry.is_dir);
        assert!(!entry.is_symlink);
        assert_eq!(entry.link_target, "");
        assert_eq!(entry.size, 3);
        assert!(entry.owner.contains(':'));
    }

    #[test]
    fn a_link_to_a_directory_is_a_directory() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("real")).unwrap();
        let link = root.path().join("link");
        symlink("real", &link).unwrap();

        let entry = describe(text(&link));

        assert!(entry.is_dir);
        assert!(entry.is_symlink);
        assert_eq!(entry.link_target, "real");
        assert!(entry.mode.starts_with('L'));
    }

    #[test]
    fn a_link_to_a_file_reports_the_file_size() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("data.bin"), "0123456789").unwrap();
        let link = root.path().join("link");
        symlink(root.path().join("data.bin"), &link).unwrap();

        let entry = describe(text(&link));

        assert!(!entry.is_dir);
        assert!(entry.is_symlink);
        assert_eq!(entry.size, 10);
    }

    #[test]
    fn a_broken_link_is_a_link_but_not_a_directory() {
        let root = tempfile::tempdir().unwrap();
        let link = root.path().join("dangling");
        symlink("missing", &link).unwrap();

        let entry = describe(text(&link));

        assert!(!entry.is_dir);
        assert!(entry.is_symlink);
        assert_eq!(entry.link_target, "missing");
    }

    #[test]
    fn new_fields_serialise_in_camel_case() {
        let entry = Entry { created_time: "t".into(), owner: "a:b".into(), is_symlink: true, link_target: "x".into(), ..Entry::default() };

        let json = serde_json::to_value(&entry).unwrap();

        assert_eq!(json["createdTime"], "t");
        assert_eq!(json["owner"], "a:b");
        assert_eq!(json["isSymlink"], true);
        assert_eq!(json["linkTarget"], "x");
    }
}
