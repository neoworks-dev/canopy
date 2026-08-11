package filesystem

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestReadDirectorySortsDirectoriesFirstThenByName(t *testing.T) {
	root := t.TempDir()
	mustWriteFile(t, filepath.Join(root, "banana.txt"))
	mustWriteFile(t, filepath.Join(root, "Apple.txt"))
	mustMakeDir(t, filepath.Join(root, "zebra"))
	mustMakeDir(t, filepath.Join(root, "Alpha"))

	listing, err := ReadDirectory(root)
	if err != nil {
		t.Fatalf("ReadDirectory returned error: %v", err)
	}

	actual := entryNames(listing.Entries)
	expected := []string{"Alpha", "zebra", "Apple.txt", "banana.txt"}
	assertNamesEqual(t, actual, expected)
}

func TestReadDirectoryReportsSizeAndParent(t *testing.T) {
	root := t.TempDir()
	filePath := filepath.Join(root, "data.bin")
	if err := os.WriteFile(filePath, []byte("0123456789"), 0o644); err != nil {
		t.Fatalf("writing fixture failed: %v", err)
	}

	listing, err := ReadDirectory(root)
	if err != nil {
		t.Fatalf("ReadDirectory returned error: %v", err)
	}

	if listing.ParentPath != filepath.Dir(root) {
		t.Errorf("ParentPath = %q, want %q", listing.ParentPath, filepath.Dir(root))
	}
	if len(listing.Entries) != 1 {
		t.Fatalf("got %d entries, want 1", len(listing.Entries))
	}
	if listing.Entries[0].Size != 10 {
		t.Errorf("Size = %d, want 10", listing.Entries[0].Size)
	}
	if listing.Entries[0].ModTime == "" {
		t.Error("ModTime is empty, want an RFC3339 timestamp")
	}
}

func TestReadDirectoryReportsPermissions(t *testing.T) {
	root := t.TempDir()
	filePath := filepath.Join(root, "script.sh")
	if err := os.WriteFile(filePath, []byte("#!/bin/sh\n"), 0o750); err != nil {
		t.Fatalf("writing fixture failed: %v", err)
	}
	mustMakeDir(t, filepath.Join(root, "nested"))

	listing, err := ReadDirectory(root)
	if err != nil {
		t.Fatalf("ReadDirectory returned error: %v", err)
	}

	modesByName := map[string]string{}
	for _, entry := range listing.Entries {
		modesByName[entry.Name] = entry.Mode
	}
	if modesByName["script.sh"] != "-rwxr-x---" {
		t.Errorf("file mode = %q, want -rwxr-x---", modesByName["script.sh"])
	}
	if !strings.HasPrefix(modesByName["nested"], "d") {
		t.Errorf("directory mode = %q, want a leading d", modesByName["nested"])
	}
}

func TestReadDirectoryRejectsMissingPath(t *testing.T) {
	_, err := ReadDirectory(filepath.Join(t.TempDir(), "does-not-exist"))
	if err == nil {
		t.Fatal("expected an error for a missing directory, got nil")
	}
}

func TestReadDirectoryIncludesHiddenEntries(t *testing.T) {
	root := t.TempDir()
	mustWriteFile(t, filepath.Join(root, ".hidden"))
	mustWriteFile(t, filepath.Join(root, "visible"))

	listing, err := ReadDirectory(root)
	if err != nil {
		t.Fatalf("ReadDirectory returned error: %v", err)
	}

	assertNamesEqual(t, entryNames(listing.Entries), []string{".hidden", "visible"})
}

func entryNames(entries []Entry) []string {
	names := make([]string, 0, len(entries))
	for _, entry := range entries {
		names = append(names, entry.Name)
	}
	return names
}

func assertNamesEqual(t *testing.T, actual []string, expected []string) {
	t.Helper()
	if len(actual) != len(expected) {
		t.Fatalf("got %v, want %v", actual, expected)
	}
	for index := range expected {
		if actual[index] != expected[index] {
			t.Fatalf("got %v, want %v", actual, expected)
		}
	}
}

func mustWriteFile(t *testing.T, path string) {
	t.Helper()
	if err := os.WriteFile(path, []byte("x"), 0o644); err != nil {
		t.Fatalf("writing %s failed: %v", path, err)
	}
}

func mustMakeDir(t *testing.T, path string) {
	t.Helper()
	if err := os.Mkdir(path, 0o755); err != nil {
		t.Fatalf("creating %s failed: %v", path, err)
	}
}

// BenchmarkReadDirectoryOfManyEntries is the backend half of the cost of a
// directory the listing has to virtualize.
func BenchmarkReadDirectoryOfManyEntries(b *testing.B) {
	root := b.TempDir()
	for index := range 100_000 {
		path := filepath.Join(root, fmt.Sprintf("entry-%06d.txt", index))
		if err := os.WriteFile(path, nil, 0o644); err != nil {
			b.Fatalf("write %s: %v", path, err)
		}
	}

	for b.Loop() {
		if _, err := ReadDirectory(root); err != nil {
			b.Fatalf("ReadDirectory returned error: %v", err)
		}
	}
}
