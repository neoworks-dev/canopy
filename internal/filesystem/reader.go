package filesystem

import (
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

// ReadDirectory lists one directory. Entries are sorted directories-first, then
// case-insensitively by name.
func ReadDirectory(requestedPath string) (Listing, error) {
	absolutePath, err := ResolvePath(requestedPath)
	if err != nil {
		return Listing{}, err
	}
	directoryEntries, err := os.ReadDir(absolutePath)
	if err != nil {
		return Listing{}, err
	}
	return Listing{
		Path:       absolutePath,
		ParentPath: ParentPath(absolutePath),
		Entries:    toEntries(absolutePath, directoryEntries),
	}, nil
}

func toEntries(parentPath string, directoryEntries []os.DirEntry) []Entry {
	entries := make([]Entry, 0, len(directoryEntries))
	for _, directoryEntry := range directoryEntries {
		entries = append(entries, toEntry(parentPath, directoryEntry))
	}
	sortEntries(entries)
	return entries
}

// toEntry degrades gracefully: an entry whose metadata cannot be read (a broken
// symlink, a race with deletion) still shows up with its name and path.
func toEntry(parentPath string, directoryEntry os.DirEntry) Entry {
	entry := Entry{
		Name:  directoryEntry.Name(),
		Path:  filepath.Join(parentPath, directoryEntry.Name()),
		IsDir: directoryEntry.IsDir(),
	}
	info, err := directoryEntry.Info()
	if err != nil {
		return entry
	}
	entry.Size = info.Size()
	entry.ModTime = info.ModTime().Format(time.RFC3339)
	entry.Mode = info.Mode().String()
	return entry
}

func sortEntries(entries []Entry) {
	sort.Slice(entries, func(firstIndex, secondIndex int) bool {
		first := entries[firstIndex]
		second := entries[secondIndex]
		if first.IsDir != second.IsDir {
			return first.IsDir
		}
		return strings.ToLower(first.Name) < strings.ToLower(second.Name)
	})
}
