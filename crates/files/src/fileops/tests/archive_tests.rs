use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::error::Error;

/// Writes a small tree and packs it into an archive beside it, reporting where
/// the archive landed.
fn packed_tree(root: &Path, archive_name: &str) -> PathBuf {
    let source = root.join("project");
    fs::create_dir_all(source.join("nested")).unwrap();
    write_file(&source.join("top.txt"), "top");
    write_file(&source.join("nested/deep.txt"), "deep");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Compress, &[&source], root);
    root.join(archive_name)
}

#[test]
fn packing_a_selection_lands_beside_it() {
    let root = tempfile::tempdir().unwrap();
    assert!(packed_tree(root.path(), "project.zip").exists(), "packing one folder names the archive after it");
}

#[test]
fn packing_several_entries_gives_one_archive() {
    let root = tempfile::tempdir().unwrap();
    write_file(&root.path().join("one.txt"), "one");
    write_file(&root.path().join("two.txt"), "two");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Compress, &[&root.path().join("one.txt"), &root.path().join("two.txt")], root.path());

    assert!(root.path().join("Archive.zip").exists());
}

#[test]
fn extraction_unpacks_the_whole_tree() {
    let root = tempfile::tempdir().unwrap();
    let archive_path = packed_tree(root.path(), "project.zip");
    let destination = root.path().join("unpacked");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Extract, &[&archive_path], &destination);

    assert_eq!(read_file(&destination.join("project/top.txt")), "top");
    assert_eq!(read_file(&destination.join("project/nested/deep.txt")), "deep");
}

#[test]
fn undoing_an_extraction_removes_the_folder_it_made() {
    let root = tempfile::tempdir().unwrap();
    let archive_path = packed_tree(root.path(), "project.zip");
    let destination = root.path().join("unpacked");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Extract, &[&archive_path], &destination);
    harness.engine.undo().unwrap();

    assert!(!destination.exists(), "undo must remove the folder it unpacked into");
    assert!(archive_path.exists(), "undo must leave the archive alone");
}

#[test]
fn redoing_an_extraction_unpacks_it_again() {
    let root = tempfile::tempdir().unwrap();
    let archive_path = packed_tree(root.path(), "project.zip");
    let destination = root.path().join("unpacked");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Extract, &[&archive_path], &destination);
    undo(&harness.engine);
    redo(&harness.engine);
    harness.await_finished();

    assert_eq!(read_file(&destination.join("project/top.txt")), "top");
}

#[test]
fn undoing_packing_removes_the_archive() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("project");
    fs::create_dir(&source).unwrap();
    write_file(&source.join("top.txt"), "top");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Compress, &[&source], root.path());
    harness.engine.undo().unwrap();

    assert!(!root.path().join("project.zip").exists());
    assert!(source.join("top.txt").exists(), "undo of a packing must leave what was packed alone");
}

/// An extraction into a folder that already holds entries records what it added
/// rather than the folder, so undoing it does not take the folder with it.
#[test]
fn extracting_into_an_existing_folder_only_undoes_what_it_added() {
    let root = tempfile::tempdir().unwrap();
    let archive_path = packed_tree(root.path(), "project.zip");
    let destination = root.path().join("existing");
    fs::create_dir(&destination).unwrap();
    write_file(&destination.join("mine.txt"), "mine");

    let harness = Harness::refusing_conflicts();
    harness.run(Kind::Extract, &[&archive_path], &destination);
    harness.engine.undo().unwrap();

    assert!(!destination.join("project").exists(), "undo must remove what the extraction added");
    assert_eq!(read_file(&destination.join("mine.txt")), "mine");
}

#[test]
fn extracting_something_that_is_not_an_archive_is_refused() {
    let root = tempfile::tempdir().unwrap();
    write_file(&root.path().join("note.txt"), "body");
    let harness = Harness::refusing_conflicts();

    let result = harness.engine.enqueue(Kind::Extract, &[root.path().join("note.txt")], root.path().join("out"));

    assert!(matches!(result, Err(Error::UnsupportedFormat)));
}

#[test]
fn an_extraction_needs_a_folder_to_unpack_into() {
    let root = tempfile::tempdir().unwrap();
    let archive_path = packed_tree(root.path(), "project.zip");
    let harness = Harness::refusing_conflicts();

    assert!(harness.engine.enqueue(Kind::Extract, &[archive_path], Path::new("")).is_err());
}

#[test]
fn a_member_escaping_the_folder_fails_the_extraction() {
    let root = tempfile::tempdir().unwrap();
    let archive_path = root.path().join("evil.tar");
    let mut builder = tar::Builder::new(fs::File::create(&archive_path).unwrap());
    let mut header = tar::Header::new_gnu();
    header.set_size(4);
    header.set_mode(0o644);
    header.as_old_mut().name[..14].copy_from_slice(b"../outside.txt");
    header.set_cksum();
    builder.append(&header, &b"evil"[..]).unwrap();
    builder.finish().unwrap();

    let harness = Harness::refusing_conflicts();
    let progress = harness.run(Kind::Extract, &[&archive_path], &root.path().join("out"));

    assert_eq!(progress.failures.len(), 1);
    assert!(!root.path().join("outside.txt").exists());
}

#[test]
fn the_suggested_extraction_folder_is_named_after_the_archive() {
    let root = tempfile::tempdir().unwrap();
    let archive_path = root.path().join("photos.tar.gz");
    write_file(&archive_path, "not really an archive");

    assert_eq!(extraction_folder_for(&archive_path), root.path().join("photos"));
}

#[test]
fn the_suggested_extraction_folder_steps_aside_for_one_that_exists() {
    let root = tempfile::tempdir().unwrap();
    let archive_path = root.path().join("photos.zip");
    write_file(&archive_path, "not really an archive");
    fs::create_dir(root.path().join("photos")).unwrap();

    assert_eq!(extraction_folder_for(&archive_path), root.path().join("photos (1)"));
}
