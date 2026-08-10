package index

import (
	"context"
	"io/fs"
	"path/filepath"
)

// progressInterval keeps the status callback cheap on large trees.
const progressInterval = 500

// CrawlHooks let the caller observe a scan without the crawler knowing about
// watchers or status reporting.
type CrawlHooks struct {
	// OnDirectory fires for every directory the crawl descends into, which is
	// where the watcher registers itself.
	OnDirectory func(path string)
	OnProgress  func(path string, entriesSeen int64)
}

// Crawler walks a directory tree into the store. Symlinks are recorded but never
// followed, so a link loop cannot stall a scan.
type Crawler struct {
	store      *Store
	exclusions *Exclusions
	hooks      CrawlHooks
}

func NewCrawler(store *Store, exclusions *Exclusions, hooks CrawlHooks) *Crawler {
	return &Crawler{store: store, exclusions: exclusions, hooks: hooks}
}

type ScanReport struct {
	EntriesSeen     int64
	DirectoriesSeen int64
	StaleRemoved    int64
}

// ScanTree indexes treePath and everything below it, stamping every row with the
// session's generation. It does not sweep; that belongs to a full root scan.
func (crawler *Crawler) ScanTree(scanContext context.Context, session ScanSession, treePath string) (ScanReport, error) {
	state := &scanState{
		scanContext: scanContext,
		crawler:     crawler,
		batch:       crawler.store.NewBatch(session),
		treePath:    treePath,
	}
	if err := filepath.WalkDir(treePath, state.visit); err != nil {
		return state.report, err
	}
	if err := state.batch.Flush(); err != nil {
		return state.report, err
	}
	return state.report, nil
}

type scanState struct {
	scanContext context.Context
	crawler     *Crawler
	batch       *Batch
	treePath    string
	report      ScanReport
}

func (state *scanState) visit(path string, entry fs.DirEntry, walkError error) error {
	if walkError != nil {
		return skipUnreadable(path, entry, state.treePath, walkError)
	}
	if err := state.scanContext.Err(); err != nil {
		return err
	}
	if entry.IsDir() {
		return state.visitDirectory(path, entry)
	}
	return state.recordEntry(path, entry)
}

func (state *scanState) visitDirectory(path string, entry fs.DirEntry) error {
	if state.skipsDirectory(path, entry.Name()) {
		return fs.SkipDir
	}
	state.report.DirectoriesSeen++
	state.notifyDirectory(path)
	return state.recordEntry(path, entry)
}

// skipsDirectory never rejects the tree being scanned: a user who explicitly
// indexes ~/code/node_modules means it.
func (state *scanState) skipsDirectory(path string, name string) bool {
	if path == state.treePath {
		return false
	}
	return state.crawler.exclusions.SkipsDirectory(path, name)
}

// recordEntry ignores entries whose metadata disappeared mid-walk. A file
// deleted while scanning is simply absent from this generation.
func (state *scanState) recordEntry(path string, entry fs.DirEntry) error {
	record, err := buildFileRecord(path, entry)
	if err != nil {
		return nil
	}
	state.report.EntriesSeen++
	state.notifyProgress(path)
	return state.batch.Add(record)
}

func (state *scanState) notifyDirectory(path string) {
	if state.crawler.hooks.OnDirectory == nil {
		return
	}
	state.crawler.hooks.OnDirectory(path)
}

func (state *scanState) notifyProgress(path string) {
	if state.crawler.hooks.OnProgress == nil {
		return
	}
	if state.report.EntriesSeen%progressInterval != 0 {
		return
	}
	state.crawler.hooks.OnProgress(path, state.report.EntriesSeen)
}

func buildFileRecord(path string, entry fs.DirEntry) (FileRecord, error) {
	info, err := entry.Info()
	if err != nil {
		return FileRecord{}, err
	}
	return FileRecord{
		Path:        path,
		Name:        entry.Name(),
		ParentPath:  filepath.Dir(path),
		IsDir:       entry.IsDir(),
		Size:        info.Size(),
		ModTimeUnix: info.ModTime().Unix(),
	}, nil
}

// skipUnreadable keeps a crawl going past directories the user cannot read,
// which is normal inside a home directory. A failure on the tree root itself is
// fatal, because it means the caller asked to index something unusable.
func skipUnreadable(path string, entry fs.DirEntry, treePath string, walkError error) error {
	if path == treePath {
		return walkError
	}
	if entry != nil && entry.IsDir() {
		return fs.SkipDir
	}
	return nil
}
