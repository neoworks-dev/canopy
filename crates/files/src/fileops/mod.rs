//! The operation engine: copy, move, trash, delete, extract and compress run
//! one at a time on a worker thread, with progress, conflict prompts,
//! pause/resume/cancel, and an undo/redo history.
//!
//! The caller supplies an [`OperationEmitter`] and receives
//! [`OperationEvent`]s; answering a conflict goes back through
//! [`Engine::resolve`].

mod archives;
mod copy;
mod device;
mod engine;
mod history;
mod links;
mod mounts;
mod names;
mod operation;
mod relocate;
mod run;
mod size;
mod steps;
mod trash;
mod types;

#[cfg(test)]
mod tests;

pub use archives::extraction_folder_for;
pub use engine::Engine;
pub use links::{LinkKind, create_links};
pub use names::{free_name, suggest_name, validate_name};
pub use trash::{Trash, TrashedItem, empty_trash, list_trashed, restore, trash};
pub use types::{
    ConflictPrompt, EVENT_CONFLICT, EVENT_FINISHED, EVENT_PROGRESS, Failure, Kind, OperationEmitter, OperationEvent,
    Progress, Resolution, Status,
};
