//! Measures what a folder actually holds. Walking a tree can take minutes, so a
//! measurement runs in the background, reports what it has counted so far, and
//! can be called off when nobody is waiting for it any more.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::error::{Error, Result};

/// How often a running measurement reports what it has counted so far.
const PUBLISH_INTERVAL: Duration = Duration::from_millis(120);

/// Event names the scanner emits. The UI subscribes to these.
pub const EVENT_PROGRESS: &str = "usage:progress";
pub const EVENT_FINISHED: &str = "usage:finished";

/// What a walk has counted: a running total while it goes, and everything the
/// folder holds once it is finished. Sizes are what the files claim rather than
/// what they occupy, which is the same measure the operation engine reports
/// progress in.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Measurement {
    pub id: String,
    pub path: String,
    pub bytes: i64,
    pub files: i64,
    pub directories: i64,
    /// Entries the walk could not read, which contribute nothing to the total.
    pub unreadable: i64,
    pub cancelled: bool,
}

/// Something a scanner tells its caller about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageEvent {
    /// What a running measurement has counted so far.
    Progress(Measurement),
    /// The measurement ended, whether complete or cancelled.
    Finished(Measurement),
}

impl UsageEvent {
    /// The event name the UI subscribes to (`usage:progress`, ...).
    pub fn name(&self) -> &'static str {
        match self {
            UsageEvent::Progress(_) => EVENT_PROGRESS,
            UsageEvent::Finished(_) => EVENT_FINISHED,
        }
    }

    /// The payload, serialised exactly as the UI expects it.
    pub fn payload(&self) -> serde_json::Value {
        let (UsageEvent::Progress(measurement) | UsageEvent::Finished(measurement)) = self;
        serde_json::to_value(measurement).unwrap_or(serde_json::Value::Null)
    }
}

/// Delivers an event to the caller. It runs on the walking thread.
pub type UsageEmitter = Arc<dyn Fn(UsageEvent) + Send + Sync>;

/// Runs the measurements and keeps the handle each one is cancelled by. Cloning
/// a scanner gives another handle to the same measurements.
#[derive(Clone)]
pub struct Scanner {
    shared: Arc<ScannerShared>,
}

struct ScannerShared {
    emitter: UsageEmitter,
    running: Mutex<Running>,
}

#[derive(Default)]
struct Running {
    stops: HashMap<String, Arc<AtomicBool>>,
    sequence: u64,
}

impl Scanner {
    /// A scanner that reports every measurement through `emitter`.
    pub fn new(emitter: UsageEmitter) -> Scanner {
        let running = Mutex::new(Running::default());
        Scanner { shared: Arc::new(ScannerShared { emitter, running }) }
    }

    /// Begins measuring a folder and reports the id its events carry.
    pub fn start(&self, path: impl AsRef<Path>) -> Result<String> {
        let path = path.as_ref();
        if !std::fs::metadata(path)?.is_dir() {
            return Err(Error::NotAFolderToMeasure);
        }
        let (id, stop) = self.register();
        let scanner = self.clone();
        let (walk_id, root) = (id.clone(), path.to_path_buf());
        thread::spawn(move || scanner.walk(&walk_id, &root, &stop));
        Ok(id)
    }

