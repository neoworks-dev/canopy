package filesystem

import (
	"os"
	"path/filepath"
	"strings"
)

// ResolvePath turns user-supplied input into an absolute path. An empty path
// means the home directory, and a leading "~" is expanded.
func ResolvePath(requestedPath string) (string, error) {
	if requestedPath == "" {
		return os.UserHomeDir()
	}
	expandedPath, err := expandHome(requestedPath)
	if err != nil {
		return "", err
	}
	return filepath.Abs(expandedPath)
}

// ParentPath returns the directory containing the given path, or an empty
// string at the filesystem root where there is nowhere left to go up.
func ParentPath(absolutePath string) string {
	parent := filepath.Dir(absolutePath)
	if parent == absolutePath {
		return ""
	}
	return parent
}

// IsHidden reports whether an entry name follows the Unix dotfile convention.
func IsHidden(name string) bool {
	return strings.HasPrefix(name, ".")
}

func expandHome(path string) (string, error) {
	if !strings.HasPrefix(path, "~") {
		return path, nil
	}
	homeDirectory, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(homeDirectory, strings.TrimPrefix(path, "~")), nil
}
