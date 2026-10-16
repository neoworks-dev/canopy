use std::sync::Mutex;

use crate::error::Result;

/// Something a step does when it is taken back or put again.
pub(super) type Action = Box<dyn FnMut() -> Result<()> + Send>;

/// One action the user can take back and put again: `revert` undoes what it
/// did, `replay` does it a second time.
pub(super) struct HistoryStep {
    pub(super) label: String,
    pub(super) revert: Action,
    pub(super) replay: Action,
}

/// What can be undone and what a preceding undo made redoable.
#[derive(Default)]
pub(super) struct History {
    stacks: Mutex<Stacks>,
}

#[derive(Default)]
struct Stacks {
    undoable: Vec<HistoryStep>,
    redoable: Vec<HistoryStep>,
}

impl History {
    fn stacks(&self) -> std::sync::MutexGuard<'_, Stacks> {
        self.stacks.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Files a fresh action. Anything the user had undone stops being redoable,
    /// because the state it would return to no longer exists.
    pub(super) fn record(&self, step: HistoryStep) {
        let mut stacks = self.stacks();
        stacks.undoable.push(step);
        stacks.redoable.clear();
    }

    pub(super) fn push_undoable(&self, step: HistoryStep) {
        self.stacks().undoable.push(step);
    }

    pub(super) fn push_redoable(&self, step: HistoryStep) {
        self.stacks().redoable.push(step);
    }

    pub(super) fn take_undoable(&self) -> Option<HistoryStep> {
        self.stacks().undoable.pop()
    }

    pub(super) fn take_redoable(&self) -> Option<HistoryStep> {
        self.stacks().redoable.pop()
    }

    pub(super) fn undo_label(&self) -> String {
        last_label(&self.stacks().undoable)
    }

    pub(super) fn redo_label(&self) -> String {
        last_label(&self.stacks().redoable)
    }
}

fn last_label(stack: &[HistoryStep]) -> String {
    match stack.last() {
        Some(step) => step.label.clone(),
        None => String::new(),
    }
}
