//! Tests for the operation engine, sharing one harness that collects the events
//! an engine emits and answers conflict prompts the way a test asks it to.

mod archive_tests;
mod engine_tests;
mod history_tests;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

use super::*;

const FINISH_TIMEOUT: Duration = Duration::from_secs(5);

type ConflictHandler = Box<dyn Fn(&Engine, &ConflictPrompt) + Send + Sync>;

pub(super) struct Harness {
    pub(super) engine: Arc<Engine>,
    finished: Receiver<Progress>,
    conflicts: Arc<Mutex<Vec<ConflictPrompt>>>,
    refuses_conflicts: bool,
    _trash_home: tempfile::TempDir,
}

/// What the engine's emitter does with each event: keep the conflicts, answer
/// them, and pass on the operations that finished.
struct EventSink {
    conflicts: Arc<Mutex<Vec<ConflictPrompt>>>,
    finished: Sender<Progress>,
    engine: Arc<OnceLock<Weak<Engine>>>,
    on_conflict: ConflictHandler,
}

impl EventSink {
    fn handle(&self, event: OperationEvent) {
        match event {
            OperationEvent::Conflict(prompt) => self.handle_conflict(&prompt),
            OperationEvent::Finished(progress) => {
                let _ = self.finished.send(progress);
            }
            OperationEvent::Progress(_) => {}
        }
    }

    fn handle_conflict(&self, prompt: &ConflictPrompt) {
        self.conflicts.lock().unwrap().push(prompt.clone());
        let Some(engine) = self.engine.get().and_then(Weak::upgrade) else {
            return;
        };
        (self.on_conflict)(&engine, prompt);
    }
}

impl Harness {
    fn new(on_conflict: ConflictHandler, refuses_conflicts: bool) -> Harness {
        let trash_home = tempfile::tempdir().unwrap();
        let (finished_sender, finished) = channel();
        let conflicts = Arc::new(Mutex::new(Vec::new()));
        let engine_cell: Arc<OnceLock<Weak<Engine>>> = Arc::new(OnceLock::new());
        let sink = EventSink {
            conflicts: conflicts.clone(),
            finished: finished_sender,
            engine: engine_cell.clone(),
            on_conflict,
        };
        let emitter: OperationEmitter = Arc::new(move |event| sink.handle(event));
        let engine = Arc::new(Engine::with_trash(emitter, Trash::in_data_home(trash_home.path())));
        let _ = engine_cell.set(Arc::downgrade(&engine));
        Harness { engine, finished, conflicts, refuses_conflicts, _trash_home: trash_home }
    }

    /// A harness that answers every conflict the same way.
    pub(super) fn answering(resolution: Resolution, apply_to_all: bool) -> Harness {
        let handler = move |engine: &Engine, prompt: &ConflictPrompt| {
            let _ = engine.resolve(&prompt.operation_id, resolution, apply_to_all);
        };
        Harness::new(Box::new(handler), false)
    }

    /// A harness for operations that must not hit a conflict.
    pub(super) fn refusing_conflicts() -> Harness {
        let handler = |engine: &Engine, prompt: &ConflictPrompt| {
            let _ = engine.resolve(&prompt.operation_id, Resolution::Cancel, false);
        };
        Harness::new(Box::new(handler), true)
    }

    /// A harness that leaves every conflict unanswered.
    pub(super) fn ignoring_conflicts() -> Harness {
        Harness::new(Box::new(|_engine, _prompt| {}), false)
    }

    pub(super) fn conflict_count(&self) -> usize {
        self.conflicts.lock().unwrap().len()
    }

    /// Queues an operation and waits for it to finish.
    pub(super) fn run(&self, kind: Kind, sources: &[&Path], destination: &Path) -> Progress {
        self.engine.enqueue(kind, sources, destination).unwrap();
        let progress = self.await_finished();
        if self.refuses_conflicts {
            assert_eq!(self.conflict_count(), 0, "unexpected conflict prompt");
        }
        progress
    }

    /// Waits for an operation, such as a replayed one that runs through the
    /// queue and so finishes after `redo` has already returned.
    pub(super) fn await_finished(&self) -> Progress {
        self.finished.recv_timeout(FINISH_TIMEOUT).expect("the operation did not finish in time")
    }
}

pub(super) fn write_file(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
}

pub(super) fn read_file(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

pub(super) fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

pub(super) fn undo(engine: &Engine) {
    let label = engine.undo().unwrap();
    assert!(!label.is_empty(), "undo must describe what it reversed");
}

pub(super) fn redo(engine: &Engine) {
    let label = engine.redo().unwrap();
    assert!(!label.is_empty(), "redo must describe what it performed");
}
