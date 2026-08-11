package settings

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sync"
	"time"

	"file-explorer-neoworks/internal/config"
)

const fileName = "settings.json"

// Long enough that dragging a window or clicking through columns writes once,
// short enough that a crash loses nothing anyone would notice.
const writeDelay = 500 * time.Millisecond

// Store reads and writes the settings file. Writes are debounced, because the
// frontend saves on every change, and are staged through a temporary file so an
// interrupted write cannot leave a half-written one behind.
type Store struct {
	path  string
	delay time.Duration

	mutex sync.Mutex
	// Fields a newer version of the application wrote, kept so rewriting the
	// file here does not drop them.
	unknown map[string]json.RawMessage
	pending *Settings
	timer   *time.Timer
}

// Open resolves the settings path. A store whose path cannot be resolved still
// works, it just never persists, which beats refusing to start.
func Open() *Store {
	path, err := config.Path(fileName)
	if err != nil {
		return &Store{delay: writeDelay}
	}
	return &Store{path: path, delay: writeDelay}
}

// Load reads the file, falling back to defaults when it is missing or corrupt.
func (store *Store) Load() Settings {
	if store.path == "" {
		return Defaults()
	}
	contents, err := os.ReadFile(store.path)
	if err != nil {
		return Defaults()
	}
	return store.parse(contents)
}

func (store *Store) parse(contents []byte) Settings {
	loaded := Defaults()
	err := json.Unmarshal(contents, &loaded)
	if err != nil {
		return Defaults()
	}
	store.rememberUnknownFields(contents)
	return loaded
}

func (store *Store) rememberUnknownFields(contents []byte) {
	fields := map[string]json.RawMessage{}
	err := json.Unmarshal(contents, &fields)
	if err != nil {
		return
	}
	for name := range knownFields() {
		delete(fields, name)
	}
	store.mutex.Lock()
	defer store.mutex.Unlock()
	store.unknown = fields
}

// Save schedules a write. Repeated calls inside the window collapse into one.
func (store *Store) Save(next Settings) {
	store.mutex.Lock()
	defer store.mutex.Unlock()
	store.pending = &next
	store.stopTimer()
	store.timer = time.AfterFunc(store.delay, store.writePending)
}

// Flush writes anything still pending straight away, for shutdown.
func (store *Store) Flush() error {
	store.mutex.Lock()
	store.stopTimer()
	pending := store.takePending()
	store.mutex.Unlock()
	if pending == nil {
		return nil
	}
	return store.write(*pending)
}

func (store *Store) writePending() {
	store.mutex.Lock()
	pending := store.takePending()
	store.mutex.Unlock()
	if pending == nil {
		return
	}
	// Nothing is watching a debounced write, so a failure leaves the settings as
	// they were rather than interrupting anyone.
	store.write(*pending)
}

func (store *Store) takePending() *Settings {
	pending := store.pending
	store.pending = nil
	return pending
}

func (store *Store) stopTimer() {
	if store.timer == nil {
		return
	}
	store.timer.Stop()
	store.timer = nil
}

func (store *Store) write(next Settings) error {
	if store.path == "" {
		return nil
	}
	next.Version = currentVersion
	contents, err := store.encode(next)
	if err != nil {
		return err
	}
	err = os.MkdirAll(filepath.Dir(store.path), 0o700)
	if err != nil {
		return err
	}
	return writeAtomically(store.path, contents)
}

func (store *Store) encode(next Settings) ([]byte, error) {
	fields, err := asFields(next)
	if err != nil {
		return nil, err
	}
	store.mutex.Lock()
	defer store.mutex.Unlock()
	for name, value := range store.unknown {
		fields[name] = value
	}
	return json.MarshalIndent(fields, "", "  ")
}

func writeAtomically(path string, contents []byte) error {
	temporaryPath := path + ".tmp"
	err := os.WriteFile(temporaryPath, contents, 0o600)
	if err != nil {
		return err
	}
	return os.Rename(temporaryPath, path)
}

// knownFields are the names this version writes, derived from the struct so the
// two cannot drift apart.
func knownFields() map[string]json.RawMessage {
	fields, err := asFields(Defaults())
	if err != nil {
		return map[string]json.RawMessage{}
	}
	return fields
}

func asFields(value Settings) (map[string]json.RawMessage, error) {
	encoded, err := json.Marshal(value)
	if err != nil {
		return nil, err
	}
	fields := map[string]json.RawMessage{}
	err = json.Unmarshal(encoded, &fields)
	if err != nil {
		return nil, err
	}
	return fields, nil
}
