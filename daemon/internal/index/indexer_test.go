package index

import (
	"context"
	"log/slog"
	"os"
	"path/filepath"
	"testing"
	"time"
)

const eventualTimeout = 10 * time.Second

func TestIndexerIndexesATreeAndFindsFilesByName(t *testing.T) {
	treePath := buildTree(t, []string{
		"annual_report.pdf",
		"notes.txt",
		"projects/app/main.go",
	})
	indexer := startTestIndexer(t)

	if err := indexer.AddRoot(treePath); err != nil {
		t.Fatalf("add root: %v", err)
	}
	waitForHit(t, indexer, "report", filepath.Join(treePath, "annual_report.pdf"))
	waitForHit(t, indexer, "main", filepath.Join(treePath, "projects/app/main.go"))
}

func TestIndexerSkipsExcludedDirectories(t *testing.T) {
	treePath := buildTree(t, []string{
		"keepme.txt",
		"node_modules/library/index.js",
		".git/config",
	})
	indexer := startTestIndexer(t)

	if err := indexer.AddRoot(treePath); err != nil {
		t.Fatalf("add root: %v", err)
	}
	waitForHit(t, indexer, "keepme", filepath.Join(treePath, "keepme.txt"))

	assertNoHits(t, indexer, "index.js")
	assertNoHits(t, indexer, "config")
}

// A rescan must forget files that disappeared while the daemon was not watching.
func TestRescanRemovesDeletedFiles(t *testing.T) {
	treePath := buildTree(t, []string{"temporary.txt", "permanent.txt"})
	indexer := startTestIndexer(t)

	if err := indexer.AddRoot(treePath); err != nil {
		t.Fatalf("add root: %v", err)
	}
	waitForHit(t, indexer, "temporary", filepath.Join(treePath, "temporary.txt"))

	if err := os.Remove(filepath.Join(treePath, "temporary.txt")); err != nil {
		t.Fatalf("remove file: %v", err)
	}
	if err := indexer.Rescan(treePath); err != nil {
		t.Fatalf("rescan: %v", err)
	}
	waitUntil(t, "deleted file leaves the index", func() bool {
		return len(searchOrFail(t, indexer, "temporary")) == 0
	})
	assertHasHit(t, indexer, "permanent", filepath.Join(treePath, "permanent.txt"))
}

func TestWatcherPicksUpNewFiles(t *testing.T) {
	treePath := buildTree(t, []string{"existing.txt"})
	indexer := startTestIndexer(t)

	if err := indexer.AddRoot(treePath); err != nil {
		t.Fatalf("add root: %v", err)
	}
	waitForHit(t, indexer, "existing", filepath.Join(treePath, "existing.txt"))

	appearedPath := filepath.Join(treePath, "appeared_later.txt")
	if err := os.WriteFile(appearedPath, []byte("new"), 0o600); err != nil {
		t.Fatalf("write file: %v", err)
	}
	waitForHit(t, indexer, "appeared", appearedPath)
}

func TestWatcherForgetsRemovedDirectories(t *testing.T) {
	treePath := buildTree(t, []string{"doomed/inside.txt"})
	indexer := startTestIndexer(t)

	if err := indexer.AddRoot(treePath); err != nil {
		t.Fatalf("add root: %v", err)
	}
	waitForHit(t, indexer, "inside", filepath.Join(treePath, "doomed/inside.txt"))

	if err := os.RemoveAll(filepath.Join(treePath, "doomed")); err != nil {
		t.Fatalf("remove directory: %v", err)
	}
	waitUntil(t, "removed subtree leaves the index", func() bool {
		return len(searchOrFail(t, indexer, "inside")) == 0
	})
}

func TestAddRootRejectsAPathThatIsNotADirectory(t *testing.T) {
	treePath := buildTree(t, []string{"file.txt"})
	indexer := startTestIndexer(t)

	err := indexer.AddRoot(filepath.Join(treePath, "file.txt"))
	if err == nil {
		t.Fatal("expected adding a file as a root to fail")
	}
}

func TestRemoveRootDropsItsFiles(t *testing.T) {
	treePath := buildTree(t, []string{"orphan.txt"})
	indexer := startTestIndexer(t)

	if err := indexer.AddRoot(treePath); err != nil {
		t.Fatalf("add root: %v", err)
	}
	waitForHit(t, indexer, "orphan", filepath.Join(treePath, "orphan.txt"))

	if err := indexer.RemoveRoot(treePath); err != nil {
		t.Fatalf("remove root: %v", err)
	}
	waitUntil(t, "files of a removed root leave the index", func() bool {
		return len(searchOrFail(t, indexer, "orphan")) == 0
	})
}

