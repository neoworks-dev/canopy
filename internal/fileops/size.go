package fileops

import (
	"io/fs"
	"os"
	"path/filepath"
)

// treeSize adds up the bytes of a file, or of every file below a directory.
// Entries it cannot read contribute nothing rather than failing the sizing:
// a total that is slightly low is better than no progress bar at all.
func treeSize(path string, handle *control) (int64, error) {
	info, err := os.Lstat(path)
	if err != nil {
		return 0, nil
	}
	if !info.IsDir() {
		return info.Size(), nil
	}
	return directoryTreeSize(path, handle)
}

func directoryTreeSize(path string, handle *control) (int64, error) {
	total := int64(0)
	err := filepath.WalkDir(path, func(_ string, entry fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return nil
		}
		total += entrySize(entry)
		return handle.waitWhilePaused()
	})
	if err != nil {
		return 0, err
	}
	return total, nil
}

func entrySize(entry fs.DirEntry) int64 {
	if entry.IsDir() {
		return 0
	}
	info, err := entry.Info()
	if err != nil {
		return 0
	}
	return info.Size()
}

// prefixSum is the byte total of the first count sources, which is where the
// counter snaps to once a source is finished.
func prefixSum(sizes []int64, count int) int64 {
	total := int64(0)
	for index := 0; index < count && index < len(sizes); index++ {
		total += sizes[index]
	}
	return total
}
