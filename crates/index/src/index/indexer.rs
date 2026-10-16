use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use super::crawler::{CrawlHooks, Crawler, ScanReport, record_from_metadata};
use super::error::{IndexError, IndexResult};
use super::exclusions::Exclusions;
use super::search::{Query, SearchOutcome};
use super::store::{RootRecord, ScanSession, Store};
use super::watcher::Watcher;
use crate::stop::StopSignal;

const DEFAULT_MAXIMUM_WATCHES: usize = 100_000;
const DEFAULT_RESCAN_INTERVAL: Duration = Duration::from_secs(30 * 60);
const DEFAULT_QUIET_PERIOD: Duration = Duration::from_millis(300);

/// Zero values select the defaults.
#[derive(Debug, Clone, Default)]
pub struct IndexerOptions {
    pub database_path: PathBuf,
    pub maximum_watches: usize,
    pub rescan_interval: Duration,
    pub quiet_period: Duration,
}

impl IndexerOptions {
    fn with_defaults(mut self) -> Self {
        if self.maximum_watches == 0 {
            self.maximum_watches = DEFAULT_MAXIMUM_WATCHES;
        }
        if self.rescan_interval.is_zero() {
            self.rescan_interval = DEFAULT_RESCAN_INTERVAL;
        }
        if self.quiet_period.is_zero() {
            self.quiet_period = DEFAULT_QUIET_PERIOD;
        }
        self
    }
}

/// A snapshot for the status call.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    pub scanning: bool,
    pub scanning_path: String,
    pub files_indexed: i64,
    pub watched_directories: i64,
    pub watch_limit_reached: bool,
    pub last_scan_unix: i64,
}

/// Owns the index lifecycle: roots, crawls, watches, and queries. Scans are
/// serialized against each other so two roots never interleave writes.
pub struct Indexer {
    shared: Arc<Shared>,
}

struct Shared {
    store: Store,
    exclusions: Exclusions,
    crawler: Crawler,
    watcher: Arc<Watcher>,
    stop: StopSignal,
    rescan_interval: Duration,
    scan_lock: Mutex<()>,
    scanning_path: Arc<Mutex<String>>,
}

