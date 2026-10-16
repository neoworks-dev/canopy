use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::error::Error;

fn write_project(root: &Path) -> PathBuf {
    let source = root.join("project");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("top.txt"), "top").unwrap();
    fs::write(source.join("nested/deep.txt"), "deep").unwrap();
    source
}

fn pack(archive_path: &Path, sources: &[PathBuf]) -> crate::Result<()> {
    create(archive_path, sources, |_name, _written| Ok(()))
}

fn read_members(archive_path: &Path) -> BTreeMap<String, String> {
    let mut contents = BTreeMap::new();
    walk(archive_path, |member, reader| {
        if member.is_dir {
            return Ok(());
        }
        let mut body = String::new();
        reader.read_to_string(&mut body)?;
        contents.insert(member.name.clone(), body);
        Ok(())
    })
    .unwrap();
    contents
}

#[test]
fn a_packed_tree_comes_back_whole() {
    for name in ["bundle.zip", "bundle.tar", "bundle.tar.gz"] {
        let root = tempfile::tempdir().unwrap();
        let source = write_project(root.path());
        let archive_path = root.path().join(name);

        pack(&archive_path, &[source]).unwrap();

        let contents = read_members(&archive_path);
        assert_eq!(contents["project/top.txt"], "top", "{name}");
        assert_eq!(contents["project/nested/deep.txt"], "deep", "{name}");
    }
}

#[test]
fn listing_reports_the_folders_as_well_as_the_files() {
    let root = tempfile::tempdir().unwrap();
    let source = write_project(root.path());
    let archive_path = root.path().join("bundle.zip");
    pack(&archive_path, &[source]).unwrap();

    let members = list(&archive_path).unwrap();

    let mut names: Vec<&str> = members.iter().map(|member| member.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["project", "project/nested", "project/nested/deep.txt", "project/top.txt"]);
    let nested = members.iter().find(|member| member.name == "project/nested").unwrap();
    assert!(nested.is_dir, "a folder inside an archive must be reported as one");
}

#[test]
fn a_tar_listing_has_sizes_modes_and_times() {
    let root = tempfile::tempdir().unwrap();
    let source = write_project(root.path());
    let archive_path = root.path().join("bundle.tar.gz");
    pack(&archive_path, &[source]).unwrap();

    let members = list(&archive_path).unwrap();

    let top = members.iter().find(|member| member.name == "project/top.txt").unwrap();
    assert_eq!(top.size, 3);
    assert!(top.mode.starts_with('-'));
    assert!(top.mod_time.contains('T'));
}

#[test]
fn a_visit_that_fails_stops_the_walk() {
    let root = tempfile::tempdir().unwrap();
    let source = write_project(root.path());
    let archive_path = root.path().join("bundle.zip");
    pack(&archive_path, &[source]).unwrap();

    let mut visited = 0;
    let result = walk(&archive_path, |_member, _reader| {
        visited += 1;
        Err(Error::Cancelled)
    });

    assert!(matches!(result, Err(Error::Cancelled)));
    assert_eq!(visited, 1);
}

#[test]
fn packing_a_format_that_cannot_be_written_is_refused() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("note.txt"), "body").unwrap();
    let archive_path = root.path().join("bundle.tar.bz2");

    let result = pack(&archive_path, &[root.path().join("note.txt")]);

    assert!(matches!(result, Err(Error::CannotWriteFormat)));
    assert!(!archive_path.exists(), "no half-made archive is left behind");
}

#[test]
fn opening_something_that_is_not_an_archive_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("note.txt");
    fs::write(&path, "body").unwrap();

    assert!(matches!(list(&path), Err(Error::UnsupportedFormat)));
}

#[test]
fn a_bzip2_tar_is_readable() {
    let root = tempfile::tempdir().unwrap();
    let archive_path = root.path().join("bundle.tar.bz2");
    let mut builder = tar::Builder::new(bzip2::write::BzEncoder::new(
        fs::File::create(&archive_path).unwrap(),
        bzip2::Compression::default(),
    ));
    let mut header = tar::Header::new_gnu();
    header.set_size(4);
    header.set_mode(0o644);
    builder.append_data(&mut header, "note.txt", &b"body"[..]).unwrap();
    builder.into_inner().unwrap().finish().unwrap();

    assert_eq!(read_members(&archive_path)["note.txt"], "body");
}

#[test]
fn the_report_sees_every_entry_and_can_stop_the_run() {
    let root = tempfile::tempdir().unwrap();
    let source = write_project(root.path());
    let archive_path = root.path().join("bundle.zip");

    let mut names = Vec::new();
    let result = create(&archive_path, &[source], |name, _written| {
        names.push(name.to_string());
        if names.len() == 2 {
            return Err(Error::Cancelled);
        }
        Ok(())
    });

    assert!(matches!(result, Err(Error::Cancelled)));
    assert_eq!(names, ["project", "project/nested"]);
}

#[test]
fn links_are_left_out_of_a_packed_archive() {
    let root = tempfile::tempdir().unwrap();
    let source = write_project(root.path());
    std::os::unix::fs::symlink("top.txt", source.join("link")).unwrap();
    let archive_path = root.path().join("bundle.tar");

    pack(&archive_path, &[source]).unwrap();

    assert!(list(&archive_path).unwrap().iter().all(|member| member.name != "project/link"));
}
