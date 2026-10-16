use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use super::digest::SIGNATURE_LENGTH;
use super::*;

const FINISH_TIMEOUT: Duration = Duration::from_secs(10);

/// Keeps the finished report one scanner produces, and can react to the
/// progress along the way. The callback runs on the searching thread, so one
/// that cancels does so between two files rather than racing them.
struct Collector {
    scanner: Scanner,
    finished: Receiver<Report>,
}

impl Collector {
    fn new() -> Collector {
        Collector::watching(false)
    }

    fn watching(cancel_on_progress: bool) -> Collector {
        let (finished_sender, finished) = channel();
        let scanner_cell: Arc<OnceLock<Scanner>> = Arc::new(OnceLock::new());
        let cell = scanner_cell.clone();
        let emitter: DuplicatesEmitter = Arc::new(move |event| match event {
            DuplicatesEvent::Progress(report) if cancel_on_progress => {
                if let Some(scanner) = cell.get() {
                    let _ = scanner.cancel(&report.id);
                }
            }
            DuplicatesEvent::Progress(report) => assert!(report.groups.is_none(), "progress leaves the groups out"),
            DuplicatesEvent::Finished(report) => {
                let _ = finished_sender.send(report);
            }
        });
        let scanner = Scanner::new(emitter);
        let _ = scanner_cell.set(scanner.clone());
        Collector { scanner, finished }
    }

    fn search(&self, root: &Path) -> Report {
        self.scanner.start(root).unwrap();
        self.finished.recv_timeout(FINISH_TIMEOUT).expect("the search did not finish in time")
    }
}

fn write(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
}

fn groups_of(report: &Report) -> &[Group] {
    report.groups.as_deref().expect("a finished report carries its groups")
}

fn sorted_names(paths: &[String]) -> Vec<String> {
    let mut names: Vec<String> =
        paths.iter().map(|path| Path::new(path).file_name().unwrap().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn files_with_the_same_contents_are_found() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("nested")).unwrap();
    write(&root.path().join("one.txt"), "the same bytes");
    write(&root.path().join("nested/two.txt"), "the same bytes");
    write(&root.path().join("other.txt"), "different bytes");

    let report = Collector::new().search(root.path());

    assert_eq!(groups_of(&report).len(), 1, "the one pair");
    assert_eq!(groups_of(&report)[0].paths.len(), 2, "both copies");
    assert_eq!(report.recoverable_bytes, "the same bytes".len() as i64, "one copy's worth");
    assert_eq!(report.stage, Stage::Done);
}

#[test]
fn files_of_the_same_size_that_differ_are_not_a_group() {
    let root = tempfile::tempdir().unwrap();
    write(&root.path().join("one.txt"), "aaaa");
    write(&root.path().join("two.txt"), "bbbb");

    let report = Collector::new().search(root.path());

    assert!(groups_of(&report).is_empty(), "only the sizes match");
    assert_eq!(report.candidates, 2, "the two files worth reading");
}

/// Two files can share both ends and still differ, which is what the third
/// pass over the whole contents is for.
#[test]
fn files_that_share_both_ends_are_still_compared() {
    let root = tempfile::tempdir().unwrap();
    let head = "h".repeat(SIGNATURE_LENGTH as usize);
    let tail = "t".repeat(SIGNATURE_LENGTH as usize);
    write(&root.path().join("one.bin"), &format!("{head}middle one{tail}"));
    write(&root.path().join("two.bin"), &format!("{head}middle two{tail}"));
    write(&root.path().join("three.bin"), &format!("{head}middle one{tail}"));

    let report = Collector::new().search(root.path());

    assert_eq!(groups_of(&report).len(), 1, "only the pair that matches all through");
    assert_eq!(sorted_names(&groups_of(&report)[0].paths), ["one.bin", "three.bin"]);
}

#[test]
fn empty_files_are_not_reported_as_copies() {
    let root = tempfile::tempdir().unwrap();
    write(&root.path().join("one.txt"), "");
    write(&root.path().join("two.txt"), "");

    let report = Collector::new().search(root.path());

    assert!(groups_of(&report).is_empty(), "an empty file matches every other one");
}

#[test]
fn a_second_name_for_one_file_is_not_a_copy() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("one.txt");
    write(&original, "the same bytes");
    fs::hard_link(&original, root.path().join("linked.txt")).unwrap();

    let report = Collector::new().search(root.path());

    assert!(groups_of(&report).is_empty(), "deleting a hard link frees nothing");
}

#[test]
fn a_symlink_to_a_duplicate_is_left_out() {
    let root = tempfile::tempdir().unwrap();
    write(&root.path().join("one.txt"), "the same bytes");
    write(&root.path().join("two.txt"), "the same bytes");
    symlink(root.path().join("one.txt"), root.path().join("pointer.txt")).unwrap();

    let report = Collector::new().search(root.path());

    assert_eq!(sorted_names(&groups_of(&report)[0].paths), ["one.txt", "two.txt"]);
}

#[test]
fn the_largest_files_are_compared_first() {
    let root = tempfile::tempdir().unwrap();
    write(&root.path().join("small-one.txt"), "aa");
    write(&root.path().join("small-two.txt"), "aa");
    write(&root.path().join("big-one.txt"), &"b".repeat(400));
    write(&root.path().join("big-two.txt"), &"b".repeat(400));

    let report = Collector::new().search(root.path());

    assert_eq!(groups_of(&report).len(), 2, "both pairs");
    assert!(groups_of(&report)[0].size > groups_of(&report)[1].size, "the pair worth the most space comes first");
}

#[test]
fn searching_a_file_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("note.txt");
    write(&path, "body");

    assert!(matches!(Collector::new().scanner.start(&path), Err(Error::NotAFolderToSearch)));
}

#[test]
fn cancelling_a_search_nobody_started_is_refused() {
    assert!(matches!(Collector::new().scanner.cancel("duplicates-404"), Err(Error::UnknownSearch)));
}

#[test]
fn a_cancelled_search_stops_and_says_so() {
    let root = tempfile::tempdir().unwrap();
    write(&root.path().join("one.txt"), "the same bytes");
    write(&root.path().join("two.txt"), "the same bytes");

    let report = Collector::watching(true).search(root.path());

    assert!(report.cancelled, "a cancelled search must say it was cut short");
    assert!(groups_of(&report).is_empty(), "nothing compared after the walk was stopped");
}

#[test]
fn the_json_matches_the_go_field_names() {
    let root = tempfile::tempdir().unwrap();
    write(&root.path().join("one.txt"), "same");
    write(&root.path().join("two.txt"), "same");

    let report = Collector::new().search(root.path());
    let payload = DuplicatesEvent::Finished(report.clone()).payload();

    let object = payload.as_object().unwrap();
    for key in [
        "id",
        "root",
        "stage",
        "scanned",
        "candidates",
        "compared",
        "unreadable",
        "recoverableBytes",
        "groups",
        "cancelled",
    ] {
        assert!(object.contains_key(key), "{key}");
    }
    assert_eq!(payload["stage"], "done");
    assert_eq!(payload["groups"][0]["size"], 4);
    let progress = DuplicatesEvent::Progress(Report { groups: None, ..report }).payload();
    assert!(progress["groups"].is_null());
}
