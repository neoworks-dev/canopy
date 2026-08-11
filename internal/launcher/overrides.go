package launcher

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sync"

	"file-explorer-neoworks/internal/config"
)

const overridesFileName = "open-with.json"

// overridesMutex serialises the read-modify-write of the override file so two
// quick "always open with" choices cannot lose one another.
var overridesMutex sync.Mutex

// SetDefault remembers an application for the MIME type of path. The choice is
// kept in this application's own config rather than rewritten into the user's
// mimeapps.list.
func SetDefault(path, applicationID string) error {
	_, err := loadApplication(applicationID)
	if err != nil {
		return err
	}
	overridesMutex.Lock()
	defer overridesMutex.Unlock()
	overrides := readOverrides()
	overrides[MimeTypeOf(path)] = applicationID
	return writeOverrides(overrides)
}

// ClearDefault drops the remembered application for the MIME type of path.
func ClearDefault(path string) error {
	overridesMutex.Lock()
	defer overridesMutex.Unlock()
	overrides := readOverrides()
	delete(overrides, MimeTypeOf(path))
	return writeOverrides(overrides)
}

func overrideFor(mimeType string) string {
	overridesMutex.Lock()
	defer overridesMutex.Unlock()
	return readOverrides()[mimeType]
}

func readOverrides() map[string]string {
	overrides := map[string]string{}
	path, err := overridesPath()
	if err != nil {
		return overrides
	}
	contents, err := os.ReadFile(path)
	if err != nil {
		return overrides
	}
	json.Unmarshal(contents, &overrides)
	return overrides
}

func writeOverrides(overrides map[string]string) error {
	path, err := overridesPath()
	if err != nil {
		return err
	}
	err = os.MkdirAll(filepath.Dir(path), 0o700)
	if err != nil {
		return err
	}
	contents, err := json.MarshalIndent(overrides, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, contents, 0o600)
}

func overridesPath() (string, error) {
	return config.Path(overridesFileName)
}
