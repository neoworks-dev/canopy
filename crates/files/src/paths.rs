//! Lexical path helpers that behave like Go's `path/filepath`: `clean` folds
//! `.` and `..`, `base_name` and `directory_of` never fail, and `absolute`
//! makes a relative path absolute without touching the disk.

use std::path::{Component, Path, PathBuf};

/// Removes `.` components, resolves `..` against the components before it, and
/// collapses repeated separators. An empty result becomes `.`.
pub fn clean(path: &Path) -> PathBuf {
    let mut kept: Vec<Component> = Vec::new();
    for component in path.components() {
        push_cleaned(&mut kept, component);
    }
    if kept.is_empty() {
        return PathBuf::from(".");
    }
    kept.iter().collect()
}

fn push_cleaned<'a>(kept: &mut Vec<Component<'a>>, component: Component<'a>) {
    match component {
        Component::CurDir => {}
        Component::ParentDir => push_parent(kept),
        other => kept.push(other),
    }
}

fn push_parent<'a>(kept: &mut Vec<Component<'a>>) {
    match kept.last() {
        Some(Component::Normal(_)) => {
            kept.pop();
        }
        Some(Component::RootDir) => {}
        _ => kept.push(Component::ParentDir),
    }
}

/// `clean` for a slash-separated string such as an archive member name.
pub fn clean_slash_path(name: &str) -> String {
    clean(Path::new(name)).to_string_lossy().into_owned()
}

/// The path made absolute against the working directory and cleaned.
pub fn absolute(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(clean(path));
    }
    let working_directory = std::env::current_dir()?;
    Ok(clean(&working_directory.join(path)))
}

/// The last element of a path, or the whole path at the filesystem root.
pub fn base_name(path: &Path) -> String {
    match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => path.to_string_lossy().into_owned(),
    }
}

/// The suffix of the final path element from its last dot, lowercase and with
/// the dot, or empty when there is none. A dotfile such as `.mp3` counts as
/// having that extension, the way `filepath.Ext` reads it.
pub fn extension_of(path: &Path) -> String {
    let name = base_name(path);
    match name.rfind('.') {
        Some(position) => name[position..].to_lowercase(),
        None => String::new(),
    }
}

/// The directory holding a path: the root for the root, `.` for a bare name.
pub fn directory_of(path: &Path) -> PathBuf {
    match path.parent() {
        None => path.to_path_buf(),
        Some(parent) if parent.as_os_str().is_empty() => PathBuf::from("."),
        Some(parent) => parent.to_path_buf(),
    }
}

/// Whether `child` sits inside `parent` (or is `parent`). A directory copied
/// or moved into its own subtree is caught with this before it starts.
pub fn contains_path(parent: &Path, child: &Path) -> bool {
    if parent == child {
        return true;
    }
    let mut prefix = parent.to_string_lossy().trim_end_matches('/').to_string();
    prefix.push('/');
    child.to_string_lossy().starts_with(&prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_resolves_dots_like_go() {
        assert_eq!(clean(Path::new("/a/./b/../c")), PathBuf::from("/a/c"));
        assert_eq!(clean(Path::new("a/../..")), PathBuf::from(".."));
        assert_eq!(clean(Path::new("/..")), PathBuf::from("/"));
        assert_eq!(clean(Path::new("")), PathBuf::from("."));
    }

    #[test]
    fn extensions_include_the_dot_and_ignore_case() {
        assert_eq!(extension_of(Path::new("/music/A Song.FLAC")), ".flac");
        assert_eq!(extension_of(Path::new("/music/.mp3")), ".mp3");
        assert_eq!(extension_of(Path::new("/music/archive.tar.gz")), ".gz");
        assert_eq!(extension_of(Path::new("/music/no-extension")), "");
        assert_eq!(extension_of(Path::new("/music.d/no-extension")), "");
    }

    #[test]
    fn directory_of_never_fails() {
        assert_eq!(directory_of(Path::new("/")), PathBuf::from("/"));
        assert_eq!(directory_of(Path::new("name")), PathBuf::from("."));
        assert_eq!(directory_of(Path::new("/a/b")), PathBuf::from("/a"));
    }

    #[test]
    fn contains_path_detects_a_subtree() {
        assert!(contains_path(Path::new("/home/user/photos"), Path::new("/home/user/photos/2026")));
        assert!(!contains_path(Path::new("/home/user/photos"), Path::new("/home/user/photos-backup")));
    }
}
