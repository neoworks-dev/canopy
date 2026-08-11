package fileops

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

const maxNameAttempts = 10000

var (
	ErrEmptyName     = errors.New("name must not be empty")
	ErrNameSeparator = errors.New("name must not contain a path separator")
	ErrReservedName  = errors.New(`name must not be "." or ".."`)
	ErrNameExists    = errors.New("an entry with this name already exists")
)

// ValidateName rejects anything that cannot become a single directory entry.
// Callers check this before touching the filesystem.
func ValidateName(name string) error {
	if name == "" {
		return ErrEmptyName
	}
	if strings.ContainsAny(name, "/\x00") {
		return ErrNameSeparator
	}
	if name == "." || name == ".." {
		return ErrReservedName
	}
	return nil
}

// FreeName resolves a name inside directory, failing if it is taken.
func FreeName(directory, name string) (string, error) {
	err := ValidateName(name)
	if err != nil {
		return "", err
	}
	target := filepath.Join(directory, name)
	if exists(target) {
		return "", ErrNameExists
	}
	return target, nil
}

// uniqueName appends a counter until the name is free, which is what the
// "keep both" conflict resolution needs.
func uniqueName(directory, name string) (string, error) {
	stem, extension := splitExtension(name)
	for counter := 1; counter < maxNameAttempts; counter++ {
		candidate := fmt.Sprintf("%s (%d)%s", stem, counter, extension)
		if !exists(filepath.Join(directory, candidate)) {
			return candidate, nil
		}
	}
	return "", fmt.Errorf("no free name for %q in %s", name, directory)
}

// splitExtension keeps compound suffixes such as ".tar.gz" together so the
// counter lands in front of the whole suffix rather than inside it.
func splitExtension(name string) (string, string) {
	trimmed := strings.TrimPrefix(name, ".")
	dotIndex := strings.Index(trimmed, ".")
	if dotIndex < 0 {
		return name, ""
	}
	splitAt := len(name) - len(trimmed) + dotIndex
	return name[:splitAt], name[splitAt:]
}

// containsPath reports whether child sits inside parent, which is how a copy or
// move of a directory into its own subtree is caught before it starts.
func containsPath(parent, child string) bool {
	if parent == child {
		return true
	}
	return strings.HasPrefix(child, strings.TrimSuffix(parent, string(os.PathSeparator))+string(os.PathSeparator))
}

func exists(path string) bool {
	_, err := os.Lstat(path)
	return err == nil
}

func isDirectory(path string) bool {
	info, err := os.Lstat(path)
	if err != nil {
		return false
	}
	return info.IsDir()
}