fn lock<Value>(mutex: &Mutex<Value>) -> MutexGuard<'_, Value> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Indexer {
    pub fn new(stop: StopSignal, options: IndexerOptions) -> IndexResult<Indexer> {
        let options = options.with_defaults();
        let store = Store::open(&options.database_path)?;
        let watcher = Arc::new(Watcher::new(options.maximum_watches, options.quiet_period)?);
        let scanning_path = Arc::new(Mutex::new(String::new()));
        let crawler = Crawler::new(Exclusions, build_crawl_hooks(&watcher, &scanning_path));
        let shared = Shared {
            store,
            exclusions: Exclusions,
            crawler,
            watcher,
            stop,
            rescan_interval: options.rescan_interval,
            scan_lock: Mutex::new(()),
            scanning_path,
        };
        Ok(Indexer { shared: Arc::new(shared) })
    }

    /// Launches the watcher pump, the periodic rescan, and an initial scan of
    /// every known root.
    pub fn start(&self) {
        let watcher_shared = Arc::clone(&self.shared);
        thread::spawn(move || {
            let handler_shared = Arc::clone(&watcher_shared);
            watcher_shared.watcher.run(&watcher_shared.stop, move |paths| handler_shared.handle_changes(&paths));
        });
        let rescan_shared = Arc::clone(&self.shared);
        thread::spawn(move || rescan_shared.run_periodic_rescan());
        let initial_shared = Arc::clone(&self.shared);
        thread::spawn(move || initial_shared.scan_all_roots());
    }

    /// Stops background work. The database closes once the last thread using it
    /// has finished.
    pub fn close(&self) {
        self.shared.stop.stop();
    }

    /// Registers a directory and scans it in the background, so the caller is
    /// not blocked for the length of a crawl.
    pub fn add_root(&self, requested_path: &str) -> IndexResult<()> {
        let root_path = resolve_directory(requested_path)?;
        let root_id = self.shared.store.ensure_root(&root_path)?;
        let root = RootRecord { root_id, path: root_path, ..RootRecord::default() };
        let shared = Arc::clone(&self.shared);
        thread::spawn(move || shared.scan_root(&root));
        Ok(())
    }

    pub fn remove_root(&self, requested_path: &str) -> IndexResult<()> {
        let root_path = absolute_path(requested_path)?;
        self.shared.store.remove_root(&root_path)?;
        self.shared.watcher.drop_subtree(&root_path);
        Ok(())
    }

    pub fn roots(&self) -> IndexResult<Vec<RootRecord>> {
        self.shared.store.roots()
    }

    /// Reindexes one root, or every root when `requested_path` is empty.
    pub fn rescan(&self, requested_path: &str) -> IndexResult<()> {
        let shared = Arc::clone(&self.shared);
        if requested_path.is_empty() {
            thread::spawn(move || shared.scan_all_roots());
            return Ok(());
        }
        let root = self.shared.find_root(requested_path)?;
        thread::spawn(move || shared.scan_root(&root));
        Ok(())
    }

    pub fn search(&self, query: &Query) -> IndexResult<SearchOutcome> {
        self.shared.store.search(query)
    }

    pub fn status(&self) -> IndexResult<Status> {
        let files_indexed = self.shared.store.count_files()?;
        let roots = self.shared.store.roots()?;
        let scanning_path = lock(&self.shared.scanning_path).clone();
        Ok(Status {
            scanning: !scanning_path.is_empty(),
            scanning_path,
            files_indexed,
            watched_directories: self.shared.watcher.watched_count() as i64,
            watch_limit_reached: self.shared.watcher.limit_reached(),
            last_scan_unix: latest_scan_time(&roots),
        })
    }
}

impl Drop for Indexer {
    fn drop(&mut self) {
        self.close();
    }
}

fn build_crawl_hooks(watcher: &Arc<Watcher>, scanning_path: &Arc<Mutex<String>>) -> CrawlHooks {
    let directory_watcher = Arc::clone(watcher);
    let progress_path = Arc::clone(scanning_path);
    CrawlHooks {
        on_directory: Some(Box::new(move |path| directory_watcher.add(path))),
        on_progress: Some(Box::new(move |path, _entries_seen| {
            *lock(&progress_path) = parent_directory(path);
        })),
    }
}

impl Shared {
    fn run_periodic_rescan(&self) {
        while !self.stop.wait_timeout(self.rescan_interval) {
            self.scan_all_roots();
        }
    }

    fn scan_all_roots(&self) {
        let roots = match self.store.roots() {
            Ok(roots) => roots,
            Err(error) => {
                log::error!("cannot list roots error={error}");
                return;
            }
        };
        for root in &roots {
            self.scan_root(root);
        }
    }

    fn scan_root(&self, root: &RootRecord) {
        let _serialized = lock(&self.scan_lock);
        match self.run_scan(root) {
            Ok(report) => log::info!(
                "scan finished root={} entries={} directories={} removed={}",
                root.path,
                report.entries_seen,
                report.directories_seen,
                report.stale_removed
            ),
            Err(IndexError::Cancelled) => log::debug!("scan cancelled root={}", root.path),
            Err(error) => log::error!("scan failed root={} error={error}", root.path),
        }
    }

    fn run_scan(&self, root: &RootRecord) -> IndexResult<ScanReport> {
        *lock(&self.scanning_path) = root.path.clone();
        let outcome = self.crawl_and_sweep(root);
        lock(&self.scanning_path).clear();
        outcome
    }

    fn crawl_and_sweep(&self, root: &RootRecord) -> IndexResult<ScanReport> {
        let session = self.store.begin_scan(root.root_id)?;
        let mut report = self.crawler.scan_tree(&self.store, &self.stop, &session, &root.path)?;
        report.stale_removed = self.store.finish_scan(&session)?;
        Ok(report)
    }

