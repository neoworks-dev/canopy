//! The queue and worker behind every mutating operation, so copy, move, trash
//! and delete share the same progress reporting, conflict handling and undo
//! history.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::thread;

use super::history::History;
use super::names::{free_name, is_directory};
use super::operation::Operation;
use super::steps::{create_empty_file, creation_step, make_directory, rename_step, restore_step, step_for_operation};
use super::trash::Trash;
use super::types::{Kind, OperationEmitter, Resolution, Status};
use crate::archive;
use crate::error::{Error, Result};
use crate::paths;

const QUEUE_CAPACITY: usize = 64;

/// Runs operations one at a time on a worker thread and keeps the undo history.
pub struct Engine {
    shared: Arc<EngineShared>,
}

/// What the worker thread, the operations' history steps and the `Engine` handle
/// all reach.
pub(super) struct EngineShared {
    emitter: OperationEmitter,
    trash: Trash,
    queue: SyncSender<Arc<Operation>>,
    history: History,
    registry: Mutex<Registry>,
}

#[derive(Default)]
struct Registry {
    operations: HashMap<String, Arc<Operation>>,
    sequence: u64,
}

impl Engine {
    /// An engine that reports through `emitter` and uses the user's trash.
    pub fn new(emitter: OperationEmitter) -> Engine {
        Engine::with_trash(emitter, Trash::system())
    }

    /// An engine that trashes into `trash` instead of the user's own.
    pub fn with_trash(emitter: OperationEmitter, trash: Trash) -> Engine {
        let (queue, pending) = sync_channel(QUEUE_CAPACITY);
        let shared = Arc::new(EngineShared {
            emitter,
            trash,
            queue,
            history: History::default(),
            registry: Mutex::new(Registry::default()),
        });
        let worker_shared = Arc::downgrade(&shared);
        thread::spawn(move || run_worker(pending, worker_shared));
        Engine { shared }
    }

    /// Validates a request and hands it to the worker, reporting the id the
    /// caller needs to answer conflicts or cancel.
    pub fn enqueue<S: AsRef<Path>>(&self, kind: Kind, sources: &[S], destination: impl AsRef<Path>) -> Result<String> {
        let sources = sources.iter().map(|source| source.as_ref().to_path_buf()).collect();
        self.shared.queue_request(kind, sources, destination.as_ref().to_path_buf(), false)
    }

    /// Answers the conflict prompt an operation is blocked on.
    pub fn resolve(&self, operation_id: &str, resolution: Resolution, apply_to_all: bool) -> Result<()> {
        self.shared.lookup(operation_id)?.answer(resolution, apply_to_all);
        Ok(())
    }

    /// Stops an operation; whatever it already did stays done.
    pub fn cancel(&self, operation_id: &str) -> Result<()> {
        self.shared.lookup(operation_id)?.cancel();
        Ok(())
    }

    /// Holds a running operation between files and chunks until `resume`.
    pub fn pause(&self, operation_id: &str) -> Result<()> {
        let operation = self.shared.lookup(operation_id)?;
        operation.pause();
        operation.publish();
        Ok(())
    }

    /// Lets a paused operation carry on.
    pub fn resume(&self, operation_id: &str) -> Result<()> {
        let operation = self.shared.lookup(operation_id)?;
        operation.resume();
        operation.publish();
        Ok(())
    }

    /// Renames one entry inside its own directory and reports its new path.
    pub fn rename(&self, path: impl AsRef<Path>, new_name: &str) -> Result<String> {
        let path = path.as_ref();
        let target = free_name(&paths::directory_of(path), new_name)?;
        std::fs::rename(path, &target)?;
        let label = format!("Rename to {new_name}");
        self.shared.history.record(rename_step(label, path.to_path_buf(), target.clone()));
        Ok(target.to_string_lossy().into_owned())
    }

    /// Makes an empty directory and reports its path.
    pub fn create_folder(&self, directory: impl AsRef<Path>, name: &str) -> Result<String> {
        let target = free_name(directory.as_ref(), name)?;
        make_directory(&target)?;
        let label = format!("Create folder {name}");
        self.shared.history.record(creation_step(label, target.clone(), make_directory));
        Ok(target.to_string_lossy().into_owned())
    }

    /// Makes an empty file and reports its path.
    pub fn create_file(&self, directory: impl AsRef<Path>, name: &str) -> Result<String> {
        let target = free_name(directory.as_ref(), name)?;
        create_empty_file(&target)?;
        let label = format!("Create file {name}");
        self.shared.history.record(creation_step(label, target.clone(), create_empty_file));
        Ok(target.to_string_lossy().into_owned())
    }

    /// Puts one trashed item back and reports where it landed.
    pub fn restore_trashed(&self, trash_path: impl AsRef<Path>) -> Result<String> {
        let trash_path = trash_path.as_ref();
        let restored = self.shared.trash.restore(trash_path)?;
        let step = restore_step(self.shared.trash.clone(), PathBuf::from(&restored), trash_path.to_path_buf());
        self.shared.history.record(step);
        Ok(restored)
    }

