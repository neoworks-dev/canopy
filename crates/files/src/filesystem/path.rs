use std::io;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::paths;

/// Turns user-supplied input into an absolute path. An empty path means the
/// home directory, and a leading `~` is expanded.
pub fn resolve_path(requested_path: &str) -> Result<PathBuf> {
    if requested_path.is_empty() {
        return Ok(home_directory()?);
    }
    let expanded_path = expand_home(requested_path)?;
    Ok(paths::absolute(&expanded_path)?)
}

/// The directory containing the given path, or an empty string at the
/// filesystem root where there is nowhere left to go up.
pub fn parent_path(absolute_path: &Path) -> String {
    let parent = paths::directory_of(absolute_path);
    if parent == absolute_path {
        return String::new();
    }
    parent.to_string_lossy().into_owned()
}

/// Whether an entry name follows the Unix dotfile convention.
pub fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

pub(crate) fn home_directory() -> io::Result<PathBuf> {
    match std::env::home_dir() {
        Some(directory) if !directory.as_os_str().is_empty() => Ok(directory),
        _ => Err(io::Error::other("$HOME is not defined")),
    }
}

fn expand_home(path: &str) -> io::Result<PathBuf> {
    let Some(remainder) = path.strip_prefix('~') else {
        return Ok(PathBuf::from(path));
    };
    let home = home_directory()?;
    Ok(paths::clean(&PathBuf::from(format!("{}/{}", home.display(), remainder))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_path_resolves_to_home() {
        let Ok(home) = home_directory() else { return };
        assert_eq!(resolve_path("").unwrap(), home);
    }

    #[test]
    fn tilde_is_expanded() {
        let Ok(home) = home_directory() else { return };
        assert_eq!(resolve_path("~/Documents").unwrap(), home.join("Documents"));
    }

    #[test]
    fn relative_paths_become_absolute() {
        assert!(resolve_path("./somewhere").unwrap().is_absolute());
    }

    #[test]
    fn parent_path_stops_at_root() {
        assert_eq!(parent_path(Path::new("/")), "");
        assert_eq!(parent_path(Path::new("/home/moritz")), "/home");
    }

    #[test]
    fn dotfiles_are_hidden() {
        assert!(is_hidden(".config"));
        assert!(!is_hidden("config"));
    }
}
