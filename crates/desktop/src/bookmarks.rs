//! The folders a user has pinned, in the file the GTK file choosers and every
//! desktop file manager already share. Pinning a folder here puts it in their
//! sidebars too.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::atomic_write::write_atomically;
use crate::file_uri;
use crate::path_text::base_name;

/// One pinned folder. A name that is not the folder's own is what the file
/// calls a label, and it is kept as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub path: String,
}

/// The pinned folders. A missing file means nothing is pinned yet.
pub fn list() -> Vec<Bookmark> {
    match bookmarks_path() {
        Some(path) => list_from(&path),
        None => Vec::new(),
    }
}

/// Pins a folder; one that is already pinned is left alone.
pub fn add(path: &str, name: &str) -> io::Result<()> {
    let file = require_bookmarks_path()?;
    add_to(&file, path, name)
}

pub fn remove(path: &str) -> io::Result<()> {
    let file = require_bookmarks_path()?;
    remove_from(&file, path)
}

fn list_from(file: &Path) -> Vec<Bookmark> {
    match fs::read(file) {
        Ok(contents) => parse(&String::from_utf8_lossy(&contents)),
        Err(_) => Vec::new(),
    }
}

fn add_to(file: &Path, path: &str, name: &str) -> io::Result<()> {
    let mut pinned = list_from(file);
    if index_of(&pinned, path).is_some() {
        return Ok(());
    }
    pinned.push(Bookmark { name: name.to_string(), path: path.to_string() });
    write(file, &pinned)
}

fn remove_from(file: &Path, path: &str) -> io::Result<()> {
    let mut pinned = list_from(file);
    let Some(index) = index_of(&pinned, path) else {
        return Ok(());
    };
    pinned.remove(index);
    write(file, &pinned)
}

fn index_of(pinned: &[Bookmark], path: &str) -> Option<usize> {
    pinned.iter().position(|bookmark| bookmark.path == path)
}

fn parse(contents: &str) -> Vec<Bookmark> {
    contents.split('\n').filter_map(parse_line).collect()
}

/// A line is a file URI and an optional label after a space. Anything else in
/// the file, such as a remote URI or a comment, is left alone rather than
/// shown as a folder that cannot be opened.
fn parse_line(line: &str) -> Option<Bookmark> {
    let trimmed = line.trim();
    let (uri, label) = match trimmed.split_once(' ') {
        Some(parts) => parts,
        None => (trimmed, ""),
    };
    if !uri.starts_with("file://") {
        return None;
    }
    let path = file_uri::to_path(uri)?;
    let name = name_for(&path, label);
    Some(Bookmark { name, path })
}

fn name_for(path: &str, label: &str) -> String {
    if label.is_empty() {
        return base_name(path);
    }
    label.to_string()
}

fn write(file: &Path, pinned: &[Bookmark]) -> io::Result<()> {
    let directory = file.parent().unwrap_or_else(|| Path::new("."));
    create_private_directory(directory)?;
    write_atomically(file, ".bookmarks-", format(pinned).as_bytes(), 0o600)
}

fn create_private_directory(directory: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().recursive(true).mode(0o700).create(directory)
}

fn format(pinned: &[Bookmark]) -> String {
    let lines: Vec<String> = pinned.iter().map(format_line).collect();
    lines.join("\n") + "\n"
}

/// The label is only written when it differs from the folder's own name, which
/// is how the other file managers write it.
fn format_line(bookmark: &Bookmark) -> String {
    let uri = file_uri::from_path(&bookmark.path);
    if bookmark.name == base_name(&bookmark.path) {
        return uri;
    }
    format!("{uri} {}", bookmark.name)
}

fn require_bookmarks_path() -> io::Result<PathBuf> {
    match bookmarks_path() {
        Some(path) => Ok(path),
        None => Err(io::Error::other("the home directory cannot be determined")),
    }
}

fn bookmarks_path() -> Option<PathBuf> {
    Some(config_home()?.join("gtk-3.0").join("bookmarks"))
}

fn config_home() -> Option<PathBuf> {
    if let Some(configured) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(configured));
    }
    Some(std::env::home_dir()?.join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bookmarks_file(contents: &str) -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("gtk-3.0").join("bookmarks");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, contents).unwrap();
        (directory, file)
    }

    #[test]
    fn list_reads_pinned_folders() {
        let (_directory, file) = bookmarks_file("file:///home/moritz/Projects\nfile:///srv/share Shared drive\n");

        let pinned = list_from(&file);

        assert_eq!(pinned.len(), 2);
        assert_eq!(pinned[0], Bookmark { name: "Projects".into(), path: "/home/moritz/Projects".into() });
        assert_eq!(pinned[1].name, "Shared drive");
    }

    #[test]
    fn list_leaves_out_what_is_not_a_local_folder() {
        let (_directory, file) = bookmarks_file("smb://server/share Share\n\nfile:///home/moritz/Notes\n");

        let pinned = list_from(&file);

        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].path, "/home/moritz/Notes");
    }

    #[test]
    fn add_pins_a_folder_once() {
        let (_directory, file) = bookmarks_file("");

        add_to(&file, "/home/moritz/Projects", "Projects").unwrap();
        add_to(&file, "/home/moritz/Projects", "Projects").unwrap();

        assert_eq!(list_from(&file).len(), 1);
    }

    #[test]
    fn remove_unpins_a_folder_and_leaves_the_rest() {
        let (_directory, file) = bookmarks_file("file:///a\nfile:///b\nfile:///c\n");

        remove_from(&file, "/b").unwrap();

        let paths: Vec<String> = list_from(&file).into_iter().map(|bookmark| bookmark.path).collect();
        assert_eq!(paths, ["/a", "/c"]);
    }

    #[test]
    fn removing_something_that_is_not_pinned_changes_nothing() {
        let (_directory, file) = bookmarks_file("file:///a\n");

        remove_from(&file, "/elsewhere").unwrap();

        assert_eq!(list_from(&file).len(), 1);
    }

    #[test]
    fn added_folders_survive_a_reread() {
        let (_directory, file) = bookmarks_file("");

        add_to(&file, "/srv/share", "Shared drive").unwrap();
        add_to(&file, "/home/moritz/Notes", "Notes").unwrap();

        let pinned = list_from(&file);
        assert_eq!(pinned[0].name, "Shared drive");
        assert_eq!(pinned[1].name, "Notes");
    }

    #[test]
    fn a_label_is_only_written_when_it_differs_from_the_folder_name() {
        let (_directory, file) = bookmarks_file("");

        add_to(&file, "/srv/share", "Shared drive").unwrap();
        add_to(&file, "/home/moritz/Notes", "Notes").unwrap();

        let contents = fs::read_to_string(&file).unwrap();
        assert_eq!(contents, "file:///srv/share Shared drive\nfile:///home/moritz/Notes\n");
    }

    #[test]
    fn adding_creates_the_file_and_its_directory() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("gtk-3.0").join("bookmarks");

        add_to(&file, "/a b", "a b").unwrap();

        assert_eq!(fs::read_to_string(&file).unwrap(), "file:///a%20b\n");
    }
}
