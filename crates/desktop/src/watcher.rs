//! Reports filesystem changes for the directories the explorer is currently
//! showing, so a listing does not go stale when something else writes to it.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};

/// A burst of writes (an extraction, a build) collapses into one notification
/// instead of a reload per file.
const COALESCE_WINDOW: Duration = Duration::from_millis(150);

/// Called with the path of a watched directory whose contents changed.
pub type ChangeCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Follows the directories on screen. Split panes and tabs mean there can be
/// several at once, so the set is replaced as a whole and the watches that
/// carry over are left running.
pub struct Watcher {
    notify: ChangeCallback,
    sessions: Mutex<HashMap<String, Session>>,
}

/// One notify watch and the thread draining it. Dropping the session drops the
/// watch, which closes the channel and ends the thread.
struct Session {
    _watch: RecommendedWatcher,
}

impl Watcher {
    pub fn new(notify: ChangeCallback) -> Watcher {
        Watcher { notify, sessions: Mutex::new(HashMap::new()) }
    }

    /// Follows exactly the given directories, dropping any others. A directory
    /// that cannot be watched is skipped: the pane showing it stays a
    /// snapshot, which is better than the rest going unwatched with it.
    pub fn watch(&self, paths: &[String]) {
        let mut sessions = self.lock();
        let wanted: HashSet<&String> = paths.iter().collect();
        sessions.retain(|path, _| wanted.contains(path));
        for path in paths {
            if sessions.contains_key(path) {
                continue;
            }
            if let Some(session) = start_session(path, Arc::clone(&self.notify)) {
                sessions.insert(path.clone(), session);
            }
        }
    }

    /// Drops every watch. Watching nothing is a valid state: the trash, tag and
    /// search panes show no directory.
    pub fn stop(&self) {
        self.lock().clear();
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, Session>> {
        match self.sessions.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

fn start_session(path: &str, notify: ChangeCallback) -> Option<Session> {
    let (sender, receiver) = mpsc::channel();
    let mut watch = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let Ok(event) = event else {
            return;
        };
        if is_content_change(&event.kind) {
            let _ = sender.send(());
        }
    })
    .ok()?;
    watch.watch(Path::new(path), RecursiveMode::NonRecursive).ok()?;
    let watched_path = path.to_string();
    thread::spawn(move || run_session(&watched_path, &receiver, &notify));
    Some(Session { _watch: watch })
}

/// Opening or reading a directory is not a change, and listing it would
/// otherwise trigger the reload that lists it again.
fn is_content_change(kind: &EventKind) -> bool {
    !matches!(kind, EventKind::Access(_))
}

fn run_session(path: &str, changes: &Receiver<()>, notify: &ChangeCallback) {
    while changes.recv().is_ok() {
        if !wait_out_window(changes) {
            return;
        }
        notify(path);
    }
}

/// The window is trailing-edge and never extended, so a directory under
/// constant writes still reports at a steady rate instead of starving. Returns
/// false when the watch was dropped while waiting.
fn wait_out_window(changes: &Receiver<()>) -> bool {
    let deadline = Instant::now() + COALESCE_WINDOW;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match changes.recv_timeout(remaining) {
            Ok(()) => {}
            Err(RecvTimeoutError::Timeout) => return true,
            Err(RecvTimeoutError::Disconnected) => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc::{Sender, channel};

    use super::*;

    fn watcher_reporting_to(sender: Sender<String>) -> Watcher {
        let sender = Mutex::new(sender);
        Watcher::new(Arc::new(move |path: &str| {
            let _ = sender.lock().unwrap().send(path.to_string());
        }))
    }

    fn wait_for_notification(notifications: &Receiver<String>, timeout: Duration) -> String {
        notifications.recv_timeout(timeout).expect("expected a change notification")
    }

    fn write_file(directory: &Path, name: &str) {
        fs::write(directory.join(name), "contents").unwrap();
    }

    fn text(path: &Path) -> String {
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn watch_reports_a_created_file() {
        let directory = tempfile::tempdir().unwrap();
        let (sender, notifications) = channel();
        let watcher = watcher_reporting_to(sender);

        watcher.watch(&[text(directory.path())]);
        write_file(directory.path(), "created.txt");

        assert_eq!(wait_for_notification(&notifications, Duration::from_secs(2)), text(directory.path()));
    }

    #[test]
    fn a_burst_of_writes_collapses_into_one_notification() {
        let directory = tempfile::tempdir().unwrap();
        let (sender, notifications) = channel();
        let watcher = watcher_reporting_to(sender);

        watcher.watch(&[text(directory.path())]);
        for index in 0..20 {
            write_file(directory.path(), &format!("burst-{index}.txt"));
        }

        wait_for_notification(&notifications, Duration::from_secs(2));
        thread::sleep(2 * COALESCE_WINDOW);
        assert!(notifications.try_recv().is_err(), "the burst produced extra notifications");
    }

    #[test]
    fn watching_another_directory_drops_the_previous_one() {
        let previous = tempfile::tempdir().unwrap();
        let current = tempfile::tempdir().unwrap();
        let (sender, notifications) = channel();
        let watcher = watcher_reporting_to(sender);

        watcher.watch(&[text(previous.path())]);
        watcher.watch(&[text(current.path())]);
        write_file(previous.path(), "ignored.txt");
        thread::sleep(4 * COALESCE_WINDOW);
        assert!(notifications.try_recv().is_err(), "the dropped directory still notified");

        write_file(current.path(), "seen.txt");
        assert_eq!(wait_for_notification(&notifications, Duration::from_secs(2)), text(current.path()));
    }

    #[test]
    fn a_missing_directory_is_skipped_and_the_rest_are_watched() {
        let directory = tempfile::tempdir().unwrap();
        let absent = tempfile::tempdir().unwrap().path().join("absent");
        let (sender, notifications) = channel();
        let watcher = watcher_reporting_to(sender);

        watcher.watch(&[text(&absent), text(directory.path())]);
        write_file(directory.path(), "seen.txt");

        assert_eq!(wait_for_notification(&notifications, Duration::from_secs(2)), text(directory.path()));
    }

    #[test]
    fn several_directories_are_watched_at_once() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let (sender, notifications) = channel();
        let watcher = watcher_reporting_to(sender);

        watcher.watch(&[text(first.path()), text(second.path())]);
        write_file(second.path(), "seen.txt");

        assert_eq!(wait_for_notification(&notifications, Duration::from_secs(2)), text(second.path()));
    }

    #[test]
    fn stop_ends_every_watch() {
        let directory = tempfile::tempdir().unwrap();
        let (sender, notifications) = channel();
        let watcher = watcher_reporting_to(sender);

        watcher.watch(&[text(directory.path())]);
        watcher.stop();
        write_file(directory.path(), "unseen.txt");
        thread::sleep(4 * COALESCE_WINDOW);

        assert!(notifications.try_recv().is_err());
    }
}
