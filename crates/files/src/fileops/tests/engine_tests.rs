use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use super::*;
use crate::error::Error;

#[test]
fn copy_duplicates_a_tree_with_its_permissions() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let tree = source.path().join("project");
    fs::create_dir_all(tree.join("nested")).unwrap();
    let script = tree.join("nested/script.sh");
    write_file(&script, "echo hi");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let harness = Harness::refusing_conflicts();
    let progress = harness.run(Kind::Copy, &[&tree], destination.path());

    assert!(progress.failures.is_empty(), "{:?}", progress.failures);
    let copied = destination.path().join("project/nested/script.sh");
    assert_eq!(read_file(&copied), "echo hi");
    assert_eq!(mode_of(&copied), 0o755);
    assert_eq!(read_file(&script), "echo hi", "copy must leave the source in place");
}

#[test]
fn copy_keeps_the_modification_time() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let original = source.path().join("note.txt");
    write_file(&original, "body");
    let stamp = filetime::FileTime::from_unix_time(1_700_000_000, 0);
    filetime::set_file_times(&original, stamp, stamp).unwrap();

    Harness::refusing_conflicts().run(Kind::Copy, &[&original], destination.path());

    let copied = fs::metadata(destination.path().join("note.txt")).unwrap();
    assert_eq!(filetime::FileTime::from_last_modification_time(&copied), stamp);
}

#[test]
fn copy_keeps_both_on_conflict() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    write_file(&source.path().join("note.txt"), "new");
    write_file(&destination.path().join("note.txt"), "old");

    let harness = Harness::answering(Resolution::KeepBoth, false);
    harness.run(Kind::Copy, &[&source.path().join("note.txt")], destination.path());

    assert_eq!(read_file(&destination.path().join("note.txt")), "old", "keep both must not touch the existing file");
    assert_eq!(read_file(&destination.path().join("note (1).txt")), "new");
}

#[test]
fn copy_replace_overwrites_the_destination() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    write_file(&source.path().join("note.txt"), "new");
    write_file(&destination.path().join("note.txt"), "old");

    let harness = Harness::answering(Resolution::Replace, false);
    harness.run(Kind::Copy, &[&source.path().join("note.txt")], destination.path());

    assert_eq!(read_file(&destination.path().join("note.txt")), "new");
}

#[test]
fn copy_skip_leaves_the_destination_alone() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    write_file(&source.path().join("note.txt"), "new");
    write_file(&destination.path().join("note.txt"), "old");

    let harness = Harness::answering(Resolution::Skip, false);
    let progress = harness.run(Kind::Copy, &[&source.path().join("note.txt")], destination.path());

    assert_eq!(read_file(&destination.path().join("note.txt")), "old");
    assert!(progress.failures.is_empty(), "a skip is not a failure: {:?}", progress.failures);
}

#[test]
fn cancelling_at_a_conflict_ends_the_operation_as_cancelled() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    write_file(&source.path().join("note.txt"), "new");
    write_file(&destination.path().join("note.txt"), "old");

    let harness = Harness::answering(Resolution::Cancel, false);
    let progress = harness.run(Kind::Copy, &[&source.path().join("note.txt")], destination.path());

    assert_eq!(progress.status, Status::Cancelled);
    assert_eq!(read_file(&destination.path().join("note.txt")), "old");
}

#[test]
fn apply_to_all_asks_only_once() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    for name in ["first.txt", "second.txt"] {
        write_file(&source.path().join(name), "new");
        write_file(&destination.path().join(name), "old");
    }

    let harness = Harness::answering(Resolution::Replace, true);
    let sources = [source.path().join("first.txt"), source.path().join("second.txt")];
    harness.run(Kind::Copy, &[&sources[0], &sources[1]], destination.path());

    assert_eq!(harness.conflict_count(), 1);
    assert_eq!(read_file(&destination.path().join("second.txt")), "new", "the blanket answer applies later");
}

