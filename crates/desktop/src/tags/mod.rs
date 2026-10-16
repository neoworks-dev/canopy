//! Reads and writes the labels a user puts on files.
//!
//! A file's tags live on the file itself, in the `user.xdg.tags` extended
//! attribute the freedesktop tools agree on, so they travel with it when it is
//! moved and are read by the other file managers. Filesystems without extended
//! attributes simply cannot be tagged, which is reported rather than worked
//! around.

mod attribute;
mod search;
mod vocabulary;

use std::collections::BTreeSet;
use std::io;

pub use search::search;
pub use vocabulary::Vocabulary;

/// The tags on one file, sorted; empty when it has none or when the filesystem
/// cannot carry them.
pub fn of(path: &str) -> Vec<String> {
    match attribute::read(path) {
        Some(stored) => parse(&stored),
        None => Vec::new(),
    }
}

/// Replaces the tags on one file. An empty list removes the attribute rather
/// than leaving an empty one behind.
pub fn set(path: &str, names: &[String]) -> io::Result<()> {
    let cleaned = normalize(names);
    if cleaned.is_empty() {
        return attribute::clear(path);
    }
    attribute::write(path, cleaned.join(",").as_bytes())
}

/// Adds one tag to the ones the file already has.
pub fn add(path: &str, name: &str) -> io::Result<()> {
    if name.trim().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "a tag needs a name"));
    }
    let mut names = of(path);
    names.push(name.to_string());
    set(path, &names)
}

pub fn remove(path: &str, name: &str) -> io::Result<()> {
    set(path, &without(&of(path), name))
}

/// Whether a file carries one tag, which is the question a search asks of every
/// file it walks past.
pub fn has(path: &str, name: &str) -> bool {
    let wanted = name.trim();
    of(path).iter().any(|tag| tag == wanted)
}

fn parse(stored: &str) -> Vec<String> {
    let names: Vec<String> = stored.split(',').map(str::to_string).collect();
    normalize(&names)
}

/// Tags are a set: blanks are dropped, duplicates collapse, and the order is
/// the one anything showing them would have to sort into anyway.
pub(crate) fn normalize(names: &[String]) -> Vec<String> {
    let unique: BTreeSet<&str> = names.iter().map(|name| name.trim()).filter(|name| !name.is_empty()).collect();
    unique.into_iter().map(str::to_string).collect()
}

pub(crate) fn without(names: &[String], removed: &str) -> Vec<String> {
    let wanted = removed.trim();
    names.iter().filter(|name| name.as_str() != wanted).cloned().collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;

    /// A file in a fresh directory, or `None` when the filesystem does not
    /// carry extended attributes and there is nothing to test.
    fn taggable_file() -> Option<(tempfile::TempDir, String)> {
        let directory = tempfile::tempdir().unwrap();
        let path = write_file(directory.path(), "notes.md");
        if !supports_attributes(&path) {
            return None;
        }
        Some((directory, path))
    }

    fn supports_attributes(path: &str) -> bool {
        if attribute::write(path, b"probe").is_err() {
            return false;
        }
        attribute::clear(path).unwrap();
        true
    }

    fn write_file(root: &Path, name: &str) -> String {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "contents").unwrap();
        path.to_str().unwrap().to_string()
    }

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn tags_are_read_back_from_the_file() {
        let Some((_directory, path)) = taggable_file() else { return };

        set(&path, &names(&["work", "invoice"])).unwrap();

        assert_eq!(of(&path), ["invoice", "work"]);
    }

    #[test]
    fn add_keeps_the_tags_already_there() {
        let Some((_directory, path)) = taggable_file() else { return };

        add(&path, "work").unwrap();
        add(&path, "invoice").unwrap();

        assert_eq!(of(&path).len(), 2);
    }

    #[test]
    fn add_ignores_a_tag_that_is_already_there() {
        let Some((_directory, path)) = taggable_file() else { return };

        add(&path, "work").unwrap();
        add(&path, " work ").unwrap();

        assert_eq!(of(&path), ["work"]);
    }

    #[test]
    fn add_refuses_a_tag_with_no_name() {
        let Some((_directory, path)) = taggable_file() else { return };

        assert!(add(&path, "   ").is_err());
    }

    #[test]
    fn removing_the_last_tag_leaves_nothing_behind() {
        let Some((_directory, path)) = taggable_file() else { return };
        add(&path, "work").unwrap();

        remove(&path, "work").unwrap();

        assert!(of(&path).is_empty());
        assert!(!has(&path, "work"));
        assert!(attribute::read(&path).is_none());
    }

    #[test]
    fn removing_a_tag_that_is_not_there_is_not_a_failure() {
        let Some((_directory, path)) = taggable_file() else { return };

        assert!(remove(&path, "nothing").is_ok());
    }

    #[test]
    fn an_untagged_file_has_no_tags() {
        let Some((_directory, path)) = taggable_file() else { return };

        assert!(of(&path).is_empty());
    }

    #[test]
    fn search_finds_tagged_files_and_skips_hidden_directories() {
        let root = tempfile::tempdir().unwrap();
        let tagged = write_file(root.path(), "keep/notes.md");
        write_file(root.path(), "keep/other.md");
        let hidden = write_file(root.path(), ".cache/notes.md");
        if !supports_attributes(&tagged) {
            return;
        }
        add(&tagged, "work").unwrap();
        add(&hidden, "work").unwrap();

        let found = search(root.path().to_str().unwrap(), "work");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, tagged);
    }

    #[test]
    fn search_without_a_name_finds_nothing() {
        let root = tempfile::tempdir().unwrap();

        assert!(search(root.path().to_str().unwrap(), "  ").is_empty());
    }
}
