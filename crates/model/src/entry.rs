use std::fs::Metadata;
use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Local, SecondsFormat};
use serde::{Deserialize, Serialize};

use crate::mode::mode_string;

/// A single item inside a directory, in the shape the UI reads.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: i64,
    pub mod_time: String,
    pub mode: String,
    /// The instant behind `mod_time`, so sorting compares times rather than strings.
    #[serde(skip)]
    pub modified_at: Option<SystemTime>,
}

/// The result of reading one directory.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub path: String,
    pub parent_path: String,
    pub entries: Vec<Entry>,
}

/// The entry for one path, without following a final symlink. A path whose
/// metadata cannot be read still has its name and path.
pub fn describe(path: &str) -> Entry {
    let mut entry = Entry { name: base_name(path), path: path.to_string(), ..Entry::default() };
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return entry;
    };
    fill_from_metadata(&mut entry, &metadata);
    entry
}

/// The entry for a child of a directory whose metadata is already known.
pub fn entry_from_metadata(name: String, path: String, metadata: &Metadata) -> Entry {
    let mut entry = Entry { name, path, ..Entry::default() };
    fill_from_metadata(&mut entry, metadata);
    entry
}

fn fill_from_metadata(entry: &mut Entry, metadata: &Metadata) {
    entry.is_dir = metadata.is_dir();
    entry.size = metadata.len() as i64;
    entry.mode = mode_string(metadata);
    if let Ok(modified) = metadata.modified() {
        entry.modified_at = Some(modified);
        entry.mod_time = format_time(modified);
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
