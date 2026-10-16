use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use canopy_model::config_path;

const OVERRIDES_FILE_NAME: &str = "open-with.json";

/// Where the launcher looks things up: the XDG data directories, the file that
/// remembers per-type defaults, the executable search path and `$TERMINAL`.
/// Built from the process environment, or by hand in tests.
#[derive(Debug, Clone, Default)]
pub(crate) struct Environment {
    pub data_directories: Vec<PathBuf>,
    pub overrides_path: Option<PathBuf>,
    pub executable_directories: Vec<PathBuf>,
    pub terminal: String,
}

impl Environment {
    pub(crate) fn from_process() -> Environment {
        Environment {
            data_directories: data_directories(),
            overrides_path: config_path(OVERRIDES_FILE_NAME),
            executable_directories: executable_directories(),
            terminal: env::var("TERMINAL").unwrap_or_default(),
        }
    }

    /// Every `applications/` directory on the XDG search path, most specific
    /// first, so a user override shadows a system entry.
    pub(crate) fn application_directories(&self) -> Vec<PathBuf> {
        self.data_directories.iter().map(|directory| directory.join("applications")).collect()
    }

    /// The path of an installed command, searched the way a shell would.
    pub(crate) fn find_executable(&self, name: &str) -> Option<PathBuf> {
        if name.is_empty() {
            return None;
        }
        if name.contains('/') {
            return executable_file(Path::new(name));
        }
        self.executable_directories
            .iter()
            .map(|directory| directory.join(name))
            .find_map(|candidate| executable_file(&candidate))
    }

    pub(crate) fn is_installed(&self, name: &str) -> bool {
        self.find_executable(name).is_some()
    }
}

fn executable_file(path: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return None;
    }
    Some(path.to_path_buf())
}

fn data_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(data_home) = data_home() {
        directories.push(data_home);
    }
    directories.extend(system_data_directories());
    directories
}

fn data_home() -> Option<PathBuf> {
    if let Some(configured) = non_empty_variable("XDG_DATA_HOME") {
        return Some(PathBuf::from(configured));
    }
    Some(env::home_dir()?.join(".local").join("share"))
}

fn system_data_directories() -> Vec<PathBuf> {
    let raw = match non_empty_variable("XDG_DATA_DIRS") {
        Some(configured) => configured.to_string_lossy().into_owned(),
        None => "/usr/local/share:/usr/share".to_string(),
    };
    raw.split(':').map(PathBuf::from).collect()
}

fn executable_directories() -> Vec<PathBuf> {
    match env::var_os("PATH") {
        Some(path) => env::split_paths(&path).collect(),
        None => Vec::new(),
    }
}

fn non_empty_variable(name: &str) -> Option<OsString> {
    env::var_os(name).filter(|value| !value.is_empty())
}
