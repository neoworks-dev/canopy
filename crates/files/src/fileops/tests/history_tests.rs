use std::fs;

use super::*;
use crate::error::Error;

#[test]
fn redo_of_a_move_puts_it_back_at_the_destination() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let original = source.path().join("note.txt");
    write_file(&original, "body");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Move, &[&original], destination.path());
    undo(&harness.engine);
    redo(&harness.engine);

    assert!(!original.exists(), "redo of a move must take the file away again");
    assert_eq!(read_file(&destination.path().join("note.txt")), "body");
}

#[test]
fn redo_of_a_copy_makes_the_copy_again() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    write_file(&source.path().join("note.txt"), "body");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Copy, &[&source.path().join("note.txt")], destination.path());
    undo(&harness.engine);
    redo(&harness.engine);
    harness.await_finished();

    assert_eq!(read_file(&destination.path().join("note.txt")), "body");
}

/// A replayed copy is queued like any other, but it belongs to the step
/// already back on the undo stack rather than to a new one.
#[test]
fn a_redone_copy_does_not_stack_a_second_undo() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    write_file(&source.path().join("note.txt"), "body");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Copy, &[&source.path().join("note.txt")], destination.path());
    undo(&harness.engine);
    redo(&harness.engine);
    harness.await_finished();
    undo(&harness.engine);

    assert!(!destination.path().join("note.txt").exists(), "one undo after a redo removes the copy again");
    assert_eq!(harness.engine.undo_label(), "", "a replay must not record a history step of its own");
}

#[test]
fn redo_of_a_trashing_tracks_the_new_trash_path() {
    let directory = tempfile::tempdir().unwrap();
    let original = directory.path().join("note.txt");
    write_file(&original, "body");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Trash, &[&original], Path::new(""));
    undo(&harness.engine);
    redo(&harness.engine);

    assert!(!original.exists(), "redo of a trashing must take the file away again");
    undo(&harness.engine);
    assert_eq!(read_file(&original), "body", "undo after a redo restores from where it landed the second time");
}

#[test]
fn redo_of_a_restore_takes_the_item_out_of_the_trash_again() {
    let trash_home = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let original = directory.path().join("note.txt");
    write_file(&original, "body");
    let trash = Trash::in_data_home(trash_home.path());
    let engine = Engine::with_trash(std::sync::Arc::new(|_event| {}), trash.clone());
    let trashed = trash.trash(&original).unwrap();

    engine.restore_trashed(&trashed.trash_path).unwrap();
    undo(&engine);
    assert!(!original.exists(), "undo of a restore must trash the item again");
    redo(&engine);

    assert_eq!(read_file(&original), "body", "redo of a restore must bring the item back");
}

#[test]
fn redo_of_a_created_folder_makes_it_again() {
    let directory = tempfile::tempdir().unwrap();
    let engine = Engine::new(std::sync::Arc::new(|_event| {}));

    let created = engine.create_folder(directory.path(), "reports").unwrap();
    undo(&engine);
    assert!(!std::path::Path::new(&created).exists(), "undo of a new folder must remove it");
    redo(&engine);

    assert!(std::path::Path::new(&created).is_dir());
}

#[test]
fn a_fresh_action_drops_what_was_redoable() {
    let directory = tempfile::tempdir().unwrap();
    write_file(&directory.path().join("note.txt"), "body");
    let engine = Engine::new(std::sync::Arc::new(|_event| {}));

    engine.rename(directory.path().join("note.txt"), "renamed.txt").unwrap();
    undo(&engine);
    assert_ne!(engine.redo_label(), "", "an undone rename must be redoable");
    engine.create_file(directory.path(), "other.txt").unwrap();

    assert_eq!(engine.redo_label(), "", "a fresh action must drop the redo stack");
}

#[test]
fn redo_without_an_undo_reports_nothing_to_redo() {
    let engine = Engine::new(std::sync::Arc::new(|_event| {}));
    assert!(matches!(engine.redo(), Err(Error::NothingToRedo)));
    assert!(matches!(engine.undo(), Err(Error::NothingToUndo)));
}

#[test]
fn labels_describe_the_last_action() {
    let directory = tempfile::tempdir().unwrap();
    let engine = Engine::new(std::sync::Arc::new(|_event| {}));

    engine.create_folder(directory.path(), "reports").unwrap();
    assert_eq!(engine.undo_label(), "Create folder reports");
    engine.undo().unwrap();
    assert_eq!(engine.redo_label(), "Create folder reports");
    assert!(fs::read_dir(directory.path()).unwrap().next().is_none());
}
