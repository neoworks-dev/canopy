package bookmarks

import (
	"os"
	"path/filepath"
	"testing"
)

func TestListReadsPinnedFolders(t *testing.T) {
	writeFile(t, "file:///home/moritz/Projects\nfile:///srv/share Shared drive\n")

	pinned := List()

	if len(pinned) != 2 {
		t.Fatalf("got %d bookmarks, want 2", len(pinned))
	}
	if pinned[0].Name != "Projects" || pinned[0].Path != "/home/moritz/Projects" {
		t.Errorf("got %+v, want the folder's own name", pinned[0])
	}
	if pinned[1].Name != "Shared drive" {
		t.Errorf("name = %q, want the label from the file", pinned[1].Name)
	}
}

// The file is shared with the desktop's file choosers, which put remote
// locations in it that this explorer cannot open.
func TestListLeavesOutWhatIsNotALocalFolder(t *testing.T) {
	writeFile(t, "smb://server/share Share\n\nfile:///home/moritz/Notes\n")

	pinned := List()

	if len(pinned) != 1 || pinned[0].Path != "/home/moritz/Notes" {
		t.Fatalf("got %+v, want only the local folder", pinned)
	}
}

func TestAddPinsAFolderOnce(t *testing.T) {
	writeFile(t, "")

	if err := Add("/home/moritz/Projects", "Projects"); err != nil {
		t.Fatal(err)
	}
	if err := Add("/home/moritz/Projects", "Projects"); err != nil {
		t.Fatal(err)
	}

	if len(List()) != 1 {
		t.Fatalf("got %d bookmarks, want the folder pinned once", len(List()))
	}
}

func TestRemoveUnpinsAFolderAndLeavesTheRest(t *testing.T) {
	writeFile(t, "file:///a\nfile:///b\nfile:///c\n")

	if err := Remove("/b"); err != nil {
		t.Fatal(err)
	}

	pinned := List()
	if len(pinned) != 2 || pinned[0].Path != "/a" || pinned[1].Path != "/c" {
		t.Fatalf("got %+v, want /a and /c", pinned)
	}
}

func TestRemovingSomethingThatIsNotPinnedChangesNothing(t *testing.T) {
	writeFile(t, "file:///a\n")

	if err := Remove("/elsewhere"); err != nil {
		t.Fatal(err)
	}

	if len(List()) != 1 {
		t.Fatalf("got %+v, want the list untouched", List())
	}
}

// A label is only written when it is not the folder's own name, which is how
// the other file managers write the file.
func TestAddedFoldersSurviveAReread(t *testing.T) {
	writeFile(t, "")

	if err := Add("/srv/share", "Shared drive"); err != nil {
		t.Fatal(err)
	}
	if err := Add("/home/moritz/Notes", "Notes"); err != nil {
		t.Fatal(err)
	}

	pinned := List()
	if pinned[0].Name != "Shared drive" || pinned[1].Name != "Notes" {
		t.Fatalf("got %+v", pinned)
	}
}

func writeFile(t *testing.T, contents string) {
	t.Helper()
	configHome := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", configHome)
	directory := filepath.Join(configHome, "gtk-3.0")
	if err := os.MkdirAll(directory, 0o700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(directory, "bookmarks"), []byte(contents), 0o600); err != nil {
		t.Fatal(err)
	}
}
