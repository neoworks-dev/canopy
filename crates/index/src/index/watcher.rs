use std::collections::HashSet;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};

use super::error::IndexResult;
use crate::stop::StopSignal;

type EventReceiver = Receiver<notify::Result<Event>>;

#[derive(Default)]
struct WatcherState {
    watched_paths: HashSet<String>,
    pending_paths: HashSet<String>,
    limit_reached: bool,
}

/// Turns inotify events into debounced batches of changed paths. It adds one
/// watch per directory, so it enforces its own ceiling: inotify watches are a
/// shared per-user resource and exhausting them would break other applications.
pub struct Watcher {
    notifier: Mutex<RecommendedWatcher>,
    events: Mutex<Option<EventReceiver>>,
    maximum_watches: usize,
    quiet_period: Duration,
    state: Mutex<WatcherState>,
}

fn lock<Value>(mutex: &Mutex<Value>) -> MutexGuard<'_, Value> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Watcher {
    pub fn new(maximum_watches: usize, quiet_period: Duration) -> IndexResult<Watcher> {
        let (sender, receiver) = mpsc::channel();
        let notifier = notify::recommended_watcher(move |result| {
            let _ = sender.send(result);
        })
        .map_err(into_io_error)?;
        Ok(Watcher {
            notifier: Mutex::new(notifier),
            events: Mutex::new(Some(receiver)),
            maximum_watches,
            quiet_period,
            state: Mutex::new(WatcherState::default()),
        })
    }

    /// Starts watching a directory. Exceeding the watch ceiling is not an error:
    /// the tree stays indexed, it just relies on the periodic rescan for updates.
    pub fn add(&self, directory_path: &str) {
        let mut state = lock(&self.state);
        if !self.can_watch_more(&mut state, directory_path) {
            return;
        }
        let outcome = lock(&self.notifier).watch(Path::new(directory_path), RecursiveMode::NonRecursive);
        if let Err(error) = outcome {
            log::debug!("watch failed path={directory_path} error={error}");
            return;
        }
        state.watched_paths.insert(directory_path.to_string());
    }

    /// Stops watching a removed directory and everything under it.
    pub fn drop_subtree(&self, directory_path: &str) {
        let mut state = lock(&self.state);
        let removed_prefix = format!("{directory_path}/");
        let dropped: Vec<String> = state
            .watched_paths
            .iter()
            .filter(|watched| watched.as_str() == directory_path || watched.starts_with(&removed_prefix))
            .cloned()
            .collect();
        for watched_path in dropped {
            let _ = lock(&self.notifier).unwatch(Path::new(&watched_path));
            state.watched_paths.remove(&watched_path);
        }
    }

    pub fn watched_count(&self) -> usize {
        lock(&self.state).watched_paths.len()
    }

    pub fn limit_reached(&self) -> bool {
        lock(&self.state).limit_reached
    }

    /// Pumps events until `stop` is raised, calling `handler` with each
    /// coalesced batch of paths. It owns no state beyond the pending set, so
    /// `add` is safe to call concurrently.
    pub fn run(&self, stop: &StopSignal, handler: impl Fn(Vec<String>)) {
        let Some(events) = lock(&self.events).take() else {
            return;
        };
        let mut next_flush = Instant::now() + self.quiet_period;
        while !stop.is_stopped() {
            if !self.receive_until(&events, next_flush) {
                return;
            }
            if Instant::now() >= next_flush {
                self.flush_pending(&handler);
                next_flush = Instant::now() + self.quiet_period;
            }
        }
    }

    /// Records at most one event, reporting whether the event source is alive.
    fn receive_until(&self, events: &EventReceiver, deadline: Instant) -> bool {
        match events.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(Ok(event)) => self.record_event(&event),
            Ok(Err(error)) => log::warn!("watch error error={error}"),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return false,
        }
        true
    }

    fn record_event(&self, event: &Event) {
        if matches!(event.kind, EventKind::Access(_) | EventKind::Other) {
            return;
        }
        let mut state = lock(&self.state);
        for path in &event.paths {
            state.pending_paths.insert(path.to_string_lossy().into_owned());
        }
    }

    fn flush_pending(&self, handler: &impl Fn(Vec<String>)) {
        let paths: Vec<String> = lock(&self.state).pending_paths.drain().collect();
        if paths.is_empty() {
            return;
        }
        handler(paths);
    }

    fn can_watch_more(&self, state: &mut WatcherState, directory_path: &str) -> bool {
        if state.watched_paths.contains(directory_path) {
            return false;
        }
        if state.watched_paths.len() < self.maximum_watches {
            return true;
        }
        if !state.limit_reached {
            state.limit_reached = true;
            log::warn!(
                "inotify watch ceiling reached; falling back to periodic rescans maximumWatches={}",
                self.maximum_watches
            );
        }
        false
    }
}

fn into_io_error(error: notify::Error) -> super::error::IndexError {
    super::error::IndexError::Io(std::io::Error::other(error))
}
