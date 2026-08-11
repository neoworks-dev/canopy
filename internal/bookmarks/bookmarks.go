// Package bookmarks reads and writes the folders a user has pinned, in the file
// the GTK file choosers and every desktop file manager already share. Pinning a
// folder here puts it in their sidebars too.
package bookmarks

import (
	"net/url"
	"os"
	"path/filepath"
	"strings"
)

// Bookmark is one pinned folder. A name that is not the folder's own is what
// the file calls a label, and it is kept as written.
type Bookmark struct {
	Name string `json:"name"`
	Path string `json:"path"`
}

// List reads the pinned folders. A missing file means nothing is pinned yet,
// which is not a failure.
func List() []Bookmark {
	contents, err := os.ReadFile(bookmarksPath())
	if err != nil {
		return []Bookmark{}
	}
	return parse(string(contents))
}

// Add pins a folder, ignoring one that is already there so pinning twice is not
// an error the caller has to think about.
func Add(path string, name string) error {
	pinned := List()
	if indexOf(pinned, path) >= 0 {
		return nil
	}
	return write(append(pinned, Bookmark{Name: name, Path: path}))
}

func Remove(path string) error {
	pinned := List()
	index := indexOf(pinned, path)
	if index < 0 {
		return nil
	}
	return write(append(pinned[:index:index], pinned[index+1:]...))
}

func indexOf(pinned []Bookmark, path string) int {
	for index, bookmark := range pinned {
		if bookmark.Path == path {
			return index
		}
	}
	return -1
}

func parse(contents string) []Bookmark {
	pinned := []Bookmark{}
	for _, line := range strings.Split(contents, "\n") {
		pinned = appendParsed(pinned, line)
	}
	return pinned
}

// A line is a file URI and an optional label after a space. Anything else in
// the file — a remote URI, a comment somebody added — is left alone rather than
// shown as a folder that cannot be opened.
func appendParsed(pinned []Bookmark, line string) []Bookmark {
	uri, label, _ := strings.Cut(strings.TrimSpace(line), " ")
	path, ok := pathFromURI(uri)
	if !ok {
		return pinned
	}
	return append(pinned, Bookmark{Name: nameFor(path, label), Path: path})
}

func pathFromURI(uri string) (string, bool) {
	if !strings.HasPrefix(uri, "file://") {
		return "", false
	}
	parsed, err := url.Parse(uri)
	if err != nil || parsed.Path == "" {
		return "", false
	}
	return parsed.Path, true
}

func nameFor(path string, label string) string {
	if label != "" {
		return label
	}
	return filepath.Base(path)
}

func write(pinned []Bookmark) error {
	path := bookmarksPath()
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		return err
	}
	return writeAtomically(path, format(pinned))
}

func format(pinned []Bookmark) []byte {
	lines := make([]string, 0, len(pinned))
	for _, bookmark := range pinned {
		lines = append(lines, formatLine(bookmark))
	}
	return []byte(strings.Join(lines, "\n") + "\n")
}

// The label is only written when it differs from the folder's own name, which
// is how the other file managers write it.
func formatLine(bookmark Bookmark) string {
	uri := url.URL{Scheme: "file", Path: bookmark.Path}
	if bookmark.Name == filepath.Base(bookmark.Path) {
		return uri.String()
	}
	return uri.String() + " " + bookmark.Name
}

// Staged through a temporary file, so a file chooser reading the list while it
// is rewritten sees one version or the other.
func writeAtomically(path string, contents []byte) error {
	temporary, err := os.CreateTemp(filepath.Dir(path), ".bookmarks-*")
	if err != nil {
		return err
	}
	defer os.Remove(temporary.Name())
	if err := writeAndClose(temporary, contents); err != nil {
		return err
	}
	return os.Rename(temporary.Name(), path)
}

func writeAndClose(file *os.File, contents []byte) error {
	_, err := file.Write(contents)
	if closeErr := file.Close(); err == nil {
		err = closeErr
	}
	if err != nil {
		return err
	}
	return os.Chmod(file.Name(), 0o600)
}

func bookmarksPath() string {
	return filepath.Join(configHome(), "gtk-3.0", "bookmarks")
}

func configHome() string {
	configured := os.Getenv("XDG_CONFIG_HOME")
	if configured != "" {
		return configured
	}
	home, err := os.UserHomeDir()
	if err != nil {
		return ""
	}
	return filepath.Join(home, ".config")
}
