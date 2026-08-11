package fileops

import (
	"errors"
	"os"
	"path/filepath"
	"testing"
)

func TestValidateNameRejectsUnusableNames(t *testing.T) {
	cases := map[string]error{
		"":         ErrEmptyName,
		"a/b":      ErrNameSeparator,
		"a\x00b":   ErrNameSeparator,
		".":        ErrReservedName,
		"..":       ErrReservedName,
		"report":   nil,
		".hidden":  nil,
		"a.tar.gz": nil,
	}
	for name, expected := range cases {
		err := ValidateName(name)
		if !errors.Is(err, expected) {
			t.Fatalf("ValidateName(%q) = %v, want %v", name, err, expected)
		}
	}
}

func TestFreeNameRefusesAnExistingEntry(t *testing.T) {
	directory := t.TempDir()
	writeFile(t, filepath.Join(directory, "taken.txt"), "x")

	_, err := FreeName(directory, "taken.txt")
	if !errors.Is(err, ErrNameExists) {
		t.Fatalf("FreeName on an existing entry = %v, want ErrNameExists", err)
	}
}

func TestUniqueNameKeepsCompoundExtensions(t *testing.T) {
	directory := t.TempDir()
	writeFile(t, filepath.Join(directory, "archive.tar.gz"), "x")

	name, err := uniqueName(directory, "archive.tar.gz")
	if err != nil {
		t.Fatalf("uniqueName: %v", err)
	}
	if name != "archive (1).tar.gz" {
		t.Fatalf("uniqueName = %q, want %q", name, "archive (1).tar.gz")
	}
}

func TestUniqueNameSkipsNamesAlreadyTaken(t *testing.T) {
	directory := t.TempDir()
	writeFile(t, filepath.Join(directory, "note.txt"), "x")
	writeFile(t, filepath.Join(directory, "note (1).txt"), "x")

	name, err := uniqueName(directory, "note.txt")
	if err != nil {
		t.Fatalf("uniqueName: %v", err)
	}
	if name != "note (2).txt" {
		t.Fatalf("uniqueName = %q, want %q", name, "note (2).txt")
	}
}

func TestSplitExtensionLeavesDotfilesIntact(t *testing.T) {
	stem, extension := splitExtension(".bashrc")
	if stem != ".bashrc" || extension != "" {
		t.Fatalf("splitExtension(\".bashrc\") = %q, %q, want the whole name as the stem", stem, extension)
	}
}

func TestContainsPathDetectsASubtree(t *testing.T) {
	if !containsPath("/home/user/photos", "/home/user/photos/2026") {
		t.Fatal("a child directory must count as contained")
	}
	if containsPath("/home/user/photos", "/home/user/photos-backup") {
		t.Fatal("a sibling with a shared prefix must not count as contained")
	}
}

func writeFile(t *testing.T, path, contents string) {
	t.Helper()
	err := os.WriteFile(path, []byte(contents), 0o644)
	if err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}
