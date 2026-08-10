package index

import (
	"io/fs"
	"os"
	"path/filepath"
	"testing"
)

// The index names every file the user owns, so neither the database nor the WAL
// files SQLite derives from it may be readable by other local accounts.
func TestOpenKeepsTheDatabaseAndItsWalPrivate(t *testing.T) {
	databasePath := filepath.Join(t.TempDir(), "index.db")
	store, err := Open(databasePath)
	if err != nil {
		t.Fatalf("open: %v", err)
	}
	defer store.Close()

	if _, err := store.EnsureRoot("/tmp/root"); err != nil {
		t.Fatalf("ensure root: %v", err)
	}
	assertPrivateMode(t, databasePath)
	assertPrivateMode(t, databasePath+"-wal")
}

func TestOpenTightensAnAlreadyPermissiveDatabase(t *testing.T) {
	databasePath := filepath.Join(t.TempDir(), "index.db")
	if err := os.WriteFile(databasePath, nil, 0o644); err != nil {
		t.Fatalf("create permissive database: %v", err)
	}
	store, err := Open(databasePath)
	if err != nil {
		t.Fatalf("open: %v", err)
	}
	defer store.Close()

	assertPrivateMode(t, databasePath)
}

func TestDeleteSubtreeIgnoresSiblingsWithWildcardCharacters(t *testing.T) {
	store := openTestStore(t)
	session := beginTestScan(t, store, "/tmp/root")

	paths := []string{
		"/tmp/root/100% done/report.pdf",
		"/tmp/root/100% done",
		"/tmp/root/1000 done/report.pdf",
	}
	for _, path := range paths {
		record := FileRecord{Path: path, Name: filepath.Base(path), ParentPath: filepath.Dir(path)}
		if err := store.UpsertFile(session, record); err != nil {
			t.Fatalf("upsert %s: %v", path, err)
		}
	}
	removed, err := store.DeleteSubtree("/tmp/root/100% done")
	if err != nil {
		t.Fatalf("delete subtree: %v", err)
	}
	if removed != 2 {
		t.Fatalf("removed %d rows, want the directory and its single child", removed)
	}
	remaining, err := store.CountFiles()
	if err != nil {
		t.Fatalf("count files: %v", err)
	}
	if remaining != 1 {
		t.Fatalf("%d rows left, want the sibling directory's file untouched", remaining)
	}
}

func TestFinishScanRemovesRowsFromEarlierGenerations(t *testing.T) {
	store := openTestStore(t)
	firstSession := beginTestScan(t, store, "/tmp/root")
	upsertOrFail(t, store, firstSession, "/tmp/root/stale.txt")
	upsertOrFail(t, store, firstSession, "/tmp/root/kept.txt")

	rootId := firstSession.RootId
	secondSession, err := store.BeginScan(rootId)
	if err != nil {
		t.Fatalf("begin second scan: %v", err)
	}
	upsertOrFail(t, store, secondSession, "/tmp/root/kept.txt")

	removed, err := store.FinishScan(secondSession)
	if err != nil {
		t.Fatalf("finish scan: %v", err)
	}
	if removed != 1 {
		t.Fatalf("swept %d rows, want only the file missing from the second scan", removed)
	}
}

func openTestStore(t *testing.T) *Store {
	t.Helper()
	store, err := Open(filepath.Join(t.TempDir(), "index.db"))
	if err != nil {
		t.Fatalf("open: %v", err)
	}
	t.Cleanup(func() {
		store.Close()
	})
	return store
}

func beginTestScan(t *testing.T, store *Store, rootPath string) ScanSession {
	t.Helper()
	rootId, err := store.EnsureRoot(rootPath)
	if err != nil {
		t.Fatalf("ensure root: %v", err)
	}
	session, err := store.BeginScan(rootId)
	if err != nil {
		t.Fatalf("begin scan: %v", err)
	}
	return session
}

func upsertOrFail(t *testing.T, store *Store, session ScanSession, path string) {
	t.Helper()
	record := FileRecord{Path: path, Name: filepath.Base(path), ParentPath: filepath.Dir(path)}
	if err := store.UpsertFile(session, record); err != nil {
		t.Fatalf("upsert %s: %v", path, err)
	}
}

func assertPrivateMode(t *testing.T, path string) {
	t.Helper()
	info, err := os.Stat(path)
	if err != nil {
		t.Fatalf("stat %s: %v", path, err)
	}
	if info.Mode().Perm() != fs.FileMode(0o600) {
		t.Fatalf("%s has mode %o, want 600", path, info.Mode().Perm())
	}
}
