//! Finds files under a folder that hold the same bytes.
//!
//! Reading every file to compare it would be far more work than the answer is
//! worth, so the search narrows in three passes: files that share a size, then
//! files that share a signature taken from each end, and only then the full
//! contents of what is left.

mod digest;
mod search;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use search::SearchRun;

/// Event names the scanner emits. The UI subscribes to these.
pub const EVENT_PROGRESS: &str = "duplicates:progress";
pub const EVENT_FINISHED: &str = "duplicates:finished";

/// What a search is busy with, which is what the pane says while it waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Walking,
    Comparing,
    Done,
}

/// One set of files holding the same bytes. The first path is no more original
/// than the rest; they are in the order the walk found them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub size: i64,
    pub paths: Vec<String>,
}

/// Where a search has got to, and everything it found once it is done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub id: String,
    pub root: String,
    pub stage: Stage,
    /// Files the walk has seen.
    pub scanned: i64,
    /// Of those, the ones that share a size with another and are therefore
    /// worth reading.
    pub candidates: i64,
    pub compared: i64,
    /// Files that could not be read, which are left out of the comparison.
    pub unreadable: i64,
    /// What deleting all but one file of every group would free.
    pub recoverable_bytes: i64,
    /// The groups found; progress events leave them out (JSON `null`) and only
    /// the finished report carries them.
    pub groups: Option<Vec<Group>>,
    pub cancelled: bool,
}

/// Something a scanner tells its caller about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuplicatesEvent {
    /// Where a running search has got to, without the groups.
    Progress(Report),
    /// The search ended, whether complete or cancelled, with everything found.
    Finished(Report),
}

impl DuplicatesEvent {
    /// The event name the UI subscribes to (`duplicates:progress`, ...).
    pub fn name(&self) -> &'static str {
        match self {
            DuplicatesEvent::Progress(_) => EVENT_PROGRESS,
            DuplicatesEvent::Finished(_) => EVENT_FINISHED,
        }
    }

    /// The payload, serialised exactly as the UI expects it.
    pub fn payload(&self) -> serde_json::Value {
        let (DuplicatesEvent::Progress(report) | DuplicatesEvent::Finished(report)) = self;
        serde_json::to_value(report).unwrap_or(serde_json::Value::Null)
    }
}

/// Delivers an event to the caller. It runs on the searching thread.
pub type DuplicatesEmitter = Arc<dyn Fn(DuplicatesEvent) + Send + Sync>;

/// Runs the searches and keeps the handle each one is cancelled by. Cloning a
/// scanner gives another handle to the same searches.
#[derive(Clone)]
pub struct Scanner {
    shared: Arc<ScannerShared>,
}

struct ScannerShared {
    emitter: DuplicatesEmitter,
    running: Mutex<Running>,
}

#[derive(Default)]
struct Running {
    stops: HashMap<String, Arc<AtomicBool>>,
    sequence: u64,
}

impl Scanner {
    /// A scanner that reports every search through `emitter`.
    pub fn new(emitter: DuplicatesEmitter) -> Scanner {
        let running = Mutex::new(Running::default());
        Scanner { shared: Arc::new(ScannerShared { emitter, running }) }
    }

    /// Begins searching a folder and reports the id its events carry.
    pub fn start(&self, root: impl AsRef<Path>) -> Result<String> {
        let root = root.as_ref();
        if !std::fs::metadata(root)?.is_dir() {
            return Err(Error::NotAFolderToSearch);
        }
        let (id, stop) = self.register();
        let scanner = self.clone();
        let (search_id, search_root) = (id.clone(), root.to_path_buf());
        thread::spawn(move || scanner.search(&search_id, &search_root, stop));
        Ok(id)
    }

    /// Stops a running search, which reports what it had found so far.
    pub fn cancel(&self, id: &str) -> Result<()> {
        let stop = self.running().stops.remove(id).ok_or(Error::UnknownSearch)?;
        stop.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn running(&self) -> MutexGuard<'_, Running> {
        self.shared.running.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn register(&self) -> (String, Arc<AtomicBool>) {
        let mut running = self.running();
        running.sequence += 1;
        let id = format!("duplicates-{}", running.sequence);
        let stop = Arc::new(AtomicBool::new(false));
        running.stops.insert(id.clone(), stop.clone());
        (id, stop)
    }

    fn search(&self, id: &str, root: &Path, stop: Arc<AtomicBool>) {
        let emitter = self.shared.emitter.clone();
        let mut run = SearchRun::new(id, root, stop, emitter.clone());
        run.walk(root);
        run.compare();
        self.running().stops.remove(id);
        let report = run.into_finished_report();
        emitter(DuplicatesEvent::Finished(report));
    }
}

#[cfg(test)]
mod tests;
