use std::sync::{Mutex, MutexGuard, PoisonError};

static ENVIRONMENT_LOCK: Mutex<()> = Mutex::new(());

/// Tests that change process environment variables hold this for their whole
/// run, since the variables are shared by every test thread.
pub struct EnvironmentGuard {
    _lock: MutexGuard<'static, ()>,
    restorations: Vec<(String, Option<std::ffi::OsString>)>,
}

impl EnvironmentGuard {
    pub fn acquire() -> Self {
        let lock = ENVIRONMENT_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        EnvironmentGuard { _lock: lock, restorations: Vec::new() }
    }

    pub fn set(&mut self, name: &str, value: impl AsRef<std::ffi::OsStr>) {
        self.restorations.push((name.to_string(), std::env::var_os(name)));
        unsafe { std::env::set_var(name, value) };
    }
}

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        for (name, previous) in self.restorations.drain(..).rev() {
            match previous {
                Some(value) => unsafe { std::env::set_var(&name, value) },
                None => unsafe { std::env::remove_var(&name) },
            }
        }
    }
}
