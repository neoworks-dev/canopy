use std::fs::{self, Metadata};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use super::batch::Batch;
use super::error::{IndexError, IndexResult};
use super::exclusions::Exclusions;
use super::store::{FileRecord, ScanSession, Store};
use crate::stop::StopSignal;

/// Keeps the status callback cheap on large trees.
const PROGRESS_INTERVAL: i64 = 500;

type DirectoryHook = Box<dyn Fn(&str) + Send + Sync>;
type ProgressHook = Box<dyn Fn(&str, i64) + Send + Sync>;

/// Lets the caller observe a scan without the crawler knowing about watchers or
/// status reporting.
#[derive(Default)]
pub struct CrawlHooks {
    /// Fires for every directory the crawl descends into, which is where the
    /// watcher registers itself.
    pub on_directory: Option<DirectoryHook>,
    pub on_progress: Option<ProgressHook>,
}

/// Walks a directory tree into the store. Symlinks are recorded but never
/// followed, so a link loop cannot stall a scan.
pub struct Crawler {
    exclusions: Exclusions,
    hooks: CrawlHooks,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScanReport {
    pub entries_seen: i64,
    pub directories_seen: i64,
    pub stale_removed: i64,
}

impl Crawler {
    pub fn new(exclusions: Exclusions, hooks: CrawlHooks) -> Self {
        Crawler { exclusions, hooks }
    }

    /// Indexes `tree_path` and everything below it, stamping every row with the
    /// session's generation. It does not sweep; that belongs to a full root scan.
    pub fn scan_tree(
        &self,
        store: &Store,
        stop: &StopSignal,
        session: &ScanSession,
        tree_path: &str,
    ) -> IndexResult<ScanReport> {
        let mut scan = TreeScan {
            crawler: self,
            stop,
            batch: store.new_batch(session.clone()),
            tree_path,
            report: ScanReport::default(),
        };
        scan.run()?;
        scan.batch.flush()?;
        Ok(scan.report)
    }
}

struct TreeScan<'scan> {
    crawler: &'scan Crawler,
    stop: &'scan StopSignal,
    batch: Batch<'scan>,
    tree_path: &'scan str,
    report: ScanReport,
}

impl TreeScan<'_> {
    fn run(&mut self) -> IndexResult<()> {
        let metadata = fs::symlink_metadata(self.tree_path)?;
        let tree_path = self.tree_path;
        if metadata.is_dir() {
            return self.visit_directory(Path::new(tree_path), &metadata);
        }
        self.record_entry(Path::new(tree_path), &metadata)
    }

    fn visit_directory(&mut self, path: &Path, metadata: &Metadata) -> IndexResult<()> {
        self.check_not_cancelled()?;
        let path_text = path.to_string_lossy();
        if self.skips_directory(&path_text, path) {
            return Ok(());
        }
        self.report.directories_seen += 1;
        if let Some(on_directory) = &self.crawler.hooks.on_directory {
            on_directory(&path_text);
        }
        self.record_entry(path, metadata)?;
        self.visit_children(path)
    }

    /// Never rejects the tree being scanned: a user who explicitly indexes
    /// `~/code/node_modules` means it.
    fn skips_directory(&self, path_text: &str, path: &Path) -> bool {
        if path_text == self.tree_path {
            return false;
        }
        self.crawler.exclusions.skips_directory(path_text, &base_name(path))
    }

    /// A directory the user cannot read is skipped, which is normal inside a
    /// home directory. The tree root failing is fatal, because it means the
    /// caller asked to index something unusable.
    fn visit_children(&mut self, path: &Path) -> IndexResult<()> {
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(error) if path == Path::new(self.tree_path) => return Err(error.into()),
            Err(_) => return Ok(()),
        };
        for entry in entries.flatten() {
            self.visit_child(&entry)?;
        }
        Ok(())
    }

    fn visit_child(&mut self, entry: &fs::DirEntry) -> IndexResult<()> {
        self.check_not_cancelled()?;
        let Ok(metadata) = entry.metadata() else {
            return Ok(());
        };
        if metadata.is_dir() {
            return self.visit_directory(&entry.path(), &metadata);
        }
        self.record_entry(&entry.path(), &metadata)
    }

    fn record_entry(&mut self, path: &Path, metadata: &Metadata) -> IndexResult<()> {
        let record = record_from_metadata(path, metadata);
        self.report.entries_seen += 1;
        self.notify_progress(&record.path);
        self.batch.add(record)
    }

    fn notify_progress(&self, path: &str) {
        let Some(on_progress) = &self.crawler.hooks.on_progress else {
            return;
        };
        if self.report.entries_seen % PROGRESS_INTERVAL != 0 {
            return;
        }
        on_progress(path, self.report.entries_seen);
    }

    fn check_not_cancelled(&self) -> IndexResult<()> {
        if self.stop.is_stopped() {
            return Err(IndexError::Cancelled);
        }
        Ok(())
    }
}

pub(super) fn record_from_metadata(path: &Path, metadata: &Metadata) -> FileRecord {
    FileRecord {
        path: path.to_string_lossy().into_owned(),
        name: base_name(path),
        parent_path: parent_name(path),
        is_dir: metadata.is_dir(),
        size: metadata.len() as i64,
        mod_time_unix: metadata.mtime(),
    }
}

fn base_name(path: &Path) -> String {
    match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => path.to_string_lossy().into_owned(),
    }
}

fn parent_name(path: &Path) -> String {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_string_lossy().into_owned(),
        Some(_) => ".".to_string(),
        None => path.to_string_lossy().into_owned(),
    }
}
