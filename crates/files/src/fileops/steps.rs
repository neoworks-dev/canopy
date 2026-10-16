//! The history steps that take an action back and put it again: what undo and
//! redo do for each kind of operation.

use std::fs::{self, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use super::engine::EngineShared;
use super::history::HistoryStep;
use super::operation::{MovedItem, Operation};
use super::trash::{Trash, TrashedItem, remove_all};
use super::types::Kind;
use crate::error::{Error, Result};
use crate::paths;

/// The step that reverses what a finished operation actually did, if it did
/// anything reversible. Permanent deletion and a batch that did nothing produce
/// no step at all.
pub(super) fn step_for_operation(operation: &Operation, engine: Weak<EngineShared>) -> Option<HistoryStep> {
    let outcome = operation.outcome();
    match operation.kind {
        Kind::Move => move_step(outcome.moved),
        Kind::Trash => trash_step(operation.trash.clone(), outcome.trashed),
        Kind::Copy | Kind::Extract | Kind::Compress => creation_step_of(operation, outcome.created, engine),
        Kind::Delete => None,
    }
}

/// Covers the kinds that put something new on disk: taking one back removes
/// what it made, and doing it again is the same request run a second time. The
/// replay goes through the queue rather than by hand, so a redone copy of a
/// large tree still reports progress and still asks about collisions.
fn creation_step_of(operation: &Operation, created: Vec<PathBuf>, engine: Weak<EngineShared>) -> Option<HistoryStep> {
    if created.is_empty() {
        return None;
    }
    let kind = operation.kind;
    let destination = operation.destination.clone();
    let origins = operation.sources.clone();
    let label = creation_label(kind, &created);
    Some(HistoryStep {
        label,
        revert: Box::new(move || remove_targets(&created)),
        replay: Box::new(move || replay_through_queue(&engine, kind, &origins, &destination)),
    })
}

fn replay_through_queue(
    engine: &Weak<EngineShared>,
    kind: Kind,
    origins: &[PathBuf],
    destination: &Path,
) -> Result<()> {
    let Some(shared) = engine.upgrade() else {
        return Err(Error::message("the operation engine has shut down"));
    };
    shared.enqueue_replay(kind, origins, destination)
}

fn creation_label(kind: Kind, targets: &[PathBuf]) -> String {
    let first_name = paths::base_name(&targets[0]);
    match kind {
        Kind::Extract => format!("Extraction into {first_name}"),
        Kind::Compress => format!("Archive {first_name}"),
        _ => format!("Copy of {}", item_count(targets.len())),
    }
}

fn move_step(moved: Vec<MovedItem>) -> Option<HistoryStep> {
    if moved.is_empty() {
        return None;
    }
    let reverted = moved.clone();
    Some(HistoryStep {
        label: format!("Move of {}", item_count(moved.len())),
        revert: Box::new(move || move_back(&reverted)),
        replay: Box::new(move || move_forward(&moved)),
    })
}

/// Re-reads where each item landed on every trip: the trash renames what it
/// takes in, so the paths a restore needs are not the ones from before.
fn trash_step(trash: Trash, trashed: Vec<TrashedItem>) -> Option<HistoryStep> {
    if trashed.is_empty() {
        return None;
    }
    let label = format!("Trashing of {}", item_count(trashed.len()));
    let items = Arc::new(Mutex::new(trashed));
    let (reverting_items, reverting_trash) = (items.clone(), trash.clone());
    Some(HistoryStep {
        label,
        revert: Box::new(move || restore_all(&reverting_trash, &lock(&reverting_items))),
        replay: Box::new(move || trash_again(&trash, &items)),
    })
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Trashes the items again and remembers where they sit now, including the
/// ones it managed before an error stopped it.
fn trash_again(trash: &Trash, items: &Mutex<Vec<TrashedItem>>) -> Result<()> {
    let originals = lock(items).clone();
    let (replacements, outcome) = trash_each(trash, &originals);
    *lock(items) = replacements;
    outcome
}

fn trash_each(trash: &Trash, originals: &[TrashedItem]) -> (Vec<TrashedItem>, Result<()>) {
    let mut replacements = Vec::with_capacity(originals.len());
    for item in originals {
        match trash.trash(Path::new(&item.original_path)) {
            Ok(trashed) => replacements.push(trashed),
            Err(error) => return (replacements, Err(error)),
        }
    }
    (replacements, Ok(()))
}

fn restore_all(trash: &Trash, items: &[TrashedItem]) -> Result<()> {
    for item in items {
        trash.restore(Path::new(&item.trash_path))?;
    }
    Ok(())
}

fn remove_targets(targets: &[PathBuf]) -> Result<()> {
    for target in targets {
        remove_all(target)?;
    }
    Ok(())
}

fn move_back(items: &[MovedItem]) -> Result<()> {
    for item in items {
        fs::rename(&item.to, &item.from)?;
    }
    Ok(())
}

fn move_forward(items: &[MovedItem]) -> Result<()> {
    for item in items {
        fs::rename(&item.from, &item.to)?;
    }
    Ok(())
}

/// Reverses a new entry by removing it, and puts it back by making it a second
/// time.
pub(super) fn creation_step(label: String, target: PathBuf, create: fn(&Path) -> Result<()>) -> HistoryStep {
    let removed = target.clone();
    HistoryStep { label, revert: Box::new(move || remove_all(&removed)), replay: Box::new(move || create(&target)) }
}

/// Follows the item into the trash and out again: taking a restore back trashes
/// the item afresh, under a trash name only that moment knows.
pub(super) fn restore_step(trash: Trash, restored: PathBuf, trash_path: PathBuf) -> HistoryStep {
    let label = format!("Restore {}", paths::base_name(&restored));
    let current = Arc::new(Mutex::new(trash_path));
    let (reverting_trash, replaying_trash) = (trash.clone(), trash);
    let reverting_current = current.clone();
    HistoryStep {
        label,
        revert: Box::new(move || trash_restored(&reverting_trash, &restored, &reverting_current)),
        replay: Box::new(move || restore_current(&replaying_trash, &current)),
    }
}

fn trash_restored(trash: &Trash, restored: &Path, current: &Mutex<PathBuf>) -> Result<()> {
    let trashed = trash.trash(restored)?;
    *lock(current) = PathBuf::from(trashed.trash_path);
    Ok(())
}

fn restore_current(trash: &Trash, current: &Mutex<PathBuf>) -> Result<()> {
    let location = lock(current).clone();
    trash.restore(&location)?;
    Ok(())
}

/// Reverses a rename by renaming back.
pub(super) fn rename_step(label: String, original: PathBuf, renamed: PathBuf) -> HistoryStep {
    let (renamed_back, original_back) = (renamed.clone(), original.clone());
    HistoryStep {
        label,
        revert: Box::new(move || Ok(fs::rename(&renamed_back, &original_back)?)),
        replay: Box::new(move || Ok(fs::rename(&original, &renamed)?)),
    }
}

pub(super) fn make_directory(path: &Path) -> Result<()> {
    std::os::unix::fs::DirBuilderExt::mode(&mut fs::DirBuilder::new(), 0o755).create(path)?;
    Ok(())
}

pub(super) fn create_empty_file(path: &Path) -> Result<()> {
    OpenOptions::new().write(true).create_new(true).mode(0o644).open(path)?;
    Ok(())
}

fn item_count(count: usize) -> String {
    if count == 1 {
        return "1 item".to_string();
    }
    format!("{count} items")
}