    /// Reverses the most recent reversible action and reports its label.
    pub fn undo(&self) -> Result<String> {
        let mut step = self.shared.history.take_undoable().ok_or(Error::NothingToUndo)?;
        (step.revert)()?;
        let label = step.label.clone();
        self.shared.history.push_redoable(step);
        Ok(label)
    }

    /// Performs the action the last undo took back and reports its label.
    pub fn redo(&self) -> Result<String> {
        let mut step = self.shared.history.take_redoable().ok_or(Error::NothingToRedo)?;
        (step.replay)()?;
        let label = step.label.clone();
        self.shared.history.push_undoable(step);
        Ok(label)
    }

    /// Describes what `undo` would reverse, empty when there is nothing.
    pub fn undo_label(&self) -> String {
        self.shared.history.undo_label()
    }

    /// Describes what `redo` would perform, empty when there is nothing.
    pub fn redo_label(&self) -> String {
        self.shared.history.redo_label()
    }
}

fn run_worker(pending: Receiver<Arc<Operation>>, shared: Weak<EngineShared>) {
    for operation in pending {
        let Some(shared) = shared.upgrade() else {
            return;
        };
        shared.execute(&operation);
    }
}

impl EngineShared {
    fn registry(&self) -> MutexGuard<'_, Registry> {
        self.registry.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Runs an operation again for a redo. It records no history of its own,
    /// because the step being redone goes back on the undo stack already.
    pub(super) fn enqueue_replay(self: &Arc<Self>, kind: Kind, sources: &[PathBuf], destination: &Path) -> Result<()> {
        self.queue_request(kind, sources.to_vec(), destination.to_path_buf(), true)?;
        Ok(())
    }

    fn queue_request(
        self: &Arc<Self>,
        kind: Kind,
        sources: Vec<PathBuf>,
        destination: PathBuf,
        replaying: bool,
    ) -> Result<String> {
        validate_request(kind, &sources, &destination)?;
        let pending = self.new_operation(kind, sources, destination, replaying);
        pending.publish();
        let id = pending.id.clone();
        if self.queue.send(pending).is_err() {
            self.forget(&id);
            return Err(Error::message("the operation queue is closed"));
        }
        Ok(id)
    }

    fn new_operation(
        &self,
        kind: Kind,
        sources: Vec<PathBuf>,
        destination: PathBuf,
        replaying: bool,
    ) -> Arc<Operation> {
        let mut registry = self.registry();
        registry.sequence += 1;
        let id = format!("op-{}", registry.sequence);
        let operation = Arc::new(Operation::new(
            id.clone(),
            kind,
            sources,
            destination,
            replaying,
            self.trash.clone(),
            self.emitter.clone(),
        ));
        registry.operations.insert(id, operation.clone());
        operation
    }

    fn lookup(&self, operation_id: &str) -> Result<Arc<Operation>> {
        match self.registry().operations.get(operation_id) {
            Some(operation) => Ok(operation.clone()),
            None => Err(Error::UnknownRequest),
        }
    }

    fn forget(&self, operation_id: &str) {
        self.registry().operations.remove(operation_id);
    }

    fn execute(self: &Arc<Self>, operation: &Operation) {
        operation.set_status(Status::Running);
        operation.publish();
        operation.run_all();
        self.finish(operation);
    }

    fn finish(self: &Arc<Self>, operation: &Operation) {
        operation.set_status(operation.final_status());
        self.record_operation(operation);
        operation.emit_finished();
        self.forget(&operation.id);
    }

    /// Files what a finished batch actually did. A replay is left out: the step
    /// it belongs to is already back on the undo stack.
    fn record_operation(self: &Arc<Self>, operation: &Operation) {
        if operation.replaying {
            return;
        }
        if let Some(step) = step_for_operation(operation, Arc::downgrade(self)) {
            self.history.record(step);
        }
    }
}

fn validate_request(kind: Kind, sources: &[PathBuf], destination: &Path) -> Result<()> {
    if sources.is_empty() {
        return Err(Error::UnknownRequest);
    }
    if kind == Kind::Trash || kind == Kind::Delete {
        return Ok(());
    }
    // An extraction makes the folder it unpacks into, so its destination is the
    // one that may still be missing.
    if kind == Kind::Extract {
        return validate_extraction(sources, destination);
    }
    if !is_directory(destination) {
        return Err(Error::message(format!("destination {} is not a directory", destination.display())));
    }
    check_not_into_itself(sources, destination)
}

fn validate_extraction(sources: &[PathBuf], destination: &Path) -> Result<()> {
    if sources.len() != 1 {
        return Err(Error::UnknownRequest);
    }
    if destination.as_os_str().is_empty() {
        return Err(Error::message("an extraction needs a folder to unpack into"));
    }
    if !archive::is_archive(&sources[0].to_string_lossy()) {
        return Err(Error::UnsupportedFormat);
    }
    Ok(())
}

fn check_not_into_itself(sources: &[PathBuf], destination: &Path) -> Result<()> {
    for source in sources {
        if is_directory(source) && paths::contains_path(source, destination) {
            return Err(Error::IntoItself);
        }
    }
    Ok(())
}
