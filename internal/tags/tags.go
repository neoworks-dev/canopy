// Package tags reads and writes the labels a user puts on files.
//
// A file's tags live on the file itself, in the user.xdg.tags extended
// attribute the freedesktop tools agree on, so they travel with it when it is
// moved and are read by the other file managers. Filesystems without extended
// attributes simply cannot be tagged, which is reported rather than worked
// around.
package tags

import (
	"errors"
	"sort"
	"strings"
	"syscall"
)

const attributeName = "user.xdg.tags"

var errNameEmpty = errors.New("a tag needs a name")

// Of reports the tags on one file, sorted, empty when it has none or when the
// filesystem cannot carry them.
func Of(path string) []string {
	size, err := syscall.Getxattr(path, attributeName, nil)
	if err != nil || size <= 0 {
		return []string{}
	}
	stored := make([]byte, size)
	read, err := syscall.Getxattr(path, attributeName, stored)
	if err != nil {
		return []string{}
	}
	return parse(string(stored[:read]))
}

// Set replaces the tags on one file. An empty list removes the attribute rather
// than leaving an empty one behind.
func Set(path string, names []string) error {
	cleaned := normalize(names)
	if len(cleaned) == 0 {
		return clear(path)
	}
	return syscall.Setxattr(path, attributeName, []byte(strings.Join(cleaned, ",")), 0)
}

func Add(path string, name string) error {
	if strings.TrimSpace(name) == "" {
		return errNameEmpty
	}
	return Set(path, append(Of(path), name))
}

func Remove(path string, name string) error {
	return Set(path, without(Of(path), name))
}

// Has reports whether a file carries one tag, which is the question a search
// asks of every file it walks past.
func Has(path string, name string) bool {
	wanted := strings.TrimSpace(name)
	for _, tag := range Of(path) {
		if tag == wanted {
			return true
		}
	}
	return false
}

// A file that never had the attribute is already in the state clearing asks
// for, so its absence is not an error.
func clear(path string) error {
	err := syscall.Removexattr(path, attributeName)
	if errors.Is(err, syscall.ENODATA) {
		return nil
	}
	return err
}

func parse(stored string) []string {
	return normalize(strings.Split(stored, ","))
}

// Tags are a set: blanks are dropped, duplicates collapse, and the order is the
// one anything showing them would have to sort into anyway.
func normalize(names []string) []string {
	unique := map[string]bool{}
	for _, name := range names {
		rememberName(unique, name)
	}
	return sorted(unique)
}

func rememberName(unique map[string]bool, name string) {
	trimmed := strings.TrimSpace(name)
	if trimmed == "" {
		return
	}
	unique[trimmed] = true
}

func sorted(unique map[string]bool) []string {
	names := make([]string, 0, len(unique))
	for name := range unique {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}

func without(names []string, removed string) []string {
	wanted := strings.TrimSpace(removed)
	kept := make([]string, 0, len(names))
	for _, name := range names {
		kept = appendUnless(kept, name, wanted)
	}
	return kept
}

func appendUnless(kept []string, name string, removed string) []string {
	if name == removed {
		return kept
	}
	return append(kept, name)
}
