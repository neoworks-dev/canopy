//! One queued request and everything the worker, the conflict prompts and the
//! undo stack need to know about it: its status, its progress counters, and the
//! pause, cancel and conflict controls.

use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, MutexGuard};

use super::names::{exists, is_directory, unique_name};
use super::size::{prefix_sum, tree_size};
use super::trash::{Trash, TrashedItem};
use super::types::{ConflictPrompt, Failure, Kind, OperationEmitter, OperationEvent, Progress, Resolution, Status};
use crate::error::{Error, Result};
use crate::paths;

/// How much must be copied before another progress event is worth emitting.
const PUBLISH_BYTE_INTERVAL: i64 = 4 * 1024 * 1024;

/// The caller's answer to a conflict, waiting for the operation to read it.
struct ConflictAnswer {
    resolution: Resolution,
    apply_to_all: bool,
}

/// Remembers where something came from so a move can be reversed.
#[derive(Clone)]
pub(super) struct MovedItem {
    pub(super) from: PathBuf,
    pub(super) to: PathBuf,
}

/// What a finished operation actually did, which is what undoing it reverses.
pub(super) struct Outcome {
    pub(super) created: Vec<PathBuf>,
    pub(super) moved: Vec<MovedItem>,
    pub(super) trashed: Vec<TrashedItem>,
}

#[derive(Default)]
struct State {
    status: Status,
    current: String,
    done: usize,
    failures: Vec<Failure>,
    /// The answer that applies to every later conflict, once the caller chose
    /// "apply to all".
    blanket: Option<Resolution>,
    paused: bool,
    cancelled: bool,
    answer: Option<ConflictAnswer>,
    source_sizes: Vec<i64>,
    total_bytes: i64,
    done_bytes: i64,
    /// Byte count at the last emitted event, so events stay coarse.
    published_bytes: i64,
    created: Vec<PathBuf>,
    moved: Vec<MovedItem>,
    trashed: Vec<TrashedItem>,
}

impl State {
    /// Reads the caller's answer, remembering it for every later conflict when
    /// the caller chose "apply to all".
    fn take_answer(&mut self) -> Option<Resolution> {
        let answer = self.answer.take()?;
        if answer.apply_to_all {
            self.blanket = Some(answer.resolution);
        }
        Some(answer.resolution)
    }
}

pub(super) struct Operation {
    pub(super) id: String,
    pub(super) kind: Kind,
    pub(super) sources: Vec<PathBuf>,
    pub(super) destination: PathBuf,
    /// A replay is a redo running through the queue, which records no history.
    pub(super) replaying: bool,
    pub(super) trash: Trash,
    emitter: OperationEmitter,
    state: Mutex<State>,
    /// Signalled when the operation is resumed, cancelled or answered.
    changed: Condvar,
}

