package tags

import (
	"io/fs"
	"path/filepath"
	"strings"

	"file-explorer-neoworks/internal/filesystem"
)

// Enough matches to work with. The tags live on the files rather than in an
// index, so finding them means walking the tree and asking each file, and a
// walk that never stops would be one nobody waits for.
const searchLimit = 500

// Directories a tagged file would not be put in on purpose, skipped so a walk
// spends its time where the user's own files are.
var skippedDirectoryNames = map[string]bool{
	"node_modules": true,
	"__pycache__":  true,
	"target":       true,
	"vendor":       true,
}

// Search lists the files under root that carry one tag.
func Search(root string, name string) []filesystem.Entry {
	found := []filesystem.Entry{}
	wanted := strings.TrimSpace(name)
	if wanted == "" {
		return found
	}
	filepath.WalkDir(root, func(path string, entry fs.DirEntry, err error) error {
		if err != nil {
			return skipUnreadable(entry)
		}
		if isSkippedDirectory(path, root, entry) {
			return fs.SkipDir
		}
		found = appendTagged(found, path, wanted)
		return stopAtLimit(found)
	})
	return found
}

// A directory that cannot be read is stepped over rather than ending the walk,
// since one unreadable folder says nothing about the rest of the tree.
func skipUnreadable(entry fs.DirEntry) error {
	if entry != nil && entry.IsDir() {
		return fs.SkipDir
	}
	return nil
}

func isSkippedDirectory(path string, root string, entry fs.DirEntry) bool {
	if !entry.IsDir() || path == root {
		return false
	}
	name := entry.Name()
	return strings.HasPrefix(name, ".") || skippedDirectoryNames[name]
}

func appendTagged(found []filesystem.Entry, path string, name string) []filesystem.Entry {
	if !Has(path, name) {
		return found
	}
	return append(found, filesystem.Describe(path))
}

func stopAtLimit(found []filesystem.Entry) error {
	if len(found) >= searchLimit {
		return fs.SkipAll
	}
	return nil
}
