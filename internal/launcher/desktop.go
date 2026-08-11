package launcher

import (
	"os"
	"path/filepath"
	"strings"
)

// Application is one desktop entry that can open a file.
type Application struct {
	ID   string `json:"id"`
	Name string `json:"name"`
	Icon string `json:"icon"`
	Exec string `json:"exec"`
}

// desktopEntry is the subset of a .desktop file that matters here.
type desktopEntry struct {
	name      string
	icon      string
	exec      string
	terminal  bool
	noDisplay bool
	isApp     bool
}

// applicationDirectories lists every applications/ directory on the XDG search
// path, most specific first, so a user override shadows a system entry.
func applicationDirectories() []string {
	directories := []string{}
	for _, dataDirectory := range dataDirectories() {
		directories = append(directories, filepath.Join(dataDirectory, "applications"))
	}
	return directories
}

func dataDirectories() []string {
	directories := []string{}
	dataHome := os.Getenv("XDG_DATA_HOME")
	if dataHome == "" {
		dataHome = defaultDataHome()
	}
	if dataHome != "" {
		directories = append(directories, dataHome)
	}
	return append(directories, systemDataDirectories()...)
}

func defaultDataHome() string {
	homeDirectory, err := os.UserHomeDir()
	if err != nil {
		return ""
	}
	return filepath.Join(homeDirectory, ".local", "share")
}

func systemDataDirectories() []string {
	raw := os.Getenv("XDG_DATA_DIRS")
	if raw == "" {
		raw = "/usr/local/share:/usr/share"
	}
	return strings.Split(raw, ":")
}

// desktopIDsFor collects the entries associated with a MIME type from both the
// mimeapps.list preferences and the mimeinfo.cache indexes, in that order.
func desktopIDsFor(mimeType string) []string {
	identifiers := []string{}
	for _, directory := range applicationDirectories() {
		identifiers = append(identifiers, associationsIn(directory, mimeType)...)
	}
	return deduplicate(identifiers)
}

func associationsIn(directory, mimeType string) []string {
	preferred := readAssociationList(filepath.Join(directory, "mimeapps.list"), mimeType)
	cached := readAssociationList(filepath.Join(directory, "mimeinfo.cache"), mimeType)
	return append(preferred, cached...)
}

// readAssociationList pulls the desktop ids a key file lists for one MIME type.
// Both mimeapps.list and mimeinfo.cache use the same "type=a.desktop;b.desktop;"
// shape, so one reader covers them.
func readAssociationList(path, mimeType string) []string {
	contents, err := os.ReadFile(path)
	if err != nil {
		return nil
	}
	identifiers := []string{}
	for _, line := range strings.Split(string(contents), "\n") {
		identifiers = append(identifiers, associationsOnLine(line, mimeType)...)
	}
	return identifiers
}

func associationsOnLine(line, mimeType string) []string {
	prefix := mimeType + "="
	if !strings.HasPrefix(strings.TrimSpace(line), prefix) {
		return nil
	}
	value := strings.TrimPrefix(strings.TrimSpace(line), prefix)
	return splitDesktopIDs(value)
}

func splitDesktopIDs(value string) []string {
	identifiers := []string{}
	for _, candidate := range strings.Split(value, ";") {
		identifiers = appendNonEmpty(identifiers, strings.TrimSpace(candidate))
	}
	return identifiers
}

func appendNonEmpty(values []string, candidate string) []string {
	if candidate == "" {
		return values
	}
	return append(values, candidate)
}

func deduplicate(values []string) []string {
	seen := map[string]bool{}
	unique := []string{}
	for _, value := range values {
		unique = appendUnseen(unique, seen, value)
	}
	return unique
}

func appendUnseen(values []string, seen map[string]bool, candidate string) []string {
	if seen[candidate] {
		return values
	}
	seen[candidate] = true
	return append(values, candidate)
}

// findDesktopFile resolves a desktop id to a file. Ids may encode a
// subdirectory with dashes, so "kde-okular.desktop" can live at "kde/okular.desktop".
func findDesktopFile(desktopID string) string {
	for _, directory := range applicationDirectories() {
		match := desktopFileIn(directory, desktopID)
		if match != "" {
			return match
		}
	}
	return ""
}

func desktopFileIn(directory, desktopID string) string {
	for _, relativePath := range desktopIDVariants(desktopID) {
		candidate := filepath.Join(directory, relativePath)
		if isRegularFile(candidate) {
			return candidate
		}
	}
	return ""
}

func desktopIDVariants(desktopID string) []string {
	variants := []string{desktopID}
	remaining := desktopID
	for strings.Contains(remaining, "-") {
		remaining = strings.Replace(remaining, "-", string(filepath.Separator), 1)
		variants = append(variants, remaining)
	}
	return variants
}

func isRegularFile(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return info.Mode().IsRegular()
}

// readDesktopEntry parses the [Desktop Entry] group, which is all that is needed
// to show an entry and launch it.
func readDesktopEntry(path string) (desktopEntry, bool) {
	contents, err := os.ReadFile(path)
	if err != nil {
		return desktopEntry{}, false
	}
	entry := desktopEntry{}
	inGroup := false
	for _, line := range strings.Split(string(contents), "\n") {
		inGroup = applyDesktopLine(&entry, strings.TrimSpace(line), inGroup)
	}
	return entry, entry.isApp && entry.exec != ""
}

// applyDesktopLine reads one line and reports whether the parser is still inside
// the [Desktop Entry] group afterwards.
func applyDesktopLine(entry *desktopEntry, line string, inGroup bool) bool {
	if strings.HasPrefix(line, "[") {
		return line == "[Desktop Entry]"
	}
	if !inGroup {
		return false
	}
	assignDesktopField(entry, line)
	return true
}

func assignDesktopField(entry *desktopEntry, line string) {
	key, value, found := strings.Cut(line, "=")
	if !found {
		return
	}
	setDesktopField(entry, strings.TrimSpace(key), strings.TrimSpace(value))
}

func setDesktopField(entry *desktopEntry, key, value string) {
	switch key {
	case "Name":
		entry.name = value
	case "Icon":
		entry.icon = value
	case "Exec":
		entry.exec = value
	case "Terminal":
		entry.terminal = value == "true"
	case "NoDisplay":
		entry.noDisplay = value == "true"
	case "Type":
		entry.isApp = value == "Application"
	}
}
