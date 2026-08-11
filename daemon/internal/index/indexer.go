package index

import (
	"context"
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

const (
	defaultMaximumWatches = 100000
	defaultRescanInterval = 30 * time.Minute
	defaultQuietPeriod    = 300 * time.Millisecond
)

type Options struct {
	DatabasePath   string
	MaximumWatches int
	RescanInterval time.Duration
	QuietPeriod    time.Duration
	Logger         *slog.Logger
}

// Status is a snapshot for the status RPC.
type Status struct {
	Scanning           bool
	ScanningPath       string
	FilesIndexed       int64
	WatchedDirectories int
	WatchLimitReached  bool
	LastScanUnix       int64
}

// Indexer owns the index lifecycle: roots, crawls, watches, and queries. Scans
// are serialized against each other so two roots never interleave writes.
type Indexer struct {
	store          *Store
	exclusions     *Exclusions
	crawler        *Crawler
	watcher        *Watcher
	logger         *slog.Logger
	runContext     context.Context
	rescanInterval time.Duration

	scanMutex sync.Mutex

	stateMutex   sync.Mutex
	scanningPath string
}

func New(runContext context.Context, options Options) (*Indexer, error) {
	applyOptionDefaults(&options)
	store, err := Open(options.DatabasePath)
	if err != nil {
		return nil, err
	}
	indexer := &Indexer{
		store:          store,
		exclusions:     DefaultExclusions(),
		logger:         options.Logger,
		runContext:     runContext,
		rescanInterval: options.RescanInterval,
	}
	if err := indexer.attachWatcher(options); err != nil {
		store.Close()
		return nil, err
	}
	return indexer, nil
}

// Start launches the watcher pump, the periodic rescan, and an initial scan of
// every known root.
func (indexer *Indexer) Start() {
	go indexer.watcher.Run(indexer.runContext)
	go indexer.runPeriodicRescan()
	go indexer.scanAllRoots()
}

func (indexer *Indexer) Close() error {
	watcherError := indexer.watcher.Close()
	storeError := indexer.store.Close()
	if watcherError != nil {
		return watcherError
	}
	return storeError
}

// AddRoot registers a directory and scans it in the background, so the caller is
// not blocked for the length of a crawl.
func (indexer *Indexer) AddRoot(requestedPath string) error {
	rootPath, err := resolveDirectory(requestedPath)
	if err != nil {
		return err
	}
	rootId, err := indexer.store.EnsureRoot(rootPath)
	if err != nil {
		return err
	}
	go indexer.scanRoot(RootRecord{RootId: rootId, Path: rootPath})
	return nil
}

func (indexer *Indexer) RemoveRoot(requestedPath string) error {
	rootPath, err := filepath.Abs(requestedPath)
	if err != nil {
		return err
	}
	if err := indexer.store.RemoveRoot(rootPath); err != nil {
		return err
	}
	indexer.watcher.DropSubtree(rootPath)
	return nil
}

func (indexer *Indexer) Roots() ([]RootRecord, error) {
	return indexer.store.Roots()
}

// Rescan reindexes one root, or every root when requestedPath is empty.
func (indexer *Indexer) Rescan(requestedPath string) error {
	if requestedPath == "" {
		go indexer.scanAllRoots()
		return nil
	}
	root, err := indexer.findRoot(requestedPath)
	if err != nil {
		return err
	}
	go indexer.scanRoot(root)
	return nil
}

func (indexer *Indexer) Search(query Query) ([]FileRecord, bool, error) {
	return indexer.store.Search(query)
}

func (indexer *Indexer) Status() (Status, error) {
	filesIndexed, err := indexer.store.CountFiles()
	if err != nil {
		return Status{}, err
	}
	roots, err := indexer.store.Roots()
	if err != nil {
		return Status{}, err
	}
	scanningPath := indexer.currentScanningPath()
	return Status{
		Scanning:           scanningPath != "",
		ScanningPath:       scanningPath,
		FilesIndexed:       filesIndexed,
		WatchedDirectories: indexer.watcher.WatchedCount(),
		WatchLimitReached:  indexer.watcher.LimitReached(),
		LastScanUnix:       latestScanTime(roots),
	}, nil
}

func (indexer *Indexer) attachWatcher(options Options) error {
	watcher, err := NewWatcher(options.MaximumWatches, options.QuietPeriod, options.Logger, indexer.handleChanges)
	if err != nil {
		return err
	}
	indexer.watcher = watcher
	indexer.crawler = NewCrawler(indexer.store, indexer.exclusions, CrawlHooks{
		OnDirectory: watcher.Add,
		OnProgress:  indexer.reportProgress,
	})
	return nil
}

func (indexer *Indexer) runPeriodicRescan() {
	ticker := time.NewTicker(indexer.rescanInterval)
	defer ticker.Stop()

	for waitForTick(indexer.runContext, ticker.C) {
		indexer.scanAllRoots()
	}
}

func (indexer *Indexer) scanAllRoots() {
	roots, err := indexer.store.Roots()
	if err != nil {
		indexer.logger.Error("cannot list roots", "error", err)
		return
	}
	for _, root := range roots {
		indexer.scanRoot(root)
	}
}

func (indexer *Indexer) scanRoot(root RootRecord) {
	indexer.scanMutex.Lock()
	defer indexer.scanMutex.Unlock()

	report, err := indexer.runScan(root)
	if err != nil {
		indexer.logger.Error("scan failed", "root", root.Path, "error", err)
		return
	}
	indexer.logger.Info("scan finished",
		"root", root.Path,
		"entries", report.EntriesSeen,
		"directories", report.DirectoriesSeen,
		"removed", report.StaleRemoved)
}

func (indexer *Indexer) runScan(root RootRecord) (ScanReport, error) {
	indexer.setScanningPath(root.Path)
	defer indexer.setScanningPath("")

	session, err := indexer.store.BeginScan(root.RootId)
	if err != nil {
		return ScanReport{}, err
	}
	report, err := indexer.crawler.ScanTree(indexer.runContext, session, root.Path)
	if err != nil {
		return report, err
	}
	report.StaleRemoved, err = indexer.store.FinishScan(session)
	return report, err
}

// handleChanges resolves the owning root once per batch instead of per path,
// which matters because an unpack or a build can produce thousands of events.
func (indexer *Indexer) handleChanges(paths []string) {
	roots, err := indexer.store.Roots()
	if err != nil {
		indexer.logger.Warn("cannot resolve roots for change batch", "error", err)
		return
	}
	for _, path := range paths {
		indexer.applyChange(roots, path)
	}
}

func (indexer *Indexer) applyChange(roots []RootRecord, path string) {
	root, found := owningRoot(roots, path)
	if !found {
		return
	}
	if err := indexer.applyChangeWithinRoot(root, path); err != nil {
		indexer.logger.Warn("cannot apply change", "path", path, "error", err)
	}
}

// applyChangeWithinRoot treats a failed stat as a deletion, which covers both
// removals and renames away from the watched path.
func (indexer *Indexer) applyChangeWithinRoot(root RootRecord, path string) error {
	info, err := os.Lstat(path)
	if err != nil {
		return indexer.removePath(path)
	}
	session, err := indexer.store.CurrentSession(root.RootId)
	if err != nil {
		return err
	}
	if info.IsDir() {
		return indexer.indexAppearedDirectory(session, path)
	}
	return indexer.store.UpsertFile(session, recordFromFileInfo(path, info))
}

func (indexer *Indexer) removePath(path string) error {
	indexer.watcher.DropSubtree(path)
	_, err := indexer.store.DeleteSubtree(path)
	return err
}

// indexAppearedDirectory scans a directory that showed up under a watched
// parent. Exclusions apply here too, so an npm install does not flood the index.
func (indexer *Indexer) indexAppearedDirectory(session ScanSession, path string) error {
	if indexer.exclusions.SkipsDirectory(path, filepath.Base(path)) {
		return nil
	}
	_, err := indexer.crawler.ScanTree(indexer.runContext, session, path)
	return err
}

func (indexer *Indexer) findRoot(requestedPath string) (RootRecord, error) {
	rootPath, err := filepath.Abs(requestedPath)
	if err != nil {
		return RootRecord{}, err
	}
	roots, err := indexer.store.Roots()
	if err != nil {
		return RootRecord{}, err
	}
	for _, root := range roots {
		if root.Path == rootPath {
			return root, nil
		}
	}
	return RootRecord{}, ErrRootNotFound
}

func (indexer *Indexer) reportProgress(path string, entriesSeen int64) {
	indexer.setScanningPath(filepath.Dir(path))
}

func (indexer *Indexer) setScanningPath(path string) {
	indexer.stateMutex.Lock()
	defer indexer.stateMutex.Unlock()
	indexer.scanningPath = path
}

func (indexer *Indexer) currentScanningPath() string {
	indexer.stateMutex.Lock()
	defer indexer.stateMutex.Unlock()
	return indexer.scanningPath
}

func applyOptionDefaults(options *Options) {
	if options.MaximumWatches <= 0 {
		options.MaximumWatches = defaultMaximumWatches
	}
	if options.RescanInterval <= 0 {
		options.RescanInterval = defaultRescanInterval
	}
	if options.QuietPeriod <= 0 {
		options.QuietPeriod = defaultQuietPeriod
	}
	if options.Logger == nil {
		options.Logger = slog.New(slog.DiscardHandler)
	}
}

// waitForTick blocks until the next tick, reporting false once the context ends.
func waitForTick(runContext context.Context, tick <-chan time.Time) bool {
	select {
	case <-runContext.Done():
		return false
	case <-tick:
		return true
	}
}

func resolveDirectory(requestedPath string) (string, error) {
	absolutePath, err := filepath.Abs(requestedPath)
	if err != nil {
		return "", err
	}
	info, err := os.Stat(absolutePath)
	if err != nil {
		return "", err
	}
	if !info.IsDir() {
		return "", fmt.Errorf("index: %s is not a directory", absolutePath)
	}
	return absolutePath, nil
}

func recordFromFileInfo(path string, info os.FileInfo) FileRecord {
	return FileRecord{
		Path:        path,
		Name:        filepath.Base(path),
		ParentPath:  filepath.Dir(path),
		IsDir:       info.IsDir(),
		Size:        info.Size(),
		ModTimeUnix: info.ModTime().Unix(),
	}
}

func owningRoot(roots []RootRecord, path string) (RootRecord, bool) {
	var best RootRecord
	found := false
	for _, root := range roots {
		best, found = preferDeeperRoot(best, found, root, path)
	}
	return best, found
}

// preferDeeperRoot keeps the most specific root containing path, so nested roots
// attribute a change to the inner one.
func preferDeeperRoot(best RootRecord, found bool, candidate RootRecord, path string) (RootRecord, bool) {
	if !pathWithinRoot(path, candidate.Path) {
		return best, found
	}
	if found && len(best.Path) >= len(candidate.Path) {
		return best, found
	}
	return candidate, true
}

func pathWithinRoot(path string, rootPath string) bool {
	if path == rootPath {
		return true
	}
	return strings.HasPrefix(path, rootPath+string(filepath.Separator))
}

func latestScanTime(roots []RootRecord) int64 {
	var latest int64
	for _, root := range roots {
		if root.LastScanUnix > latest {
			latest = root.LastScanUnix
		}
	}
	return latest
}
