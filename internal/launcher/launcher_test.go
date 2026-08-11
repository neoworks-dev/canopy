package launcher

import (
	"os"
	"path/filepath"
	"testing"
)

func TestSplitExecKeepsQuotedArguments(t *testing.T) {
	fields := splitExec(`/usr/bin/app --flag "one argument" %f`)
	expected := []string{"/usr/bin/app", "--flag", "one argument", "%f"}
	if len(fields) != len(expected) {
		t.Fatalf("splitExec = %q, want %q", fields, expected)
	}
	for index, field := range expected {
		if fields[index] != field {
			t.Fatalf("field %d = %q, want %q", index, fields[index], field)
		}
	}
}

func TestBuildCommandSubstitutesTheFileCode(t *testing.T) {
	argv := buildCommand("viewer %f --quiet", "/home/user/a b.png")
	expected := []string{"viewer", "/home/user/a b.png", "--quiet"}
	assertArgv(t, argv, expected)
}

func TestBuildCommandPassesAURIForTheURICode(t *testing.T) {
	argv := buildCommand("browser %u", "/home/user/a b.html")
	assertArgv(t, argv, []string{"browser", "file:///home/user/a%20b.html"})
}

func TestBuildCommandDropsUnusedFieldCodesAndAppendsThePath(t *testing.T) {
	argv := buildCommand("editor %i %c", "/home/user/note.txt")
	assertArgv(t, argv, []string{"editor", "/home/user/note.txt"})
}

func TestReadDesktopEntryReadsOnlyTheMainGroup(t *testing.T) {
	directory := t.TempDir()
	path := filepath.Join(directory, "viewer.desktop")
	contents := "[Desktop Entry]\nType=Application\nName=Viewer\nName[de]=Betrachter\nIcon=viewer\nExec=viewer %f\n\n[Desktop Action New]\nName=New Window\nExec=viewer --new\n"
	writeFile(t, path, contents)

	entry, usable := readDesktopEntry(path)
	if !usable {
		t.Fatal("a Type=Application entry with an Exec must be usable")
	}
	if entry.name != "Viewer" {
		t.Fatalf("name = %q, want the untranslated Name", entry.name)
	}
	if entry.exec != "viewer %f" {
		t.Fatalf("exec = %q, want the main group's Exec", entry.exec)
	}
}

func TestReadDesktopEntryRejectsANonApplication(t *testing.T) {
	directory := t.TempDir()
	path := filepath.Join(directory, "link.desktop")
	writeFile(t, path, "[Desktop Entry]\nType=Link\nName=Somewhere\nURL=https://example.org\n")

	_, usable := readDesktopEntry(path)
	if usable {
		t.Fatal("a Link entry cannot launch a file")
	}
}

func TestDesktopIDVariantsCoverSubdirectories(t *testing.T) {
	variants := desktopIDVariants("kde-okular.desktop")
	if len(variants) != 2 {
		t.Fatalf("variants = %q, want the flat and the nested form", variants)
	}
	if variants[1] != filepath.Join("kde", "okular.desktop") {
		t.Fatalf("nested variant = %q", variants[1])
	}
}

func TestAssociationsOnLineReadsOneMimeType(t *testing.T) {
	identifiers := associationsOnLine("image/png=viewer.desktop;editor.desktop;", "image/png")
	if len(identifiers) != 2 {
		t.Fatalf("identifiers = %q, want two entries", identifiers)
	}
	if len(associationsOnLine("image/jpeg=other.desktop;", "image/png")) != 0 {
		t.Fatal("a different MIME type must not match")
	}
}

func TestMimeTypeOfNamesDirectoriesAndKnownExtensions(t *testing.T) {
	directory := t.TempDir()
	if MimeTypeOf(directory) != directoryMimeType {
		t.Fatalf("MimeTypeOf(directory) = %q, want %q", MimeTypeOf(directory), directoryMimeType)
	}
	path := filepath.Join(directory, "page.html")
	writeFile(t, path, "<p>hi</p>")
	if MimeTypeOf(path) != "text/html" {
		t.Fatalf("MimeTypeOf(.html) = %q, want text/html", MimeTypeOf(path))
	}
}

func TestMimeTypeOfSniffsAnUnknownExtension(t *testing.T) {
	path := filepath.Join(t.TempDir(), "payload.unknownext")
	writeFile(t, path, "plain words")
	if MimeTypeOf(path) != "text/plain" {
		t.Fatalf("MimeTypeOf(unknown extension) = %q, want text/plain", MimeTypeOf(path))
	}
}

func TestOverridesRoundTripThroughTheConfigFile(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())
	t.Setenv("XDG_DATA_HOME", t.TempDir())
	applications := filepath.Join(os.Getenv("XDG_DATA_HOME"), "applications")
	mkdir(t, applications)
	writeFile(t, filepath.Join(applications, "viewer.desktop"), "[Desktop Entry]\nType=Application\nName=Viewer\nExec=viewer %f\n")
	path := filepath.Join(t.TempDir(), "image.png")
	writeFile(t, path, "x")

	err := SetDefault(path, "viewer.desktop")
	if err != nil {
		t.Fatalf("SetDefault: %v", err)
	}
	if overrideFor("image/png") != "viewer.desktop" {
		t.Fatalf("override = %q, want viewer.desktop", overrideFor("image/png"))
	}
	err = ClearDefault(path)
	if err != nil {
		t.Fatalf("ClearDefault: %v", err)
	}
	if overrideFor("image/png") != "" {
		t.Fatal("ClearDefault must drop the remembered application")
	}
}

func TestSetDefaultRefusesAnUnknownApplication(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())
	t.Setenv("XDG_DATA_HOME", t.TempDir())
	t.Setenv("XDG_DATA_DIRS", t.TempDir())
	path := filepath.Join(t.TempDir(), "image.png")
	writeFile(t, path, "x")

	err := SetDefault(path, "missing.desktop")
	if err == nil {
		t.Fatal("SetDefault must reject an application it cannot find")
	}
}

func assertArgv(t *testing.T, argv []string, expected []string) {
	t.Helper()
	if len(argv) != len(expected) {
		t.Fatalf("argv = %q, want %q", argv, expected)
	}
	for index, argument := range expected {
		if argv[index] != argument {
			t.Fatalf("argument %d = %q, want %q", index, argv[index], argument)
		}
	}
}

func writeFile(t *testing.T, path, contents string) {
	t.Helper()
	err := os.WriteFile(path, []byte(contents), 0o644)
	if err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}

func mkdir(t *testing.T, path string) {
	t.Helper()
	err := os.MkdirAll(path, 0o755)
	if err != nil {
		t.Fatalf("mkdir %s: %v", path, err)
	}
}
