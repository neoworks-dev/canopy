use std::path::PathBuf;

const APPLICATION_DIRECTORY: &str = "canopy";

/// Where the named config file belongs, honouring `XDG_CONFIG_HOME` and falling
/// back to `~/.config`.
pub fn config_path(file_name: &str) -> Option<PathBuf> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(config_home).join(APPLICATION_DIRECTORY).join(file_name));
    }
    let home_directory = std::env::home_dir()?;
    Some(home_directory.join(".config").join(APPLICATION_DIRECTORY).join(file_name))
}
