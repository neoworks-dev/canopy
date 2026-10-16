use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::filesystem::path::home_directory;
use crate::paths;

/// A shortcut to a well-known directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    pub path: String,
    /// The icon the sidebar draws: `home`, `desktop`, `downloads`, `documents`,
    /// `music`, `pictures`, `videos` or `root`.
    pub kind: String,
}

/// One standard directory: the XDG key that configures it, the icon kind, and
/// the English name used when the system has no XDG config.
struct StandardPlace {
    xdg_key: &'static str,
    kind: &'static str,
    fallback_name: &'static str,
}

/// The sidebar order.
const STANDARD_PLACES: [StandardPlace; 6] = [
    StandardPlace { xdg_key: "XDG_DESKTOP_DIR", kind: "desktop", fallback_name: "Desktop" },
    StandardPlace { xdg_key: "XDG_DOWNLOAD_DIR", kind: "downloads", fallback_name: "Downloads" },
    StandardPlace { xdg_key: "XDG_DOCUMENTS_DIR", kind: "documents", fallback_name: "Documents" },
    StandardPlace { xdg_key: "XDG_MUSIC_DIR", kind: "music", fallback_name: "Music" },
    StandardPlace { xdg_key: "XDG_PICTURES_DIR", kind: "pictures", fallback_name: "Pictures" },
    StandardPlace { xdg_key: "XDG_VIDEOS_DIR", kind: "videos", fallback_name: "Videos" },
];

/// Lists the well-known directories that exist on this machine, from the home
/// directory down to the filesystem root.
pub fn places() -> Vec<Place> {
    let Ok(home) = home_directory() else {
        return vec![root_place()];
    };
    let mut found =
        vec![Place { name: "Home".to_string(), path: home.to_string_lossy().into_owned(), kind: "home".to_string() }];
    found.extend(user_directories(&home));
    found.push(root_place());
    found
}

fn root_place() -> Place {
    Place { name: "Filesystem".to_string(), path: "/".to_string(), kind: "root".to_string() }
}

fn user_directories(home_directory: &Path) -> Vec<Place> {
    let configured = read_user_directory_config(home_directory);
    let mut found = Vec::new();
    for standard in &STANDARD_PLACES {
        let path = path_for(standard, &configured, home_directory);
        if !path.is_dir() {
            continue;
        }
        found.push(Place {
            name: paths::base_name(&path),
            path: path.to_string_lossy().into_owned(),
            kind: standard.kind.to_string(),
        });
    }
    found
}

fn path_for(standard: &StandardPlace, configured: &[(String, PathBuf)], home_directory: &Path) -> PathBuf {
    let configured_path = configured.iter().rev().find(|(key, _)| key == standard.xdg_key);
    match configured_path {
        Some((_, path)) if !path.as_os_str().is_empty() => path.clone(),
        _ => home_directory.join(standard.fallback_name),
    }
}

/// Parses `~/.config/user-dirs.dirs`, the XDG file holding the localized paths
/// of the standard directories. A missing file yields nothing, and callers fall
/// back to the English defaults.
fn read_user_directory_config(home_directory: &Path) -> Vec<(String, PathBuf)> {
    let Ok(contents) = fs::read_to_string(home_directory.join(".config").join("user-dirs.dirs")) else {
        return Vec::new();
    };
    let mut directories = Vec::new();
    for line in contents.lines() {
        if let Some(assignment) = parse_user_directory_line(line, home_directory) {
            directories.push(assignment);
        }
    }
    directories
}

/// Reads one `XDG_xxx_DIR="…"` assignment. Values are either absolute or
/// written relative to `$HOME`. An unusable line yields nothing.
fn parse_user_directory_line(line: &str, home_directory: &Path) -> Option<(String, PathBuf)> {
    let (key, raw_value) = line.trim().split_once('=')?;
    if !key.starts_with("XDG_") {
        return None;
    }
    let value = raw_value.trim_matches('"');
    if let Some(relative) = value.strip_prefix("$HOME/") {
        return Some((key.to_string(), paths::clean(&home_directory.join(relative))));
    }
    if Path::new(value).is_absolute() {
        return Some((key.to_string(), PathBuf::from(value)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_relative_values_are_expanded() {
        let (key, path) =
            parse_user_directory_line(r#"XDG_MUSIC_DIR="$HOME/Musik""#, Path::new("/home/tester")).unwrap();
        assert_eq!(key, "XDG_MUSIC_DIR");
        assert_eq!(path, PathBuf::from("/home/tester/Musik"));
    }

    #[test]
    fn absolute_values_are_kept() {
        let (key, path) =
            parse_user_directory_line(r#"XDG_VIDEOS_DIR="/mnt/media/videos""#, Path::new("/home/tester")).unwrap();
        assert_eq!(key, "XDG_VIDEOS_DIR");
        assert_eq!(path, PathBuf::from("/mnt/media/videos"));
    }

    #[test]
    fn lines_that_are_not_assignments_are_rejected() {
        let unusable = [
            "# This file is written by xdg-user-dirs-update",
            "",
            r#"XDG_DESKTOP_DIR="relative/desktop""#,
            r#"SOME_OTHER_DIR="$HOME/Desktop""#,
        ];
        for line in unusable {
            assert!(parse_user_directory_line(line, Path::new("/home/tester")).is_none(), "{line}");
        }
    }

    #[test]
    fn configured_names_win_over_the_english_ones() {
        let home = tempfile::tempdir().unwrap();
        fs::create_dir(home.path().join(".config")).unwrap();
        fs::write(home.path().join(".config/user-dirs.dirs"), r#"XDG_DOCUMENTS_DIR="$HOME/Dokumente""#).unwrap();
        fs::create_dir(home.path().join("Dokumente")).unwrap();
        fs::create_dir(home.path().join("Documents")).unwrap();

        let found = user_directories(home.path());

        let documents = found.iter().find(|place| place.kind == "documents").unwrap();
        assert_eq!(documents.name, "Dokumente");
    }

    #[test]
    fn missing_directories_are_skipped() {
        let home = tempfile::tempdir().unwrap();
        fs::create_dir(home.path().join("Downloads")).unwrap();

        let found = user_directories(home.path());

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, "downloads");
    }

    #[test]
    fn places_start_at_home_and_end_at_root() {
        let found = places();
        assert!(found.len() >= 2);
        assert_eq!(found[0].kind, "home");
        let last = found.last().unwrap();
        assert_eq!((last.kind.as_str(), last.path.as_str()), ("root", "/"));
    }
}
