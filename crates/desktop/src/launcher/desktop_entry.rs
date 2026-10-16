use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::environment::Environment;

/// One desktop entry that can open a file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub exec: String,
}

/// The subset of a `.desktop` file that matters here.
#[derive(Debug, Clone, Default)]
pub(crate) struct DesktopEntry {
    pub name: String,
    pub icon: String,
    pub exec: String,
    pub terminal: bool,
    pub no_display: bool,
    pub is_application: bool,
}

impl Environment {
    /// The desktop ids associated with a MIME type, from both the
    /// `mimeapps.list` preferences and the `mimeinfo.cache` indexes, in that
    /// order and without repeats.
    pub(crate) fn desktop_ids_for(&self, mime_type: &str) -> Vec<String> {
        let mut identifiers: Vec<String> = Vec::new();
        for directory in self.application_directories() {
            identifiers.extend(associations_in(&directory, mime_type));
        }
        remove_duplicates(identifiers)
    }

    /// Resolves a desktop id to a file. Ids may encode a subdirectory with
    /// dashes, so `kde-okular.desktop` can live at `kde/okular.desktop`.
    pub(crate) fn find_desktop_file(&self, desktop_id: &str) -> Option<PathBuf> {
        self.application_directories()
            .iter()
            .find_map(|directory| desktop_file_in(directory, desktop_id))
    }

    /// The usable entry for a desktop id, if one is installed.
    pub(crate) fn load_entry(&self, desktop_id: &str) -> Option<DesktopEntry> {
        let desktop_path = self.find_desktop_file(desktop_id)?;
        read_desktop_entry(&desktop_path)
    }
}

fn associations_in(directory: &Path, mime_type: &str) -> Vec<String> {
    let mut identifiers = read_association_list(&directory.join("mimeapps.list"), mime_type);
    identifiers.extend(read_association_list(&directory.join("mimeinfo.cache"), mime_type));
    identifiers
}

/// Pulls the desktop ids a key file lists for one MIME type. Both
/// `mimeapps.list` and `mimeinfo.cache` use the same `type=a.desktop;b.desktop;`
/// shape, so one reader covers them.
fn read_association_list(path: &Path, mime_type: &str) -> Vec<String> {
    let Ok(contents) = fs::read(path) else {
        return Vec::new();
    };
    String::from_utf8_lossy(&contents)
        .split('\n')
        .flat_map(|line| associations_on_line(line, mime_type))
        .collect()
}

pub(crate) fn associations_on_line(line: &str, mime_type: &str) -> Vec<String> {
    let prefix = format!("{mime_type}=");
    let Some(value) = line.trim().strip_prefix(&prefix) else {
        return Vec::new();
    };
    value
        .split(';')
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .map(str::to_string)
        .collect()
}

fn remove_duplicates(values: Vec<String>) -> Vec<String> {
    let mut unique: Vec<String> = Vec::new();
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

fn desktop_file_in(directory: &Path, desktop_id: &str) -> Option<PathBuf> {
    desktop_id_variants(desktop_id)
        .iter()
        .map(|relative_path| directory.join(relative_path))
        .find(|candidate| is_regular_file(candidate))
}

pub(crate) fn desktop_id_variants(desktop_id: &str) -> Vec<String> {
    let mut variants = vec![desktop_id.to_string()];
    let mut remaining = desktop_id.to_string();
    while remaining.contains('-') {
        remaining = remaining.replacen('-', "/", 1);
        variants.push(remaining.clone());
    }
    variants
}

fn is_regular_file(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.is_file(),
        Err(_) => false,
    }
}

/// Parses the `[Desktop Entry]` group, which is all that is needed to show an
/// entry and launch it.
pub(crate) fn read_desktop_entry(path: &Path) -> Option<DesktopEntry> {
    let contents = fs::read(path).ok()?;
    let mut entry = DesktopEntry::default();
    let mut in_group = false;
    for line in String::from_utf8_lossy(&contents).split('\n') {
        in_group = apply_desktop_line(&mut entry, line.trim(), in_group);
    }
    if entry.is_application && !entry.exec.is_empty() {
        return Some(entry);
    }
    None
}

/// Reads one line and reports whether the parser is still inside the
/// `[Desktop Entry]` group afterwards.
fn apply_desktop_line(entry: &mut DesktopEntry, line: &str, in_group: bool) -> bool {
    if line.starts_with('[') {
        return line == "[Desktop Entry]";
    }
    if !in_group {
        return false;
    }
    if let Some((key, value)) = line.split_once('=') {
        set_desktop_field(entry, key.trim(), value.trim());
    }
    true
}

fn set_desktop_field(entry: &mut DesktopEntry, key: &str, value: &str) {
    match key {
        "Name" => entry.name = value.to_string(),
        "Icon" => entry.icon = value.to_string(),
        "Exec" => entry.exec = value.to_string(),
        "Terminal" => entry.terminal = value == "true",
        "NoDisplay" => entry.no_display = value == "true",
        "Type" => entry.is_application = value == "Application",
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_the_main_group() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("viewer.desktop");
        let contents = "[Desktop Entry]\nType=Application\nName=Viewer\nName[de]=Betrachter\nIcon=viewer\nExec=viewer %f\n\n[Desktop Action New]\nName=New Window\nExec=viewer --new\n";
        fs::write(&path, contents).unwrap();

        let entry = read_desktop_entry(&path).expect("a Type=Application entry with an Exec must be usable");

        assert_eq!(entry.name, "Viewer");
        assert_eq!(entry.exec, "viewer %f");
    }

    #[test]
    fn rejects_a_non_application() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("link.desktop");
        fs::write(&path, "[Desktop Entry]\nType=Link\nName=Somewhere\nURL=https://example.org\n").unwrap();

        assert!(read_desktop_entry(&path).is_none());
    }

    #[test]
    fn id_variants_cover_subdirectories() {
        assert_eq!(desktop_id_variants("kde-okular.desktop"), ["kde-okular.desktop", "kde/okular.desktop"]);
    }

    #[test]
    fn associations_on_a_line_read_one_mime_type() {
        let identifiers = associations_on_line("image/png=viewer.desktop;editor.desktop;", "image/png");

        assert_eq!(identifiers, ["viewer.desktop", "editor.desktop"]);
        assert!(associations_on_line("image/jpeg=other.desktop;", "image/png").is_empty());
    }
}
