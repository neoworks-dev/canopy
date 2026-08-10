package filesystem

import (
	"bufio"
	"os"
	"path/filepath"
	"strings"
)

// Place is a shortcut to a well-known directory. JSON tags drive the TypeScript
// bindings Wails generates for the frontend.
type Place struct {
	Name string `json:"name"`
	Path string `json:"path"`
	Kind string `json:"kind"`
}

// The sidebar order. Each entry maps an XDG key to the icon kind the frontend
// renders and to the directory name used when the system has no XDG config.
var standardPlaces = []struct {
	xdgKey       string
	kind         string
	fallbackName string
}{
	{xdgKey: "XDG_DESKTOP_DIR", kind: "desktop", fallbackName: "Desktop"},
	{xdgKey: "XDG_DOWNLOAD_DIR", kind: "downloads", fallbackName: "Downloads"},
	{xdgKey: "XDG_DOCUMENTS_DIR", kind: "documents", fallbackName: "Documents"},
	{xdgKey: "XDG_MUSIC_DIR", kind: "music", fallbackName: "Music"},
	{xdgKey: "XDG_PICTURES_DIR", kind: "pictures", fallbackName: "Pictures"},
	{xdgKey: "XDG_VIDEOS_DIR", kind: "videos", fallbackName: "Videos"},
}

// Places lists the well-known directories that exist on this machine, from the
// home directory down to the filesystem root.
func Places() []Place {
	homeDirectory, err := os.UserHomeDir()
	if err != nil {
		return []Place{rootPlace()}
	}
	places := []Place{{Name: "Home", Path: homeDirectory, Kind: "home"}}
	places = append(places, userDirectories(homeDirectory)...)
	return append(places, rootPlace())
}

func rootPlace() Place {
	return Place{Name: "Filesystem", Path: string(filepath.Separator), Kind: "root"}
}

func userDirectories(homeDirectory string) []Place {
	configured := readUserDirectoryConfig(homeDirectory)
	found := make([]Place, 0, len(standardPlaces))
	for _, standard := range standardPlaces {
		path := configured[standard.xdgKey]
		if path == "" {
			path = filepath.Join(homeDirectory, standard.fallbackName)
		}
		if isDirectory(path) {
			found = append(found, Place{Name: filepath.Base(path), Path: path, Kind: standard.kind})
		}
	}
	return found
}

// readUserDirectoryConfig parses ~/.config/user-dirs.dirs, the XDG file holding
// the localized paths of the standard directories. A missing file yields an
// empty map, and callers fall back to the English defaults.
func readUserDirectoryConfig(homeDirectory string) map[string]string {
	file, err := os.Open(filepath.Join(homeDirectory, ".config", "user-dirs.dirs"))
	if err != nil {
		return map[string]string{}
	}
	defer file.Close()

	directories := map[string]string{}
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		key, path := parseUserDirectoryLine(scanner.Text(), homeDirectory)
		if key != "" {
			directories[key] = path
		}
	}
	return directories
}

// parseUserDirectoryLine reads one XDG_xxx_DIR="…" assignment. Values are either
// absolute or written relative to $HOME. An unusable line yields an empty key.
func parseUserDirectoryLine(line string, homeDirectory string) (string, string) {
	key, rawValue, isAssignment := strings.Cut(strings.TrimSpace(line), "=")
	if !isAssignment || !strings.HasPrefix(key, "XDG_") {
		return "", ""
	}
	value := strings.Trim(rawValue, `"`)
	if strings.HasPrefix(value, "$HOME/") {
		return key, filepath.Join(homeDirectory, strings.TrimPrefix(value, "$HOME/"))
	}
	if filepath.IsAbs(value) {
		return key, value
	}
	return "", ""
}

func isDirectory(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return info.IsDir()
}
