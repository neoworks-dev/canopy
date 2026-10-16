//! Runs a helper program (ffmpeg, ffprobe, pdftoppm) and collects what it
//! writes to standard output, giving up on it after a time limit.

use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Why a helper produced nothing.
#[derive(Debug)]
pub enum HelperFailure {
    /// The program is not installed, which is a missing feature rather than a fault.
    NotInstalled,
    /// It started but failed, timed out, or printed nothing.
    Failed,
}

/// Runs `program` with `arguments` and returns its standard output. A program
/// that is still running after `time_limit` is killed.
pub fn run_helper(program: &str, arguments: &[String], time_limit: Duration) -> Result<Vec<u8>, HelperFailure> {
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => HelperFailure::NotInstalled,
            _ => HelperFailure::Failed,
        })?;
    let Some(mut output_pipe) = child.stdout.take() else {
        return Err(HelperFailure::Failed);
    };
    let reader = thread::spawn(move || {
        let mut collected = Vec::new();
        let _ = output_pipe.read_to_end(&mut collected);
        collected
    });
    let succeeded = wait_for_exit(&mut child, time_limit);
    let collected = reader.join().unwrap_or_default();
    if !succeeded {
        return Err(HelperFailure::Failed);
    }
    Ok(collected)
}

fn wait_for_exit(child: &mut std::process::Child, time_limit: Duration) -> bool {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {}
            Err(_) => return false,
        }
        if started.elapsed() >= time_limit {
            let _ = child.kill();
            let _ = child.wait();
            return false;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_collected() {
        let output = run_helper("echo", &["hello".to_string()], Duration::from_secs(5)).unwrap();
        assert_eq!(output, b"hello\n");
    }

    #[test]
    fn a_missing_program_is_reported_as_not_installed() {
        let result = run_helper("canopy-no-such-program", &[], Duration::from_secs(5));
        assert!(matches!(result, Err(HelperFailure::NotInstalled)));
    }

    #[test]
    fn a_slow_program_is_killed() {
        let result = run_helper("sleep", &["5".to_string()], Duration::from_millis(100));
        assert!(matches!(result, Err(HelperFailure::Failed)));
    }
}
