// Package searches keeps the queries a user has saved, so a search worth
// running twice is in the sidebar rather than retyped.
//
// A saved search is the query and the folder it was confined to, not its
// results: the index moves on, and running it again is what makes it useful.
package searches

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"sync"

	"file-explorer-neoworks/internal/config"
)

const fileName = "searches.json"

// Search is one saved query. Name is what the sidebar calls it, and is what
// identifies it: two searches cannot share a name.
type Search struct {
	Name  string `json:"name"`
	Query string `json:"query"`
	// The folder the search is confined to, empty for every indexed root.
	Scope string `json:"scope"`
}

// Store is the saved searches on disk. Reads answer from memory, and every
// change is written straight back out, the way the other small stores here
// behave.
type Store struct {
	mutex sync.Mutex
	saved []Search
}

func Open() *Store {
	store := &Store{}
	store.saved = read()
	return store
}

func (store *Store) List() []Search {
	store.mutex.Lock()
	defer store.mutex.Unlock()
	return append([]Search{}, store.saved...)
}

// Save adds a search, replacing one of the same name so saving twice under a
// name the user recognises updates it rather than growing a duplicate.
func (store *Store) Save(search Search) error {
	prepared, ok := prepare(search)
	if !ok {
		return nil
	}
	store.mutex.Lock()
	defer store.mutex.Unlock()
	store.saved = withSearch(store.saved, prepared)
	return write(store.saved)
}

func (store *Store) Forget(name string) error {
	store.mutex.Lock()
	defer store.mutex.Unlock()
	store.saved = withoutName(store.saved, strings.TrimSpace(name))
	return write(store.saved)
}

// Rename keeps the query and moves it under a new name, which is what the
// sidebar's inline edit does.
func (store *Store) Rename(name string, newName string) error {
	renamed, ok := store.searchNamed(name)
	if !ok {
		return nil
	}
	renamed.Name = newName
	err := store.Forget(name)
	if err != nil {
		return err
	}
	return store.Save(renamed)
}

func (store *Store) searchNamed(name string) (Search, bool) {
	store.mutex.Lock()
	defer store.mutex.Unlock()
	for _, saved := range store.saved {
		if saved.Name == strings.TrimSpace(name) {
			return saved, true
		}
	}
	return Search{}, false
}

// A search with no name or no query is not one: it comes back as "nothing to
// save" rather than as an error the caller has to answer for.
func prepare(search Search) (Search, bool) {
	prepared := Search{
		Name:  strings.TrimSpace(search.Name),
		Query: strings.TrimSpace(search.Query),
		Scope: strings.TrimSpace(search.Scope),
	}
	if prepared.Name == "" || prepared.Query == "" {
		return Search{}, false
	}
	return prepared, true
}

func withSearch(saved []Search, search Search) []Search {
	return append(withoutName(saved, search.Name), search)
}

func withoutName(saved []Search, name string) []Search {
	kept := []Search{}
	for _, candidate := range saved {
		kept = appendUnlessNamed(kept, candidate, name)
	}
	return kept
}

func appendUnlessNamed(kept []Search, candidate Search, name string) []Search {
	if candidate.Name == name {
		return kept
	}
	return append(kept, candidate)
}

// A missing or unreadable file means nothing has been saved yet, which is not a
// failure worth reporting to somebody opening a window.
func read() []Search {
	path, err := config.Path(fileName)
	if err != nil {
		return []Search{}
	}
	contents, err := os.ReadFile(path)
	if err != nil {
		return []Search{}
	}
	return parse(contents)
}

func parse(contents []byte) []Search {
	saved := []Search{}
	err := json.Unmarshal(contents, &saved)
	if err != nil {
		return []Search{}
	}
	return keepUsable(saved)
}

func keepUsable(saved []Search) []Search {
	usable := []Search{}
	for _, candidate := range saved {
		usable = appendPrepared(usable, candidate)
	}
	return usable
}

func appendPrepared(usable []Search, candidate Search) []Search {
	prepared, ok := prepare(candidate)
	if !ok {
		return usable
	}
	return append(usable, prepared)
}

func write(saved []Search) error {
	path, err := config.Path(fileName)
	if err != nil {
		return err
	}
	err = os.MkdirAll(filepath.Dir(path), 0o700)
	if err != nil {
		return err
	}
	contents, err := json.MarshalIndent(saved, "", "\t")
	if err != nil {
		return err
	}
	return os.WriteFile(path, contents, 0o600)
}