func startTestIndexer(t *testing.T) *Indexer {
	t.Helper()
	runContext, stopRunning := context.WithCancel(context.Background())
	indexer, err := New(runContext, Options{
		DatabasePath:   filepath.Join(t.TempDir(), "index.db"),
		RescanInterval: time.Hour,
		QuietPeriod:    50 * time.Millisecond,
		Logger:         slog.New(slog.DiscardHandler),
	})
	if err != nil {
		stopRunning()
		t.Fatalf("new indexer: %v", err)
	}
	indexer.Start()
	t.Cleanup(func() {
		stopRunning()
		indexer.Close()
	})
	return indexer
}

func buildTree(t *testing.T, relativePaths []string) string {
	t.Helper()
	treePath := t.TempDir()
	for _, relativePath := range relativePaths {
		writeTreeFile(t, treePath, relativePath)
	}
	return treePath
}

func writeTreeFile(t *testing.T, treePath string, relativePath string) {
	t.Helper()
	fullPath := filepath.Join(treePath, relativePath)
	if err := os.MkdirAll(filepath.Dir(fullPath), 0o700); err != nil {
		t.Fatalf("create directory for %s: %v", relativePath, err)
	}
	if err := os.WriteFile(fullPath, []byte(relativePath), 0o600); err != nil {
		t.Fatalf("write %s: %v", relativePath, err)
	}
}

func waitForHit(t *testing.T, indexer *Indexer, query string, expectedPath string) {
	t.Helper()
	waitUntil(t, "index contains "+expectedPath, func() bool {
		return containsPath(searchOrFail(t, indexer, query), expectedPath)
	})
}

func assertHasHit(t *testing.T, indexer *Indexer, query string, expectedPath string) {
	t.Helper()
	if !containsPath(searchOrFail(t, indexer, query), expectedPath) {
		t.Fatalf("expected %q to still match %s", query, expectedPath)
	}
}

func assertNoHits(t *testing.T, indexer *Indexer, query string) {
	t.Helper()
	hits := searchOrFail(t, indexer, query)
	if len(hits) != 0 {
		t.Fatalf("expected no hits for %q, got %d (first: %s)", query, len(hits), hits[0].Path)
	}
}

func searchOrFail(t *testing.T, indexer *Indexer, query string) []FileRecord {
	t.Helper()
	hits, _, err := indexer.Search(Query{Text: query, Limit: 100})
	if err != nil {
		t.Fatalf("search %q: %v", query, err)
	}
	return hits
}

func containsPath(hits []FileRecord, expectedPath string) bool {
	for _, hit := range hits {
		if hit.Path == expectedPath {
			return true
		}
	}
	return false
}

// waitUntil polls because indexing is asynchronous by design; AddRoot returns
// before the crawl finishes.
func waitUntil(t *testing.T, description string, satisfied func() bool) {
	t.Helper()
	deadline := time.Now().Add(eventualTimeout)
	for time.Now().Before(deadline) {
		if satisfied() {
			return
		}
		time.Sleep(25 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %s", description)
}

func TestSearchScopeLimitsResultsToOneSubtree(t *testing.T) {
	treePath := buildTree(t, []string{
		"invoices/report.pdf",
		"invoices/nested/report.txt",
		"archive/report.pdf",
	})
	indexer := startTestIndexer(t)

	if err := indexer.AddRoot(treePath); err != nil {
		t.Fatalf("add root: %v", err)
	}
	waitForHit(t, indexer, "report", filepath.Join(treePath, "archive/report.pdf"))

	scope := filepath.Join(treePath, "invoices")
	hits, _, err := indexer.Search(Query{Text: "report", Limit: 100, Scope: scope})
	if err != nil {
		t.Fatalf("scoped search: %v", err)
	}
	if len(hits) != 2 {
		t.Fatalf("got %d hits, want the two below %s", len(hits), scope)
	}
	if !containsPath(hits, filepath.Join(scope, "nested/report.txt")) {
		t.Fatal("a scoped search should reach into subdirectories")
	}
	if containsPath(hits, filepath.Join(treePath, "archive/report.pdf")) {
		t.Fatal("a scoped search returned a hit from outside the scope")
	}
}

func TestAnEmptyScopeSearchesEverything(t *testing.T) {
	treePath := buildTree(t, []string{"invoices/report.pdf", "archive/report.pdf"})
	indexer := startTestIndexer(t)

	if err := indexer.AddRoot(treePath); err != nil {
		t.Fatalf("add root: %v", err)
	}
	waitForHit(t, indexer, "report", filepath.Join(treePath, "archive/report.pdf"))

	hits, _, err := indexer.Search(Query{Text: "report", Limit: 100})
	if err != nil {
		t.Fatalf("search: %v", err)
	}
	if len(hits) != 2 {
		t.Fatalf("got %d hits, want both", len(hits))
	}
}
