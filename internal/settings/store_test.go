package settings

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

// storeIn builds a store over a throwaway config directory, writing without a
// debounce window so a test does not have to wait one out.
func storeIn(t *testing.T) *Store {
	t.Helper()
	return &Store{path: filepath.Join(t.TempDir(), "settings.json"), delay: time.Millisecond}
}

func writeFile(t *testing.T, store *Store, contents string) {
	t.Helper()
	if err := os.WriteFile(store.path, []byte(contents), 0o600); err != nil {
		t.Fatalf("write settings: %v", err)
	}
}

func readFile(t *testing.T, store *Store) map[string]json.RawMessage {
	t.Helper()
	contents, err := os.ReadFile(store.path)
	if err != nil {
		t.Fatalf("read settings: %v", err)
	}
	fields := map[string]json.RawMessage{}
	if err := json.Unmarshal(contents, &fields); err != nil {
		t.Fatalf("parse settings: %v", err)
	}
	return fields
}

func TestLoadingWithoutAFileReturnsDefaults(t *testing.T) {
	loaded := storeIn(t).Load()
	if loaded.Version != currentVersion {
		t.Fatalf("version %d, want %d", loaded.Version, currentVersion)
	}
	if loaded.Sort.Key != "name" || !loaded.Sort.DirectoriesFirst {
		t.Fatalf("sort %+v, want the default order", loaded.Sort)
	}
}

func TestSettingsSurviveAWriteAndRead(t *testing.T) {
	store := storeIn(t)
	saved := Defaults()
	saved.ViewMode = "grid"
	saved.Columns = []string{"kind", "size"}
	saved.ShowHidden = true
	saved.LastDirectory = "/tmp"
	saved.Window = Window{Width: 900, Height: 600, X: 40, Y: 20}
	store.Save(saved)

	if err := store.Flush(); err != nil {
		t.Fatalf("flush: %v", err)
	}

	loaded := storeIn(t)
	loaded.path = store.path
	reloaded := loaded.Load()
	if reloaded.ViewMode != "grid" || reloaded.ShowHidden != true {
		t.Fatalf("got %+v, want the saved view state", reloaded)
	}
	if reloaded.Window.Width != 900 || reloaded.Window.X != 40 {
		t.Fatalf("window %+v, want the saved geometry", reloaded.Window)
	}
	if len(reloaded.Columns) != 2 {
		t.Fatalf("columns %v, want two", reloaded.Columns)
	}
}

func TestACorruptFileFallsBackToDefaults(t *testing.T) {
	store := storeIn(t)
	writeFile(t, store, "{not json at all")

	loaded := store.Load()
	if loaded.Version != currentVersion || loaded.Window.Width != 1100 {
		t.Fatalf("got %+v, want defaults", loaded)
	}
}

func TestAFieldFromANewerVersionIsNotDropped(t *testing.T) {
	store := storeIn(t)
	writeFile(t, store, `{"version": 2, "viewMode": "grid", "somethingNewer": {"kept": true}}`)

	store.Load()
	store.Save(Defaults())
	if err := store.Flush(); err != nil {
		t.Fatalf("flush: %v", err)
	}

	fields := readFile(t, store)
	if _, exists := fields["somethingNewer"]; !exists {
		t.Fatalf("rewriting dropped the unknown field, file holds %v", fields)
	}
}

func TestRewritingStampsTheCurrentVersion(t *testing.T) {
	store := storeIn(t)
	writeFile(t, store, `{"version": 99}`)

	store.Load()
	store.Save(Defaults())
	if err := store.Flush(); err != nil {
		t.Fatalf("flush: %v", err)
	}

	fields := readFile(t, store)
	if string(fields["version"]) != "1" {
		t.Fatalf("version %s, want 1", fields["version"])
	}
}

func TestRepeatedSavesCollapseIntoTheLastOne(t *testing.T) {
	store := storeIn(t)
	for _, mode := range []string{"rows", "grid", "rows"} {
		next := Defaults()
		next.ViewMode = mode
		store.Save(next)
	}
	if err := store.Flush(); err != nil {
		t.Fatalf("flush: %v", err)
	}

	if store.Load().ViewMode != "rows" {
		t.Fatalf("view mode %q, want the last one saved", store.Load().ViewMode)
	}
}

func TestADebouncedSaveLandsOnItsOwn(t *testing.T) {
	store := storeIn(t)
	next := Defaults()
	next.LastDirectory = "/var"
	store.Save(next)

	deadline := time.Now().Add(time.Second)
	for time.Now().Before(deadline) {
		if store.Load().LastDirectory == "/var" {
			return
		}
		time.Sleep(5 * time.Millisecond)
	}
	t.Fatal("the debounced write never landed")
}

func TestFlushingNothingIsNotAnError(t *testing.T) {
	if err := storeIn(t).Flush(); err != nil {
		t.Fatalf("flush: %v", err)
	}
}

func TestAStoreWithoutAPathNeverWrites(t *testing.T) {
	store := &Store{delay: time.Millisecond}
	store.Save(Defaults())
	if err := store.Flush(); err != nil {
		t.Fatalf("flush: %v", err)
	}
	if store.Load().Version != currentVersion {
		t.Fatal("a store without a path should still report defaults")
	}
}

func TestAHandEditedKeymapIsReadBack(t *testing.T) {
	store := storeIn(t)
	writeFile(t, store, `{
		"version": 1,
		"keybindings": [
			{"key": "f", "command": "search.here"},
			{"key": "g g", "command": "list.move.first", "when": "listActive"}
		]
	}`)

	loaded := store.Load()
	if len(loaded.Keybindings) != 2 {
		t.Fatalf("read %d bindings, want 2", len(loaded.Keybindings))
	}
	if loaded.Keybindings[1].Key != "g g" || loaded.Keybindings[1].When != "listActive" {
		t.Fatalf("second binding %+v, want the sequence with its clause", loaded.Keybindings[1])
	}
}

// Nothing in the application authors a keymap, so a save that knows nothing
// about one must not be able to erase it.
func TestSavingDoesNotDropAHandEditedKeymap(t *testing.T) {
	store := storeIn(t)
	writeFile(t, store, `{"version": 1, "keybindings": [{"key": "f", "command": "search.here"}]}`)
	store.Load()

	saved := Defaults()
	saved.ViewMode = "grid"
	if err := store.write(saved); err != nil {
		t.Fatalf("write: %v", err)
	}

	reloaded := store.Load()
	if len(reloaded.Keybindings) != 1 || reloaded.Keybindings[0].Command != "search.here" {
		t.Fatalf("keybindings %+v, want the hand-edited one intact", reloaded.Keybindings)
	}
	if reloaded.ViewMode != "grid" {
		t.Fatalf("view mode %q, want the saved one", reloaded.ViewMode)
	}
}

// An absent keymap should not add an empty list to a file that never had one.
func TestAFileWithoutAKeymapStaysWithoutOne(t *testing.T) {
	store := storeIn(t)
	if err := store.write(Defaults()); err != nil {
		t.Fatalf("write: %v", err)
	}
	if _, present := readFile(t, store)["keybindings"]; present {
		t.Fatal("keybindings written into a file that had none")
	}
}