    /// Stops a running measurement, which reports what it had counted.
    pub fn cancel(&self, id: &str) -> Result<()> {
        let stop = self.running().stops.remove(id).ok_or(Error::UnknownMeasurement)?;
        stop.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn running(&self) -> MutexGuard<'_, Running> {
        self.shared.running.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn register(&self) -> (String, Arc<AtomicBool>) {
        let mut running = self.running();
        running.sequence += 1;
        let id = format!("usage-{}", running.sequence);
        let stop = Arc::new(AtomicBool::new(false));
        running.stops.insert(id.clone(), stop.clone());
        (id, stop)
    }

    fn walk(&self, id: &str, root: &Path, stop: &AtomicBool) {
        let mut measurement =
            Measurement { id: id.to_string(), path: root.to_string_lossy().into_owned(), ..Measurement::default() };
        // No time has passed yet, so the first entry publishes and a pane shows
        // it is counting rather than sitting empty for the first interval.
        let mut published: Option<Instant> = None;
        let walker = WalkDir::new(root).follow_links(false).follow_root_links(false).sort_by_file_name();
        for entry in walker {
            if stop.load(Ordering::SeqCst) {
                measurement.cancelled = true;
                break;
            }
            count(&mut measurement, entry);
            published = self.publish_every(&measurement, published);
        }
        self.running().stops.remove(id);
        (self.shared.emitter)(UsageEvent::Finished(measurement));
    }

    /// Keeps the events coarse: a walk visits far more entries than a pane can
    /// usefully redraw for.
    fn publish_every(&self, measurement: &Measurement, published: Option<Instant>) -> Option<Instant> {
        if published.is_some_and(|last| last.elapsed() < PUBLISH_INTERVAL) {
            return published;
        }
        (self.shared.emitter)(UsageEvent::Progress(measurement.clone()));
        Some(Instant::now())
    }
}

/// Folds one visited entry into the totals. The root is the folder being
/// measured rather than something it holds, so it is left out. A folder that
/// cannot be opened is counted as a folder when it is visited and as
/// unreadable when its contents fail to load.
fn count(measurement: &mut Measurement, entry: walkdir::Result<walkdir::DirEntry>) {
    let entry = match entry {
        Ok(entry) => entry,
        Err(_) => {
            measurement.unreadable += 1;
            return;
        }
    };
    if entry.depth() == 0 {
        return;
    }
    if entry.file_type().is_dir() {
        measurement.directories += 1;
        return;
    }
    match entry.metadata() {
        Ok(metadata) => {
            measurement.files += 1;
            measurement.bytes += metadata.len() as i64;
        }
        Err(_) => measurement.unreadable += 1,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::OnceLock;
    use std::sync::mpsc::{Receiver, channel};

    use super::*;

    const FINISH_TIMEOUT: Duration = Duration::from_secs(5);

    /// Keeps the finished measurement one scanner reports, and can react to the
    /// progress along the way. The callback runs on the walking thread, so one
    /// that cancels does so between two entries rather than racing them.
    struct Collector {
        scanner: Scanner,
        finished: Receiver<Measurement>,
    }

    impl Collector {
        fn new(cancel_on_progress: bool) -> Collector {
            let (finished_sender, finished) = channel();
            let scanner_cell: Arc<OnceLock<Scanner>> = Arc::new(OnceLock::new());
            let cell = scanner_cell.clone();
            let emitter: UsageEmitter = Arc::new(move |event| match event {
                UsageEvent::Progress(measurement) if cancel_on_progress => {
                    if let Some(scanner) = cell.get() {
                        let _ = scanner.cancel(&measurement.id);
                    }
                }
                UsageEvent::Progress(_) => {}
                UsageEvent::Finished(measurement) => {
                    let _ = finished_sender.send(measurement);
                }
            });
            let scanner = Scanner::new(emitter);
            let _ = scanner_cell.set(scanner.clone());
            Collector { scanner, finished }
        }

        fn measure(&self, path: &Path) -> Measurement {
            self.scanner.start(path).unwrap();
            self.finished.recv_timeout(FINISH_TIMEOUT).expect("the measurement did not finish in time")
        }
    }

    #[test]
    fn measuring_counts_the_whole_tree() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("nested/deeper")).unwrap();
        fs::write(root.path().join("top.txt"), "0123456789").unwrap();
        fs::write(root.path().join("nested/middle.txt"), "abcde").unwrap();
        fs::write(root.path().join("nested/deeper/bottom.txt"), "xyz").unwrap();

        let measurement = Collector::new(false).measure(root.path());

        assert_eq!(measurement.bytes, 18);
        assert_eq!(measurement.files, 3);
        assert_eq!(measurement.directories, 2, "the two below the root");
        assert!(!measurement.cancelled);
    }

    #[test]
    fn an_empty_folder_measures_zero() {
        let root = tempfile::tempdir().unwrap();
        let measurement = Collector::new(false).measure(root.path());
        assert_eq!((measurement.bytes, measurement.files, measurement.directories), (0, 0, 0));
    }

    #[test]
    fn an_unreadable_folder_is_counted_rather_than_fatal() {
        if rustix::process::geteuid().is_root() {
            return;
        }
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("top.txt"), "0123456789").unwrap();
        let closed = root.path().join("closed");
        fs::create_dir(&closed).unwrap();
        fs::write(closed.join("hidden.txt"), "secret").unwrap();
        fs::set_permissions(&closed, fs::Permissions::from_mode(0o000)).unwrap();

        let measurement = Collector::new(false).measure(root.path());
        fs::set_permissions(&closed, fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(measurement.unreadable, 1);
        assert_eq!(measurement.directories, 1, "the folder is still a folder");
        assert_eq!(measurement.bytes, 10, "what could actually be read");
    }

    #[test]
    fn cancelling_stops_the_walk_and_says_so() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("nested")).unwrap();
        fs::write(root.path().join("nested/note.txt"), "body").unwrap();

        let measurement = Collector::new(true).measure(root.path());

        assert!(measurement.cancelled, "a cancelled measurement must say it was cut short");
        assert_eq!(measurement.files, 0, "nothing counted after the first entry");
    }

    #[test]
    fn measuring_a_file_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("note.txt");
        fs::write(&path, "body").unwrap();

        let result = Collector::new(false).scanner.start(&path);

        assert!(matches!(result, Err(Error::NotAFolderToMeasure)));
    }

    #[test]
    fn cancelling_a_measurement_nobody_started_is_refused() {
        let result = Collector::new(false).scanner.cancel("usage-404");
        assert!(matches!(result, Err(Error::UnknownMeasurement)));
    }

    #[test]
    fn the_json_matches_the_go_field_names() {
        let measurement = Measurement { id: "usage-1".to_string(), path: "/x".to_string(), ..Measurement::default() };
        assert_eq!(
            UsageEvent::Finished(measurement).payload(),
            serde_json::json!({
                "id": "usage-1", "path": "/x", "bytes": 0, "files": 0,
                "directories": 0, "unreadable": 0, "cancelled": false
            })
        );
    }
}
