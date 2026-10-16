use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// The portable floor for the kernel's `sun_path` field (macOS allows 104 bytes,
/// Linux 108). Exceeding it fails with a bare "invalid argument", so the limit
/// is checked explicitly instead.
pub const MAXIMUM_SOCKET_PATH_LENGTH: usize = 103;

const APPLICATION_DIRECTORY_NAME: &str = "neoworks";

/// Overrides every runtime path at once, which is what test harnesses and
/// sandboxed runs use to stay isolated.
pub const RUNTIME_DIRECTORY_ENVIRONMENT_VARIABLE: &str = "NEOWORKS_RUNTIME_DIR";

/// Overrides where the index database lives.
pub const DATA_DIRECTORY_ENVIRONMENT_VARIABLE: &str = "NEOWORKS_DATA_DIR";

fn environment_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from)
}

/// Holds the socket and lock files. It is deliberately on a tmpfs so a reboot
/// cannot leave a stale socket behind.
pub fn runtime_directory() -> PathBuf {
    if let Some(override_path) = environment_path(RUNTIME_DIRECTORY_ENVIRONMENT_VARIABLE) {
        return override_path;
    }
    if let Some(runtime_home) = environment_path("XDG_RUNTIME_DIR") {
        return runtime_home.join(APPLICATION_DIRECTORY_NAME);
    }
    let user_id = unsafe { libc::getuid() };
    std::env::temp_dir().join(format!("{APPLICATION_DIRECTORY_NAME}-{user_id}"))
}

pub fn socket_path() -> PathBuf {
    runtime_directory().join("daemon.sock")
}

/// A path the kernel cannot bind. Both sides check it, because a client that
/// cannot dial would otherwise spawn a daemon that cannot listen.
pub fn validate_socket_path(socket_path: &Path) -> Result<(), String> {
    let length = socket_path.as_os_str().len();
    if length <= MAXIMUM_SOCKET_PATH_LENGTH {
        return Ok(());
    }
    Err(format!(
        "socket path {:?} is {} bytes, over the {} byte kernel limit; point {} at a shorter directory",
        socket_path, length, MAXIMUM_SOCKET_PATH_LENGTH, RUNTIME_DIRECTORY_ENVIRONMENT_VARIABLE
    ))
}

/// Held by the daemon for its entire lifetime, which is what makes a second
/// daemon impossible.
pub fn daemon_lock_path() -> PathBuf {
    runtime_directory().join("daemon.lock")
}

/// Held by a client only while it starts a daemon, so simultaneous launches of
/// several apps spawn exactly one.
pub fn spawn_lock_path() -> PathBuf {
    runtime_directory().join("spawn.lock")
}

/// Holds durable state: the index database and the daemon log.
pub fn data_directory() -> PathBuf {
    if let Some(override_path) = environment_path(DATA_DIRECTORY_ENVIRONMENT_VARIABLE) {
        return override_path;
    }
    let data_home = environment_path("XDG_DATA_HOME").unwrap_or_else(default_data_home);
    data_home.join(APPLICATION_DIRECTORY_NAME)
}

pub fn index_database_path() -> PathBuf {
    data_directory().join("index.db")
}

pub fn log_path() -> PathBuf {
    data_directory().join("daemon.log")
}

/// Creates a directory only the owning user can enter. Existing directories are
/// tightened too, because a permissive mode on the runtime directory would
/// expose the socket.
pub fn ensure_private_directory(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

fn default_data_home() -> PathBuf {
    match std::env::home_dir() {
        Some(home_directory) => home_directory.join(".local").join("share"),
        None => std::env::temp_dir().join("neoworks-data"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::EnvironmentGuard;

    #[test]
    fn validate_socket_path_accepts_a_path_the_kernel_can_bind() {
        assert!(validate_socket_path(Path::new("/run/user/1000/neoworks/daemon.sock")).is_ok());
    }

    #[test]
    fn validate_socket_path_rejects_an_overlong_path() {
        let overlong_path = Path::new("/tmp").join("d".repeat(MAXIMUM_SOCKET_PATH_LENGTH)).join("daemon.sock");
        let message = validate_socket_path(&overlong_path).unwrap_err();
        assert!(message.contains(RUNTIME_DIRECTORY_ENVIRONMENT_VARIABLE), "{message}");
    }

    #[test]
    fn runtime_directory_honours_the_override() {
        let mut environment = EnvironmentGuard::acquire();
        environment.set(RUNTIME_DIRECTORY_ENVIRONMENT_VARIABLE, "/tmp/neoworks-test-runtime");
        assert_eq!(runtime_directory(), Path::new("/tmp/neoworks-test-runtime"));
        assert_eq!(socket_path().parent().unwrap(), Path::new("/tmp/neoworks-test-runtime"));
    }

    #[test]
    fn data_directory_honours_the_override() {
        let mut environment = EnvironmentGuard::acquire();
        environment.set(DATA_DIRECTORY_ENVIRONMENT_VARIABLE, "/tmp/neoworks-test-data");
        assert_eq!(index_database_path().parent().unwrap(), Path::new("/tmp/neoworks-test-data"));
    }
}
