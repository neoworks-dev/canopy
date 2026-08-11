// Package recents remembers what the explorer opened, newest first, so the
// sidebar can offer it again. It records only what this application opened:
// nothing else on the system writes to it, and it says nothing about what other
// programs have touched.
package recents

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sync"
	"time"

	"file-explorer-neoworks/internal/config"
)

const fileName = "recents.json"

// Long enough to reach back past a morning's browsing, short enough that the
// sidebar stays a list rather than a history.
const limit = 40

// Entry is one thing that was opened. JSON tags drive both the file on disk and
// the TypeScript bindings Wails generates.
type Entry struct {
	Name     string `json:"name"`
	Path     string `json:"path"`
	IsDir    bool   `json:"isDir"`
	OpenedAt string `json:"openedAt"`
}

// Store holds the list and keeps the file behind it current. A store whose path
// cannot be resolved still works for the session, it just never persists.
type Store struct {
	path string

	mutex   sync.Mutex
	entries []Entry
}

func Open() *Store {
	path, err := config.Path(fileName)
	if err != nil {
		return &Store{}
	}
	return &Store{path: path, entries: read(path)}
}

// Record moves a path to the front of the list, so opening something again
// promotes it rather than repeating it.
func (store *Store) Record(path string, isDir bool) {
	store.mutex.Lock()
	if isAlreadyInFront(store.entries, path) {
		store.mutex.Unlock()
		return
	}
	store.entries = withEntryInFront(store.entries, entryFor(path, isDir))
	saving := copyOf(store.entries)
	store.mutex.Unlock()
	store.write(saving)
}

func (store *Store) List() []Entry {
	store.mutex.Lock()
	defer store.mutex.Unlock()
	return copyOf(store.entries)
}

func (store *Store) Clear() {
	store.mutex.Lock()
	store.entries = []Entry{}
	store.mutex.Unlock()
	store.write([]Entry{})
}

// Re-reading a folder the window is already sitting on is not a new visit, and
// a watched directory is re-read whenever anything in it changes.
func isAlreadyInFront(entries []Entry, path string) bool {
	if len(entries) == 0 {
		return false
	}
	return entries[0].Path == path
}

// An empty list is a list, not nothing: the frontend is handed a set it can
// walk either way.
func copyOf(entries []Entry) []Entry {
	return append(make([]Entry, 0, len(entries)), entries...)
}

func entryFor(path string, isDir bool) Entry {
	return Entry{
		Name:     filepath.Base(path),
		Path:     path,
		IsDir:    isDir,
		OpenedAt: time.Now().Format(time.RFC3339),
	}
}

func withEntryInFront(entries []Entry, recorded Entry) []Entry {
	promoted := append([]Entry{recorded}, withoutPath(entries, recorded.Path)...)
	if len(promoted) <= limit {
		return promoted
	}
	return promoted[:limit]
}

func withoutPath(entries []Entry, path string) []Entry {
	kept := make([]Entry, 0, len(entries))
	for _, entry := range entries {
		kept = appendUnlessPath(kept, entry, path)
	}
	return kept
}

func appendUnlessPath(kept []Entry, entry Entry, path string) []Entry {
	if entry.Path == path {
		return kept
	}
	return append(kept, entry)
}

func read(path string) []Entry {
	contents, err := os.ReadFile(path)
	if err != nil {
		return []Entry{}
	}
	entries := []Entry{}
	if json.Unmarshal(contents, &entries) != nil {
		return []Entry{}
	}
	return entries
}

// A list that cannot be written is not worth reporting: the session still has
// it, and the next launch simply starts from what was last saved.
func (store *Store) write(entries []Entry) {
	if store.path == "" {
		return
	}
	contents, err := json.MarshalIndent(entries, "", "  ")
	if err != nil || os.MkdirAll(filepath.Dir(store.path), 0o700) != nil {
		return
	}
	os.WriteFile(store.path, contents, 0o600)
}
