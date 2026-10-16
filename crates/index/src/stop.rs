use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// A cancellation flag that sleeping threads can wait on. Clones share one flag.
#[derive(Clone, Default)]
pub struct StopSignal {
    state: Arc<(Mutex<bool>, Condvar)>,
}

impl StopSignal {
    pub fn new() -> Self {
        StopSignal::default()
    }

    pub fn stop(&self) {
        let (flag, condition) = &*self.state;
        *flag.lock().unwrap_or_else(PoisonError::into_inner) = true;
        condition.notify_all();
    }

    pub fn is_stopped(&self) -> bool {
        *self.state.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Blocks until the signal is raised.
    pub fn wait(&self) {
        let (flag, condition) = &*self.state;
        let mut stopped = flag.lock().unwrap_or_else(PoisonError::into_inner);
        while !*stopped {
            stopped = condition.wait(stopped).unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Sleeps up to `duration`, returning true as soon as the signal is raised.
    pub fn wait_timeout(&self, duration: Duration) -> bool {
        let deadline = Instant::now() + duration;
        let (flag, condition) = &*self.state;
        let mut stopped = flag.lock().unwrap_or_else(PoisonError::into_inner);
        while !*stopped {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            stopped = condition.wait_timeout(stopped, remaining).unwrap_or_else(PoisonError::into_inner).0;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_timeout_expires_without_a_stop() {
        assert!(!StopSignal::new().wait_timeout(Duration::from_millis(10)));
    }

    #[test]
    fn wait_timeout_returns_early_when_stopped() {
        let signal = StopSignal::new();
        let remote = signal.clone();
        std::thread::spawn(move || remote.stop());
        assert!(signal.wait_timeout(Duration::from_secs(10)));
        assert!(signal.is_stopped());
    }
}