    /// Resolves the owning root once per batch instead of per path, which
    /// matters because an unpack or a build can produce thousands of events.
    fn handle_changes(&self, paths: &[String]) {
        let roots = match self.store.roots() {
            Ok(roots) => roots,
            Err(error) => {
                log::warn!("cannot resolve roots for change batch error={error}");
                return;
            }
        };
        for path in paths {
            self.apply_change(&roots, path);
        }
    }

    fn apply_change(&self, roots: &[RootRecord], path: &str) {
        let Some(root) = owning_root(roots, path) else {
            return;
        };
        if let Err(error) = self.apply_change_within_root(root, path) {
            log::warn!("cannot apply change path={path} error={error}");
        }
    }

    /// A failed stat is treated as a deletion, which covers both removals and
    /// renames away from the watched path.
    fn apply_change_within_root(&self, root: &RootRecord, path: &str) -> IndexResult<()> {
        let Ok(metadata) = fs::symlink_metadata(path) else {
            return self.remove_path(path);
        };
        let session = self.store.current_session(root.root_id)?;
        if metadata.is_dir() {
            return self.index_appeared_directory(&session, path);
        }
        self.store.upsert_file(&session, &record_from_metadata(Path::new(path), &metadata))
    }

    fn remove_path(&self, path: &str) -> IndexResult<()> {
        self.watcher.drop_subtree(path);
        self.store.delete_subtree(path)?;
        Ok(())
    }

    /// Scans a directory that showed up under a watched parent. Exclusions apply
    /// here too, so an npm install does not flood the index.
    fn index_appeared_directory(&self, session: &ScanSession, path: &str) -> IndexResult<()> {
        let name = Path::new(path).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        if self.exclusions.skips_directory(path, &name) {
            return Ok(());
        }
        self.crawler.scan_tree(&self.store, &self.stop, session, path)?;
        Ok(())
    }

    fn find_root(&self, requested_path: &str) -> IndexResult<RootRecord> {
        let root_path = absolute_path(requested_path)?;
        let roots = self.store.roots()?;
        roots.into_iter().find(|root| root.path == root_path).ok_or(IndexError::RootNotFound)
    }
}

fn parent_directory(path: &str) -> String {
    match Path::new(path).parent() {
        Some(parent) => parent.to_string_lossy().into_owned(),
        None => path.to_string(),
    }
}

/// Lexically cleaned absolute form, like Go's `filepath.Abs`.
fn absolute_path(requested_path: &str) -> IndexResult<String> {
    let joined = std::path::absolute(requested_path)?;
    let mut cleaned = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::ParentDir => {
                cleaned.pop();
            }
            Component::CurDir => {}
            other => cleaned.push(other),
        }
    }
    Ok(cleaned.to_string_lossy().into_owned())
}

fn resolve_directory(requested_path: &str) -> IndexResult<String> {
    let absolute = absolute_path(requested_path)?;
    let metadata = fs::metadata(&absolute)?;
    if !metadata.is_dir() {
        return Err(IndexError::NotADirectory(absolute));
    }
    Ok(absolute)
}

fn owning_root<'roots>(roots: &'roots [RootRecord], path: &str) -> Option<&'roots RootRecord> {
    roots
        .iter()
        .filter(|root| path_within_root(path, &root.path))
        .reduce(|best, candidate| if candidate.path.len() > best.path.len() { candidate } else { best })
}

fn path_within_root(path: &str, root_path: &str) -> bool {
    if path == root_path {
        return true;
    }
    path.strip_prefix(root_path).is_some_and(|rest| rest.starts_with('/'))
}

