package filesystem

import (
	"os"
	"path/filepath"
	"testing"
)

func TestParseUserDirectoryLineExpandsHome(t *testing.T) {
	key, path := parseUserDirectoryLine(`XDG_MUSIC_DIR="$HOME/Musik"`, "/home/tester")

	if key != "XDG_MUSIC_DIR" {
		t.Fatalf("key = %q, want XDG_MUSIC_DIR", key)
	}
	if path != "/home/tester/Musik" {
		t.Fatalf("path = %q, want /home/tester/Musik", path)
	}
}

func TestParseUserDirectoryLineKeepsAbsolutePath(t *testing.T) {
	key, path := parseUserDirectoryLine(`XDG_VIDEOS_DIR="/mnt/media/videos"`, "/home/tester")

	if key != "XDG_VIDEOS_DIR" {
		t.Fatalf("key = %q, want XDG_VIDEOS_DIR", key)
	}
	if path != "/mnt/media/videos" {
		t.Fatalf("path = %q, want /mnt/media/videos", path)
	}
}

func TestParseUserDirectoryLineRejectsNonAssignments(t *testing.T) {
	unusableLines := []string{
		"# This file is written by xdg-user-dirs-update",
		"",
		`XDG_DESKTOP_DIR="relative/desktop"`,
		`SOME_OTHER_DIR="$HOME/Desktop"`,
	}

	for _, line := range unusableLines {
		key, _ := parseUserDirectoryLine(line, "/home/tester")
		if key != "" {
			t.Errorf("line %q yielded key %q, want none", line, key)
		}
	}
}

func TestUserDirectoriesPrefersConfiguredNames(t *testing.T) {
	homeDirectory := t.TempDir()
	writeUserDirectoryConfig(t, homeDirectory, `XDG_DOCUMENTS_DIR="$HOME/Dokumente"`)
	mustMakeDirectory(t, filepath.Join(homeDirectory, "Dokumente"))
	mustMakeDirectory(t, filepath.Join(homeDirectory, "Documents"))

	places := userDirectories(homeDirectory)

	documents := findPlaceByKind(places, "documents")
	if documents == nil {
		t.Fatal("no documents place found")
	}
	if documents.Name != "Dokumente" {
		t.Fatalf("name = %q, want Dokumente", documents.Name)
	}
}

func TestUserDirectoriesSkipsMissingDirectories(t *testing.T) {
	homeDirectory := t.TempDir()
	mustMakeDirectory(t, filepath.Join(homeDirectory, "Downloads"))

	places := userDirectories(homeDirectory)

	if len(places) != 1 {
		t.Fatalf("got %d places, want 1", len(places))
	}
	if places[0].Kind != "downloads" {
		t.Fatalf("kind = %q, want downloads", places[0].Kind)
	}
}

func TestPlacesStartsAtHomeAndEndsAtRoot(t *testing.T) {
	places := Places()

	if len(places) < 2 {
		t.Fatalf("got %d places, want at least home and root", len(places))
	}
	if places[0].Kind != "home" {
		t.Errorf("first kind = %q, want home", places[0].Kind)
	}
	last := places[len(places)-1]
	if last.Kind != "root" || last.Path != "/" {
		t.Errorf("last place = %+v, want the filesystem root", last)
	}
}

func writeUserDirectoryConfig(t *testing.T, homeDirectory string, contents string) {
	t.Helper()
	configDirectory := filepath.Join(homeDirectory, ".config")
	mustMakeDirectory(t, configDirectory)
	err := os.WriteFile(filepath.Join(configDirectory, "user-dirs.dirs"), []byte(contents), 0o644)
	if err != nil {
		t.Fatalf("writing user-dirs.dirs: %v", err)
	}
}

func mustMakeDirectory(t *testing.T, path string) {
	t.Helper()
	if err := os.MkdirAll(path, 0o755); err != nil {
		t.Fatalf("creating %s: %v", path, err)
	}
}

func findPlaceByKind(places []Place, kind string) *Place {
	for index := range places {
		if places[index].Kind == kind {
			return &places[index]
		}
	}
	return nil
}
