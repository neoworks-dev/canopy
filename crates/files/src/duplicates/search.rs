use std::collections::{HashMap, HashSet};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use walkdir::WalkDir;

use super::digest::{contents_of, signature_of};
use super::{DuplicatesEmitter, DuplicatesEvent, Group, Report, Stage};
use crate::error::Result;

/// How often a running search reports where it has got to.
const PUBLISH_INTERVAL: Duration = Duration::from_millis(120);

/// One file the walk found, kept with the information the identity check needs
/// so hard links are not counted as copies.
struct Candidate {
    path: PathBuf,
    device: u64,
    inode: u64,
}

type Digest = fn(&Path, i64) -> Result<String>;

/// The state of one search, which belongs to the thread running it and to
/// nothing else.
pub(super) struct SearchRun {
    report: Report,
    emitter: DuplicatesEmitter,
    stop: Arc<AtomicBool>,
    by_size: HashMap<i64, Vec<Candidate>>,
    /// Keeps progress events coarse: a search visits far more files than a pane
    /// can usefully redraw for. `None` until the first one goes out.
    published: Option<Instant>,
}

impl SearchRun {
    pub(super) fn new(id: &str, root: &Path, stop: Arc<AtomicBool>, emitter: DuplicatesEmitter) -> SearchRun {
        let report = Report {
            id: id.to_string(),
            root: root.to_string_lossy().into_owned(),
            stage: Stage::Walking,
            scanned: 0,
            candidates: 0,
            compared: 0,
            unreadable: 0,
            recoverable_bytes: 0,
            groups: Some(Vec::new()),
            cancelled: false,
        };
        SearchRun { report, emitter, stop, by_size: HashMap::new(), published: None }
    }

    pub(super) fn into_finished_report(mut self) -> Report {
        self.report.stage = Stage::Done;
        self.report
    }

    pub(super) fn walk(&mut self, root: &Path) {
        let walker = WalkDir::new(root).follow_links(false).follow_root_links(false).sort_by_file_name();
        for entry in walker {
            if self.stopped() {
                return;
            }
            self.consider(entry);
            self.publish();
        }
    }

    /// Folds one visited entry into the search. Only ordinary files with
    /// something in them can be copies of each other: an empty file matches
    /// every other empty file, which is noise rather than a finding.
    fn consider(&mut self, entry: walkdir::Result<walkdir::DirEntry>) {
        let Ok(entry) = entry else {
            self.report.unreadable += 1;
            return;
        };
        if !entry.file_type().is_file() {
            return;
        }
        let Ok(metadata) = entry.metadata() else {
            self.report.unreadable += 1;
            return;
        };
        self.report.scanned += 1;
        if metadata.len() == 0 {
            return;
        }
        let candidate = Candidate { path: entry.into_path(), device: metadata.dev(), inode: metadata.ino() };
        self.by_size.entry(metadata.len() as i64).or_default().push(candidate);
    }

    pub(super) fn compare(&mut self) {
        self.report.stage = Stage::Comparing;
        for size in self.sizes_largest_first() {
            if self.stopped() {
                return;
            }
            let sharing = self.by_size.remove(&size).unwrap_or_default();
            self.compare_size(size, sharing);
        }
    }

    /// The largest files first, so the groups worth the most space are the ones
    /// a cancelled search has already found.
    fn sizes_largest_first(&self) -> Vec<i64> {
        let mut sizes: Vec<i64> = self.by_size.keys().copied().collect();
        sizes.sort_by(|first, second| second.cmp(first));
        sizes
    }

    /// Takes one set of same-sized files down to the groups that really do hold
    /// the same bytes.
    fn compare_size(&mut self, size: i64, sharing: Vec<Candidate>) {
        let distinct = without_repeated_files(sharing);
        if distinct.len() < 2 {
            return;
        }
        self.report.candidates += distinct.len() as i64;
        for by_signature in self.split(distinct, size, signature_of) {
            let by_contents = self.split(by_signature, size, contents_of);
            self.collect(size, by_contents);
        }
    }

    /// Divides a set of files by what the digest makes of them, keeping only
    /// the sets that still have something to compare.
    fn split(&mut self, sharing: Vec<Candidate>, size: i64, digest: Digest) -> Vec<Vec<Candidate>> {
        let mut digest_order: Vec<String> = Vec::new();
        let mut by_digest: HashMap<String, Vec<Candidate>> = HashMap::new();
        for file in sharing {
            let Some(key) = self.digest_of(&file, size, digest) else {
                continue;
            };
            if !by_digest.contains_key(&key) {
                digest_order.push(key.clone());
            }
            by_digest.entry(key).or_default().push(file);
        }
        let mut kept = Vec::new();
        for key in digest_order {
            let matching = by_digest.remove(&key).unwrap_or_default();
            if matching.len() >= 2 {
                kept.push(matching);
            }
        }
        kept
    }

    fn digest_of(&mut self, file: &Candidate, size: i64, digest: Digest) -> Option<String> {
        if self.stopped() {
            return None;
        }
        let Ok(key) = digest(&file.path, size) else {
            self.report.unreadable += 1;
            return None;
        };
        self.report.compared += 1;
        self.publish();
        Some(key)
    }

    fn collect(&mut self, size: i64, found: Vec<Vec<Candidate>>) {
        for matching in found {
            let paths = matching.iter().map(|file| file.path.to_string_lossy().into_owned()).collect();
            if let Some(groups) = self.report.groups.as_mut() {
                groups.push(Group { size, paths });
            }
            self.report.recoverable_bytes += size * (matching.len() as i64 - 1);
        }
    }

    fn stopped(&mut self) -> bool {
        if self.stop.load(Ordering::SeqCst) {
            self.report.cancelled = true;
            return true;
        }
        false
    }

    fn publish(&mut self) {
        if self.published.is_some_and(|last| last.elapsed() < PUBLISH_INTERVAL) {
            return;
        }
        self.published = Some(Instant::now());
        (self.emitter)(DuplicatesEvent::Progress(self.progress_report()));
    }

    /// The groups are only worth sending once, with the finished report.
    fn progress_report(&self) -> Report {
        Report { id: self.report.id.clone(), root: self.report.root.clone(), groups: None, ..self.report }
    }
}

/// Drops the second name of a hard link. Two names for one file are not two
/// copies, and deleting one frees nothing.
fn without_repeated_files(sharing: Vec<Candidate>) -> Vec<Candidate> {
    let mut seen = HashSet::new();
    let mut distinct = Vec::with_capacity(sharing.len());
    for file in sharing {
        if seen.insert((file.device, file.inode)) {
            distinct.push(file);
        }
    }
    distinct
}
