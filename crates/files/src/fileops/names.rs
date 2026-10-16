use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

const MAX_NAME_ATTEMPTS: u32 = 10_000;

/// Rejects anything that cannot become a single directory entry. Callers check
/// this before touching the filesystem.
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::EmptyName);
    }
    if name.contains('/') || name.contains('\0') {
        return Err(Error::NameSeparator);
    }
    if name == "." || name == ".." {
        return Err(Error::ReservedName);
    }
    Ok(())
}

/// Resolves a name inside `directory`, failing if it is taken.
pub fn free_name(directory: &Path, name: &str) -> Result<PathBuf> {
    validate_name(name)?;
    let target = directory.join(name);
    if exists(&target) {
        return Err(Error::NameExists);
    }
    Ok(target)
}

/// Picks a free name in `directory`, starting from the one asked for. It is
/// what "New folder" needs when the obvious name is taken.
pub fn suggest_name(directory: &Path, name: &str) -> String {
    if !exists(&directory.join(name)) {
        return name.to_string();
    }
    match unique_name(directory, name) {
        Ok(suggestion) => suggestion,
        Err(_) => name.to_string(),
    }
}

/// Appends a counter until the name is free, which is what the "keep both"
/// conflict resolution needs.
pub(super) fn unique_name(directory: &Path, name: &str) -> Result<String> {
    let (stem, extension) = split_extension(name);
    for counter in 1..MAX_NAME_ATTEMPTS {
        let candidate = format!("{stem} ({counter}){extension}");
        if !exists(&directory.join(&candidate)) {
            return Ok(candidate);
        }
    }
    Err(Error::message(format!("no free name for {name:?} in {}", directory.display())))
}

/// Keeps compound suffixes such as `.tar.gz` together so the counter lands in
/// front of the whole suffix rather than inside it.
pub(super) fn split_extension(name: &str) -> (&str, &str) {
    let trimmed = name.strip_prefix('.').unwrap_or(name);
    let Some(dot_index) = trimmed.find('.') else {
        return (name, "");
    };
    let split_at = name.len() - trimmed.len() + dot_index;
    (&name[..split_at], &name[split_at..])
}

/// Whether anything is at `path`, without following a final symlink.
pub(super) fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Whether `path` is a directory, without following a final symlink.
pub(super) fn is_directory(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unusable_names_are_rejected() {
        assert!(matches!(validate_name(""), Err(Error::EmptyName)));
        assert!(matches!(validate_name("a/b"), Err(Error::NameSeparator)));
        assert!(matches!(validate_name("a\0b"), Err(Error::NameSeparator)));
        assert!(matches!(validate_name("."), Err(Error::ReservedName)));
        assert!(matches!(validate_name(".."), Err(Error::ReservedName)));
    }

    #[test]
    fn ordinary_names_are_accepted() {
        for name in ["report", ".hidden", "a.tar.gz"] {
            assert!(validate_name(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn an_existing_entry_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("taken.txt"), "x").unwrap();
        assert!(matches!(free_name(directory.path(), "taken.txt"), Err(Error::NameExists)));
    }

    #[test]
    fn unique_names_keep_compound_extensions() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("archive.tar.gz"), "x").unwrap();
        assert_eq!(unique_name(directory.path(), "archive.tar.gz").unwrap(), "archive (1).tar.gz");
    }

    #[test]
    fn unique_names_skip_names_already_taken() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("note.txt"), "x").unwrap();
        std::fs::write(directory.path().join("note (1).txt"), "x").unwrap();
        assert_eq!(unique_name(directory.path(), "note.txt").unwrap(), "note (2).txt");
    }

    #[test]
    fn dotfiles_keep_their_whole_name_as_the_stem() {
        assert_eq!(split_extension(".bashrc"), (".bashrc", ""));
        assert_eq!(split_extension(".config.json"), (".config", ".json"));
    }

    #[test]
    fn a_suggestion_is_the_name_itself_when_free() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(suggest_name(directory.path(), "New folder"), "New folder");
        std::fs::create_dir(directory.path().join("New folder")).unwrap();
        assert_eq!(suggest_name(directory.path(), "New folder"), "New folder (1)");
    }
}