impl Operation {
    pub(super) fn new(
        id: String,
        kind: Kind,
        sources: Vec<PathBuf>,
        destination: PathBuf,
        replaying: bool,
        trash: Trash,
        emitter: OperationEmitter,
    ) -> Operation {
        Operation {
            id,
            kind,
            sources,
            destination,
            replaying,
            trash,
            emitter,
            state: Mutex::new(State::default()),
            changed: Condvar::new(),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    // Events

    /// Tells the caller where the operation is now.
    pub(super) fn publish(&self) {
        (self.emitter)(OperationEvent::Progress(self.snapshot()));
    }

    pub(super) fn emit_finished(&self) {
        (self.emitter)(OperationEvent::Finished(self.snapshot()));
    }

    pub(super) fn snapshot(&self) -> Progress {
        let state = self.state();
        Progress {
            id: self.id.clone(),
            kind: self.kind,
            status: state.status,
            destination: self.destination.to_string_lossy().into_owned(),
            total_items: self.sources.len(),
            done_items: state.done,
            total_bytes: state.total_bytes,
            done_bytes: state.done_bytes,
            current_path: state.current.clone(),
            failures: state.failures.clone(),
        }
    }

    // Status

    pub(super) fn set_status(&self, status: Status) {
        self.state().status = status;
    }

    pub(super) fn final_status(&self) -> Status {
        if self.is_cancelled() {
            return Status::Cancelled;
        }
        Status::Done
    }

    // Pause, resume and cancel

    pub(super) fn pause(&self) {
        let mut state = self.state();
        if state.status != Status::Running {
            return;
        }
        state.status = Status::Paused;
        state.paused = true;
    }

    pub(super) fn resume(&self) {
        let mut state = self.state();
        if state.status != Status::Paused {
            return;
        }
        state.status = Status::Running;
        state.paused = false;
        self.changed.notify_all();
    }

    /// Safe to call more than once: the UI may cancel an operation that has
    /// already answered a conflict with "cancel".
    pub(super) fn cancel(&self) {
        self.state().cancelled = true;
        self.changed.notify_all();
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.state().cancelled
    }

    /// Checked between files and between chunks of a large file, so both pause
    /// and cancel land promptly.
    pub(super) fn wait_while_paused(&self) -> Result<()> {
        let mut state = self.state();
        loop {
            if state.cancelled {
                return Err(Error::Cancelled);
            }
            if !state.paused {
                return Ok(());
            }
            state = self.changed.wait(state).unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    // Conflicts

    /// Delivers the caller's answer to the conflict the operation is blocked on.
    pub(super) fn answer(&self, resolution: Resolution, apply_to_all: bool) {
        self.state().answer = Some(ConflictAnswer { resolution, apply_to_all });
        self.changed.notify_all();
    }

    /// Decides where `source` may land. Two directories merge silently, because
    /// merging never destroys entries the source does not carry; anything else
    /// asks the caller.
    pub(super) fn resolve_target(&self, source: &Path, target: &Path) -> Result<PathBuf> {
        if !exists(target) {
            return Ok(target.to_path_buf());
        }
        if is_directory(source) && is_directory(target) {
            return Ok(target.to_path_buf());
        }
        let resolution = self.ask_conflict(source, target)?;
        self.apply_resolution(target, resolution)
    }

    fn apply_resolution(&self, target: &Path, resolution: Resolution) -> Result<PathBuf> {
        match resolution {
            Resolution::Replace => Ok(target.to_path_buf()),
            Resolution::Skip => Err(Error::Skipped),
            Resolution::KeepBoth => keep_both_target(target),
            Resolution::Cancel => {
                self.cancel();
                Err(Error::Cancelled)
            }
        }
    }

    fn ask_conflict(&self, source: &Path, target: &Path) -> Result<Resolution> {
        if let Some(blanket) = self.state().blanket {
            return Ok(blanket);
        }
        (self.emitter)(OperationEvent::Conflict(ConflictPrompt {
            operation_id: self.id.clone(),
            source_path: source.to_string_lossy().into_owned(),
            destination_path: target.to_string_lossy().into_owned(),
            source_is_dir: is_directory(source),
        }));
        self.wait_for_answer()
    }

    fn wait_for_answer(&self) -> Result<Resolution> {
        let mut state = self.state();
        loop {
            if state.cancelled {
                return Err(Error::Cancelled);
            }
            if let Some(resolution) = state.take_answer() {
                return Ok(resolution);
            }
            state = self.changed.wait(state).unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    // Progress

    /// Sizes the sources up front so progress can be reported in bytes.
    /// Trashing and deleting move no data, so they keep the item counter alone.
    pub(super) fn measure(&self) {
        if self.kind != Kind::Copy && self.kind != Kind::Move {
            return;
        }
        let mut sizes = Vec::with_capacity(self.sources.len());
        for source in &self.sources {
            match tree_size(source, self) {
                Ok(size) => sizes.push(size),
                Err(_) => return,
            }
        }
        let mut state = self.state();
        state.total_bytes = sizes.iter().sum();
        state.source_sizes = sizes;
    }

    /// Snaps the byte counter to the exact total of the sources finished so
    /// far, which both corrects drift and covers a move that renamed rather
    /// than copied and therefore streamed no bytes at all.
    fn advance(&self, index: usize) {
        let mut state = self.state();
        state.done += 1;
        state.done_bytes = prefix_sum(&state.source_sizes, index + 1);
    }

    /// Counts bytes written, publishing only when enough has moved: one event
    /// per copied chunk would be thousands per gigabyte.
    pub(super) fn add_bytes(&self, count: i64) {
        if self.accumulate(count) {
            self.publish();
        }
    }

    fn accumulate(&self, count: i64) -> bool {
        let mut state = self.state();
        state.done_bytes += count;
        if state.done_bytes - state.published_bytes < PUBLISH_BYTE_INTERVAL {
            return false;
        }
        state.published_bytes = state.done_bytes;
        true
    }

    pub(super) fn set_current(&self, path: &Path) {
        self.state().current = path.to_string_lossy().into_owned();
        self.publish();
    }

    /// Files the end of one source: it counts as done, and a failure other than
    /// a skip is reported.
    pub(super) fn record_outcome(&self, index: usize, source: &Path, outcome: Result<()>) {
        self.advance(index);
        if let Err(error) = outcome
            && !error.is_skipped()
        {
            self.add_failure(source, &error);
        }
        self.publish();
    }

    fn add_failure(&self, source: &Path, error: &Error) {
        let failure = Failure { path: source.to_string_lossy().into_owned(), message: error.to_string() };
        self.state().failures.push(failure);
    }

    // What the operation did

    pub(super) fn remember_created(&self, target: &Path) {
        self.state().created.push(target.to_path_buf());
    }

    pub(super) fn remember_moved(&self, source: &Path, target: &Path) {
        self.state().moved.push(MovedItem { from: source.to_path_buf(), to: target.to_path_buf() });
    }

    pub(super) fn remember_trashed(&self, item: TrashedItem) {
        self.state().trashed.push(item);
    }

    /// Whether the destination folder is itself something this operation made,
    /// in which case removing it covers everything inside.
    pub(super) fn created_destination(&self) -> bool {
        let state = self.state();
        state.created.first().is_some_and(|first| *first == self.destination)
    }

    pub(super) fn outcome(&self) -> Outcome {
        let state = self.state();
        Outcome { created: state.created.clone(), moved: state.moved.clone(), trashed: state.trashed.clone() }
    }
}

fn keep_both_target(target: &Path) -> Result<PathBuf> {
    let directory = paths::directory_of(target);
    let name = unique_name(&directory, &paths::base_name(target))?;
    Ok(directory.join(name))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    use super::*;

    fn running_operation() -> Arc<Operation> {
        let operation = Operation::new(
            "op-1".to_string(),
            Kind::Copy,
            vec![PathBuf::from("/source")],
            PathBuf::from("/destination"),
            false,
            Trash::system(),
            Arc::new(|_event| {}),
        );
        operation.set_status(Status::Running);
        Arc::new(operation)
    }

    fn wait_in_thread(operation: &Arc<Operation>) -> (thread::JoinHandle<Result<()>>, Arc<AtomicBool>) {
        let returned = Arc::new(AtomicBool::new(false));
        let (waiting, flag) = (operation.clone(), returned.clone());
        let handle = thread::spawn(move || {
            let outcome = waiting.wait_while_paused();
            flag.store(true, Ordering::SeqCst);
            outcome
        });
        (handle, returned)
    }

    #[test]
    fn a_paused_operation_waits_until_it_is_resumed() {
        let operation = running_operation();
        operation.pause();
        assert_eq!(operation.snapshot().status, Status::Paused);

        let (handle, returned) = wait_in_thread(&operation);
        thread::sleep(Duration::from_millis(100));
        assert!(!returned.load(Ordering::SeqCst), "a paused operation must not continue");
        operation.resume();

        assert!(handle.join().unwrap().is_ok());
        assert_eq!(operation.snapshot().status, Status::Running);
    }

    #[test]
    fn cancelling_releases_a_paused_operation() {
        let operation = running_operation();
        operation.pause();

        let (handle, _returned) = wait_in_thread(&operation);
        thread::sleep(Duration::from_millis(50));
        operation.cancel();

        assert!(matches!(handle.join().unwrap(), Err(Error::Cancelled)));
        assert_eq!(operation.final_status(), Status::Cancelled);
    }

    #[test]
    fn only_a_running_operation_can_be_paused() {
        let operation = running_operation();
        operation.set_status(Status::Queued);
        operation.pause();
        assert_eq!(operation.snapshot().status, Status::Queued);
        assert!(operation.wait_while_paused().is_ok());
    }

    #[test]
    fn progress_events_are_coarse() {
        let operation = running_operation();
        operation.add_bytes(1024);
        assert_eq!(operation.snapshot().done_bytes, 1024);
        assert!(!operation.accumulate(1024));
        assert!(operation.accumulate(PUBLISH_BYTE_INTERVAL));
    }
}