#[test]
fn directories_merge_without_prompting() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    fs::create_dir(source.path().join("shared")).unwrap();
    write_file(&source.path().join("shared/incoming.txt"), "new");
    fs::create_dir(destination.path().join("shared")).unwrap();
    write_file(&destination.path().join("shared/existing.txt"), "old");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Copy, &[&source.path().join("shared")], destination.path());

    assert_eq!(read_file(&destination.path().join("shared/existing.txt")), "old");
    assert_eq!(read_file(&destination.path().join("shared/incoming.txt")), "new");
}

#[test]
fn move_relocates_and_undo_brings_it_back() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let original = source.path().join("note.txt");
    write_file(&original, "body");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Move, &[&original], destination.path());

    assert!(!original.exists(), "move must remove the source");
    assert_eq!(read_file(&destination.path().join("note.txt")), "body");
    undo(&harness.engine);
    assert_eq!(read_file(&original), "body");
}

#[test]
fn moving_a_directory_into_an_existing_one_merges_and_removes_the_source() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    fs::create_dir(source.path().join("shared")).unwrap();
    write_file(&source.path().join("shared/incoming.txt"), "new");
    fs::create_dir(destination.path().join("shared")).unwrap();
    write_file(&destination.path().join("shared/existing.txt"), "old");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Move, &[&source.path().join("shared")], destination.path());

    assert!(!source.path().join("shared").exists());
    assert_eq!(read_file(&destination.path().join("shared/incoming.txt")), "new");
    assert_eq!(read_file(&destination.path().join("shared/existing.txt")), "old");
}

#[test]
fn undo_of_a_copy_removes_the_copies() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    write_file(&source.path().join("note.txt"), "body");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Copy, &[&source.path().join("note.txt")], destination.path());
    harness.engine.undo().unwrap();

    assert!(!destination.path().join("note.txt").exists());
    assert!(source.path().join("note.txt").exists(), "undo of a copy must not touch the source");
}

#[test]
fn permanent_deletion_is_not_undoable() {
    let directory = tempfile::tempdir().unwrap();
    write_file(&directory.path().join("note.txt"), "body");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Delete, &[&directory.path().join("note.txt")], Path::new(""));

    assert!(!directory.path().join("note.txt").exists());
    assert_eq!(harness.engine.undo_label(), "");
}

#[test]
fn trashing_is_undoable() {
    let directory = tempfile::tempdir().unwrap();
    let original = directory.path().join("note.txt");
    write_file(&original, "body");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Trash, &[&original], Path::new(""));

    assert!(!original.exists(), "trashing must remove the file from its directory");
    harness.engine.undo().unwrap();
    assert_eq!(read_file(&original), "body");
}

#[test]
fn one_failure_does_not_abandon_the_batch() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    write_file(&source.path().join("good.txt"), "body");

    let harness = Harness::refusing_conflicts();
    let missing = source.path().join("missing.txt");
    let good = source.path().join("good.txt");
    let progress = harness.run(Kind::Copy, &[&missing, &good], destination.path());

    assert_eq!(progress.failures.len(), 1, "exactly the missing source: {:?}", progress.failures);
    assert_eq!(progress.failures[0].path, missing.to_str().unwrap());
    assert!(destination.path().join("good.txt").exists());
    assert_eq!(progress.done_items, 2);
}

#[test]
fn copy_reports_the_bytes_it_moved() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let tree = source.path().join("project");
    fs::create_dir_all(tree.join("nested")).unwrap();
    write_file(&tree.join("top.txt"), "0123456789");
    write_file(&tree.join("nested/deep.txt"), "abcde");
    let loose = source.path().join("loose.txt");
    write_file(&loose, "xyz");

    let harness = Harness::refusing_conflicts();
    let progress = harness.run(Kind::Copy, &[&tree, &loose], destination.path());

    assert_eq!(progress.total_bytes, 18, "the whole tree plus the loose file");
    assert_eq!(progress.done_bytes, progress.total_bytes);
}