fn latest_scan_time(roots: &[RootRecord]) -> i64 {
    roots.iter().map(|root| root.last_scan_unix).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    const EVENTUAL_TIMEOUT: Duration = Duration::from_secs(10);

    struct TestIndexer {
        indexer: Indexer,
        _database_directory: tempfile::TempDir,
    }

    impl std::ops::Deref for TestIndexer {
        type Target = Indexer;

        fn deref(&self) -> &Indexer {
            &self.indexer
        }
    }

    fn start_test_indexer() -> TestIndexer {
        let database_directory = tempfile::tempdir().unwrap();
        let options = IndexerOptions {
            database_path: database_directory.path().join("index.db"),
            rescan_interval: Duration::from_secs(3600),
            quiet_period: Duration::from_millis(50),
            ..IndexerOptions::default()
        };
        let indexer = Indexer::new(StopSignal::new(), options).unwrap();
        indexer.start();
        TestIndexer { indexer, _database_directory: database_directory }
    }

    fn build_tree(relative_paths: &[&str]) -> tempfile::TempDir {
        let tree = tempfile::tempdir().unwrap();
        for relative_path in relative_paths {
            let full_path = tree.path().join(relative_path);
            fs::create_dir_all(full_path.parent().unwrap()).unwrap();
            fs::write(&full_path, relative_path).unwrap();
        }
        tree
    }

    fn tree_string(tree: &tempfile::TempDir) -> String {
        tree.path().to_string_lossy().into_owned()
    }

    fn hit_paths(indexer: &Indexer, text: &str) -> Vec<String> {
        let query = Query { text: text.to_string(), limit: 100, ..Query::default() };
        indexer.search(&query).unwrap().records.into_iter().map(|record| record.path).collect()
    }

    fn wait_until(description: &str, satisfied: impl Fn() -> bool) {
        let deadline = Instant::now() + EVENTUAL_TIMEOUT;
        while Instant::now() < deadline {
            if satisfied() {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("timed out waiting for {description}");
    }

    fn wait_for_hit(indexer: &Indexer, text: &str, expected_path: &Path) {
        let expected = expected_path.to_string_lossy().into_owned();
        wait_until(&format!("index contains {expected}"), || hit_paths(indexer, text).contains(&expected));
    }

    fn wait_for_no_hits(indexer: &Indexer, text: &str, description: &str) {
        wait_until(description, || hit_paths(indexer, text).is_empty());
    }

    #[test]
    fn indexes_a_tree_and_finds_files_by_name() {
        let tree = build_tree(&["annual_report.pdf", "notes.txt", "projects/app/main.go"]);
        let indexer = start_test_indexer();
        indexer.add_root(&tree_string(&tree)).unwrap();
        wait_for_hit(&indexer, "report", &tree.path().join("annual_report.pdf"));
        wait_for_hit(&indexer, "main", &tree.path().join("projects/app/main.go"));
    }

    #[test]
    fn skips_excluded_directories() {
        let tree = build_tree(&["keepme.txt", "node_modules/library/index.js", ".git/config"]);
        let indexer = start_test_indexer();
        indexer.add_root(&tree_string(&tree)).unwrap();
        wait_for_hit(&indexer, "keepme", &tree.path().join("keepme.txt"));
        assert!(hit_paths(&indexer, "index.js").is_empty());
        assert!(hit_paths(&indexer, "config").is_empty());
    }

    #[test]
    fn rescan_removes_deleted_files() {
        let tree = build_tree(&["temporary.txt", "permanent.txt"]);
        let indexer = start_test_indexer();
        indexer.add_root(&tree_string(&tree)).unwrap();
        wait_for_hit(&indexer, "temporary", &tree.path().join("temporary.txt"));

        fs::remove_file(tree.path().join("temporary.txt")).unwrap();
        indexer.rescan(&tree_string(&tree)).unwrap();
        wait_for_no_hits(&indexer, "temporary", "deleted file leaves the index");
        let permanent = tree.path().join("permanent.txt").to_string_lossy().into_owned();
        assert!(hit_paths(&indexer, "permanent").contains(&permanent));
    }

    #[test]
    fn watcher_picks_up_new_files() {
        let tree = build_tree(&["existing.txt"]);
        let indexer = start_test_indexer();
        indexer.add_root(&tree_string(&tree)).unwrap();
        wait_for_hit(&indexer, "existing", &tree.path().join("existing.txt"));

        let appeared_path = tree.path().join("appeared_later.txt");
        fs::write(&appeared_path, "new").unwrap();
        wait_for_hit(&indexer, "appeared", &appeared_path);
    }

    #[test]
    fn watcher_forgets_removed_directories() {
        let tree = build_tree(&["doomed/inside.txt"]);
        let indexer = start_test_indexer();
        indexer.add_root(&tree_string(&tree)).unwrap();
        wait_for_hit(&indexer, "inside", &tree.path().join("doomed/inside.txt"));

        fs::remove_dir_all(tree.path().join("doomed")).unwrap();
        wait_for_no_hits(&indexer, "inside", "removed subtree leaves the index");
    }

    #[test]
    fn add_root_rejects_a_path_that_is_not_a_directory() {
        let tree = build_tree(&["file.txt"]);
        let indexer = start_test_indexer();
        let file_path = tree.path().join("file.txt");
        assert!(indexer.add_root(&file_path.to_string_lossy()).is_err());
    }

    #[test]
    fn remove_root_drops_its_files() {
        let tree = build_tree(&["orphan.txt"]);
        let indexer = start_test_indexer();
        indexer.add_root(&tree_string(&tree)).unwrap();
        wait_for_hit(&indexer, "orphan", &tree.path().join("orphan.txt"));

        indexer.remove_root(&tree_string(&tree)).unwrap();
        wait_for_no_hits(&indexer, "orphan", "files of a removed root leave the index");
    }

    #[test]
    fn search_scope_limits_results_to_one_subtree() {
        let tree = build_tree(&["invoices/report.pdf", "invoices/nested/report.txt", "archive/report.pdf"]);
        let indexer = start_test_indexer();
        indexer.add_root(&tree_string(&tree)).unwrap();
        wait_for_hit(&indexer, "report", &tree.path().join("archive/report.pdf"));

        let scope = tree.path().join("invoices");
        let query = Query {
            text: "report".into(),
            limit: 100,
            scope: scope.to_string_lossy().into_owned(),
            ..Query::default()
        };
        let paths: Vec<String> = indexer.search(&query).unwrap().records.into_iter().map(|record| record.path).collect();
        assert_eq!(paths.len(), 2, "the two below {}", scope.display());
        assert!(paths.contains(&scope.join("nested/report.txt").to_string_lossy().into_owned()));
        assert!(!paths.contains(&tree.path().join("archive/report.pdf").to_string_lossy().into_owned()));
    }

    #[test]
    fn an_empty_scope_searches_everything() {
        let tree = build_tree(&["invoices/report.pdf", "archive/report.pdf"]);
        let indexer = start_test_indexer();
        indexer.add_root(&tree_string(&tree)).unwrap();
        wait_for_hit(&indexer, "report", &tree.path().join("archive/report.pdf"));
        wait_for_hit(&indexer, "report", &tree.path().join("invoices/report.pdf"));
        assert_eq!(hit_paths(&indexer, "report").len(), 2);
    }

    fn root_record(root_id: i64, path: &str) -> RootRecord {
        RootRecord { root_id, path: path.to_string(), ..RootRecord::default() }
    }

    #[test]
    fn owning_root_prefers_the_deepest_root() {
        let roots = [root_record(1, "/home/user"), root_record(2, "/home/user/projects")];
        let root = owning_root(&roots, "/home/user/projects/app/main.go").unwrap();
        assert_eq!(root.root_id, 2);
    }

    #[test]
    fn owning_root_rejects_sibling_prefixes() {
        let roots = [root_record(1, "/home/user/docs")];
        assert!(owning_root(&roots, "/home/user/docs-backup/file.txt").is_none());
    }

    #[test]
    fn absolute_path_cleans_dot_segments() {
        assert_eq!(absolute_path("/a/b/../c/./d").unwrap(), "/a/c/d");
    }
}
