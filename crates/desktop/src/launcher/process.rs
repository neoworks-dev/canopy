use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

use super::environment::Environment;

impl Environment {
    /// Starts the process in its own session so it outlives the explorer, and
    /// reaps it in the background rather than leaving a zombie. A working
    /// directory of `None` leaves the explorer's own.
    pub(crate) fn run_detached(&self, directory: Option<&Path>, name: &str, arguments: &[String]) -> io::Result<()> {
        let Some(executable) = self.find_executable(name) else {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("{name} is not installed")));
        };
        let mut command = Command::new(executable);
        command.args(arguments).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if let Some(directory) = directory {
            command.current_dir(directory);
        }
        // SAFETY: setsid is async-signal-safe and touches no memory of the child.
        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        let mut child = command.spawn()?;
        thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }
}
