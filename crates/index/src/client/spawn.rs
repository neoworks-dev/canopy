use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::{ClientError, ClientResult, Options};
use crate::lockfile;
use crate::proto;

/// Points at the daemon binary. Development builds and test harnesses use it; a
/// shipped application passes `Options::binary_path`.
pub const BINARY_PATH_ENVIRONMENT_VARIABLE: &str = "CANOPYD_BIN";

const DAEMON_BINARY_NAME: &str = "canopyd";
const DIAL_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Connects to a running daemon, starting one if there is none.
pub(super) fn dial_or_spawn(options: &Options) -> ClientResult<UnixStream> {
    proto::validate_socket_path(options.socket_path()).map_err(ClientError::Startup)?;
    match UnixStream::connect(options.socket_path()) {
        Ok(stream) => Ok(stream),
        Err(_) => spawn_and_dial(options),
    }
}

/// Serializes startup on the spawn lock, so several applications launching at
/// once produce one daemon rather than a race.
fn spawn_and_dial(options: &Options) -> ClientResult<UnixStream> {
    if let Some(directory) = options.socket_path().parent() {
        proto::ensure_private_directory(directory)?;
    }
    proto::ensure_private_directory(&proto::runtime_directory())?;
    let _lock = acquire_spawn_lock(&proto::spawn_lock_path(), options)?;

    // Whoever held the lock before us may already have started the daemon.
    if let Ok(stream) = UnixStream::connect(options.socket_path()) {
        return Ok(stream);
    }
    start_daemon(options)?;
    dial_until_ready(options)
}

fn acquire_spawn_lock(path: &Path, options: &Options) -> ClientResult<lockfile::Lock> {
    lockfile::acquire_within(path, options.spawn_timeout)
        .map_err(|error| ClientError::Startup(format!("waiting for daemon startup: {error}")))
}

fn dial_until_ready(options: &Options) -> ClientResult<UnixStream> {
    let deadline = Instant::now() + options.spawn_timeout;
    loop {
        match UnixStream::connect(options.socket_path()) {
            Ok(stream) => return Ok(stream),
            Err(error) if Instant::now() >= deadline => {
                return Err(ClientError::Startup(format!(
                    "daemon did not accept connections within {:?}: {error}",
                    options.spawn_timeout
                )));
            }
            Err(_) => thread::sleep(DIAL_POLL_INTERVAL),
        }
    }
}

/// Launches the daemon in its own session so it survives the exit of the
/// application that started it and never receives the terminal's signals.
fn start_daemon(options: &Options) -> ClientResult<()> {
    let binary_path = resolve_daemon_binary(options.binary_path.as_deref())?;
    let log_file = open_daemon_log()?;
    let mut command = Command::new(&binary_path);
    command
        .arg("-socket")
        .arg(options.socket_path())
        .stdin(Stdio::null())
        .stdout(log_file.try_clone()?)
        .stderr(log_file);
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let mut child = command
        .spawn()
        .map_err(|error| ClientError::Startup(format!("start {}: {error}", binary_path.display())))?;
    // Reap the child if it exits while this application is still running. Once
    // we exit, init adopts the daemon.
    thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Prefers an explicit path, then the environment override, then a binary
/// shipped next to the running application, and finally `PATH`.
fn resolve_daemon_binary(configured_path: Option<&Path>) -> ClientResult<PathBuf> {
    let found = daemon_binary_candidates(configured_path).into_iter().find(|candidate| is_executable_file(candidate));
    if let Some(candidate) = found {
        return Ok(candidate);
    }
    look_up_on_path(DAEMON_BINARY_NAME)
        .ok_or_else(|| ClientError::Startup(format!("executable file {DAEMON_BINARY_NAME:?} not found in $PATH")))
}

fn daemon_binary_candidates(configured_path: Option<&Path>) -> Vec<PathBuf> {
    let environment_path = std::env::var_os(BINARY_PATH_ENVIRONMENT_VARIABLE).map(PathBuf::from);
    [configured_path.map(Path::to_path_buf), environment_path, sibling_of_executable()]
        .into_iter()
        .flatten()
        .filter(|candidate| !candidate.as_os_str().is_empty())
        .collect()
}

fn sibling_of_executable() -> Option<PathBuf> {
    let executable_path = std::env::current_exe().ok()?;
    Some(executable_path.parent()?.join(DAEMON_BINARY_NAME))
}

fn look_up_on_path(binary_name: &str) -> Option<PathBuf> {
    let search_path = std::env::var_os("PATH")?;
    std::env::split_paths(&search_path)
        .map(|directory| directory.join(binary_name))
        .find(|candidate| is_executable_file(candidate))
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    !metadata.is_dir() && metadata.mode() & 0o111 != 0
}

fn open_daemon_log() -> io::Result<File> {
    proto::ensure_private_directory(&proto::data_directory())?;
    OpenOptions::new().create(true).append(true).mode(0o600).open(proto::log_path())
}