#[test]
fn move_reports_bytes_even_when_it_only_renames() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("target");
    fs::create_dir(&destination).unwrap();
    let source = directory.path().join("note.txt");
    write_file(&source, "0123456789");

    let harness = Harness::refusing_conflicts();
    let progress = harness.run(Kind::Move, &[&source], &destination);

    assert_eq!((progress.done_bytes, progress.total_bytes), (10, 10));
}

#[test]
fn trashing_reports_no_byte_total() {
    let directory = tempfile::tempdir().unwrap();
    write_file(&directory.path().join("note.txt"), "body");

    let harness = Harness::refusing_conflicts();
    let progress = harness.run(Kind::Trash, &[&directory.path().join("note.txt")], Path::new(""));

    assert_eq!(progress.total_bytes, 0);
}

#[test]
fn copying_a_directory_into_itself_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let nested = directory.path().join("inner");
    fs::create_dir(&nested).unwrap();

    let harness = Harness::refusing_conflicts();
    let result = harness.engine.enqueue(Kind::Copy, &[directory.path()], &nested);

    assert!(matches!(result, Err(Error::IntoItself)));
}

#[test]
fn an_empty_selection_is_refused() {
    let destination = tempfile::tempdir().unwrap();
    let harness = Harness::refusing_conflicts();
    let none: [&Path; 0] = [];

    assert!(matches!(harness.engine.enqueue(Kind::Copy, &none, destination.path()), Err(Error::UnknownRequest)));
}

#[test]
fn a_destination_that_is_not_a_directory_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    write_file(&directory.path().join("note.txt"), "body");
    let harness = Harness::refusing_conflicts();

    let result =
        harness.engine.enqueue(Kind::Copy, &[directory.path().join("note.txt")], directory.path().join("absent"));

    assert!(result.is_err());
}

#[test]
fn rename_is_validated_and_undoable() {
    let directory = tempfile::tempdir().unwrap();
    let original = directory.path().join("note.txt");
    write_file(&original, "body");
    let harness = Harness::refusing_conflicts();

    assert!(matches!(harness.engine.rename(&original, "bad/name"), Err(Error::NameSeparator)));
    let renamed = harness.engine.rename(&original, "renamed.txt").unwrap();

    assert_eq!(read_file(Path::new(&renamed)), "body");
    harness.engine.undo().unwrap();
    assert!(original.exists(), "undo of a rename must restore the old name");
}

#[test]
fn create_folder_and_file_refuse_taken_names() {
    let directory = tempfile::tempdir().unwrap();
    let harness = Harness::refusing_conflicts();

    let created = harness.engine.create_folder(directory.path(), "reports").unwrap();

    assert!(Path::new(&created).is_dir());
    assert!(matches!(harness.engine.create_file(directory.path(), "reports"), Err(Error::NameExists)));
}

#[test]
fn an_unknown_operation_cannot_be_controlled() {
    let harness = Harness::refusing_conflicts();

    assert!(matches!(harness.engine.cancel("op-404"), Err(Error::UnknownRequest)));
    assert!(matches!(harness.engine.pause("op-404"), Err(Error::UnknownRequest)));
    assert!(matches!(harness.engine.resume("op-404"), Err(Error::UnknownRequest)));
    assert!(matches!(harness.engine.resolve("op-404", Resolution::Skip, false), Err(Error::UnknownRequest)));
}

#[test]
fn an_operation_waiting_on_a_conflict_can_be_cancelled() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    write_file(&source.path().join("note.txt"), "new");
    write_file(&destination.path().join("note.txt"), "old");
    let harness = Harness::ignoring_conflicts();

    let id = harness.engine.enqueue(Kind::Copy, &[source.path().join("note.txt")], destination.path()).unwrap();
    while harness.conflict_count() == 0 {
        std::thread::sleep(Duration::from_millis(5));
    }
    harness.engine.cancel(&id).unwrap();
    let progress = harness.await_finished();

    assert_eq!(progress.status, Status::Cancelled);
    assert_eq!(read_file(&destination.path().join("note.txt")), "old");
}
