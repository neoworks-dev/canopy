//! Advisory file locks. Two locks shape the daemon's lifecycle: the daemon
//! holds one for as long as it runs, which makes a second daemon impossible,
//! and a launching client holds the other only while starting a daemon, which
//! makes simultaneous app launches spawn exactly one.

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

const ACQUIRE_POLL_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("lockfile: already held")]
    Busy,
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Held until dropped. Closing the file releases the flock, so a crashed
/// process never leaves a lock behind.
#[derive(Debug)]
pub struct Lock {
    _file: File,
}

/// Takes the lock without blocking, returning `LockError::Busy` when another
/// process holds it.
pub fn try_acquire(path: &Path) -> Result<Lock, LockError> {
    let file = OpenOptions::new().create(true).truncate(false).read(true).write(true).mode(0o600).open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Lock { _file: file }),
        Err(TryLockError::WouldBlock) => Err(LockError::Busy),
        Err(TryLockError::Error(error)) => Err(LockError::Io(error)),
    }
}

/// Polls until the lock is free or the timeout expires.
pub fn acquire_within(path: &Path, timeout: Duration) -> Result<Lock, LockError> {
    let deadline = Instant::now() + timeout;
    loop {
        match try_acquire(path) {
            Err(LockError::Busy) if Instant::now() < deadline => std::thread::sleep(ACQUIRE_POLL_INTERVAL),
            outcome => return outcome,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_lock_is_busy_until_released() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("daemon.lock");
        let lock = try_acquire(&path).unwrap();
        assert!(matches!(try_acquire(&path), Err(LockError::Busy)));
        drop(lock);
        assert!(try_acquire(&path).is_ok());
    }

    #[test]
    fn acquire_within_gives_up_after_the_timeout() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("spawn.lock");
        let _lock = try_acquire(&path).unwrap();
        let outcome = acquire_within(&path, Duration::from_millis(60));
        assert!(matches!(outcome, Err(LockError::Busy)));
    }
}
