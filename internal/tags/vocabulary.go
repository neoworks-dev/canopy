package tags

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sync"

	"canopy/internal/config"
)

const vocabularyFileName = "tags.json"

// Vocabulary is the list of tag names the sidebar offers.
//
// The tags themselves live on the files; this is only the set of names that
// have been used, which is not something the filesystem can be asked for
// without walking all of it. Removing a name here hides it from the sidebar and
// leaves every file that carries it untouched.
type Vocabulary struct {
	path string

	mutex sync.Mutex
	names []string
}

func OpenVocabulary() *Vocabulary {
	path, err := config.Path(vocabularyFileName)
	if err != nil {
		return &Vocabulary{}
	}
	return &Vocabulary{path: path, names: readVocabulary(path)}
}

func (vocabulary *Vocabulary) Names() []string {
	vocabulary.mutex.Lock()
	defer vocabulary.mutex.Unlock()
	return copyOf(vocabulary.names)
}

// Remember adds names that are now in use, which is how tagging a file for the
// first time puts its tag in the sidebar.
func (vocabulary *Vocabulary) Remember(names []string) {
	vocabulary.mutex.Lock()
	vocabulary.names = normalize(append(vocabulary.names, names...))
	saving := copyOf(vocabulary.names)
	vocabulary.mutex.Unlock()
	vocabulary.write(saving)
}

func (vocabulary *Vocabulary) Forget(name string) {
	vocabulary.mutex.Lock()
	vocabulary.names = without(vocabulary.names, name)
	saving := copyOf(vocabulary.names)
	vocabulary.mutex.Unlock()
	vocabulary.write(saving)
}

// An empty vocabulary is a list, not nothing: the sidebar is handed a set it
// can walk either way.
func copyOf(names []string) []string {
	return append(make([]string, 0, len(names)), names...)
}

func readVocabulary(path string) []string {
	contents, err := os.ReadFile(path)
	if err != nil {
		return []string{}
	}
	names := []string{}
	if json.Unmarshal(contents, &names) != nil {
		return []string{}
	}
	return normalize(names)
}

// A list that cannot be written costs the sidebar a name after the next launch,
// which is not worth failing a tagging action over.
func (vocabulary *Vocabulary) write(names []string) {
	if vocabulary.path == "" {
		return
	}
	contents, err := json.MarshalIndent(names, "", "  ")
	if err != nil || os.MkdirAll(filepath.Dir(vocabulary.path), 0o700) != nil {
		return
	}
	os.WriteFile(vocabulary.path, contents, 0o600)
}
