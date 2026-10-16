use canopy_model::{Entry, describe};
use walkdir::{DirEntry, WalkDir};

use super::has;

/// Enough matches to work with. The tags live on the files rather than in an
/// index, so finding them means walking the tree and asking each file, and a
/// walk that never stops would be one nobody waits for.
const SEARCH_LIMIT: usize = 500;

/// Directories a tagged file would not be put in on purpose, skipped so a walk
/// spends its time where the user's own files are.
const SKIPPED_DIRECTORY_NAMES: [&str; 4] = ["node_modules", "__pycache__", "target", "vendor"];

/// Lists the files under `root` that carry one tag.
pub fn search(root: &str, name: &str) -> Vec<Entry> {
    let wanted = name.trim();
    let mut found = Vec::new();
    if wanted.is_empty() {
        return found;
    }
    let walk = WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| !is_skipped_directory(entry));
    // A directory that cannot be read is stepped over: one unreadable folder
    // says nothing about the rest of the tree.
    for entry in walk.filter_map(Result::ok) {
        let path = entry.path().to_string_lossy();
        if has(&path, wanted) {
            found.push(describe(&path));
        }
        if found.len() >= SEARCH_LIMIT {
            break;
        }
    }
    found
}

fn is_skipped_directory(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() || entry.depth() == 0 {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    name.starts_with('.') || SKIPPED_DIRECTORY_NAMES.contains(&name.as_ref())
}
