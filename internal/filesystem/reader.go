package filesystem

import (
	"os"
	"path/filepath"
	"time"
)

// ReadDirectory lists one directory in the given order. Sorting stays on this
// side so it holds for the whole directory, not just the entries the listing
// currently has mounted.
func ReadDirectory(requestedPath string, order Order) (Listing, error) {
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
		Entries:    toEntries(absolutePath, directoryEntries, order),
	}, nil
}

// Describe reads one path on its own, for the panes that show entries which do
// not come from a single directory. A path whose metadata cannot be read still
// comes back with its name, the same way a listing degrades.
func Describe(path string) Entry {
	entry := Entry{Name: filepath.Base(path), Path: path}
	info, err := os.Lstat(path)
	if err != nil {
		return entry
	}
	entry.IsDir = info.IsDir()
	entry.Size = info.Size()
	entry.ModTime = info.ModTime().Format(time.RFC3339)
	entry.modifiedAt = info.ModTime()
	entry.Mode = info.Mode().String()
	return entry
}

func toEntries(parentPath string, directoryEntries []os.DirEntry, order Order) []Entry {
	entries := make([]Entry, 0, len(directoryEntries))
	for _, directoryEntry := range directoryEntries {
		entries = append(entries, toEntry(parentPath, directoryEntry))
	}
	sortEntries(entries, order)
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
	entry.modifiedAt = info.ModTime()
	entry.Mode = info.Mode().String()
	return entry
}
