package launcher

import (
	"os"
	"path/filepath"
	"testing"
)

func TestTerminalCommandPrefersTheEnvironment(t *testing.T) {
	environment := isolateTerminalLookup(t)
	environment.install(t, "faketerm")
	environment.install(t, "otherterm")
	environment.declareDefault(t, "other.desktop", "Exec=otherterm %u")
	t.Setenv("TERMINAL", "faketerm")

	assertArgv(t, terminalCommand(), []string{"faketerm"})
}

func TestTerminalCommandFallsBackToTheDesktopDefault(t *testing.T) {
	environment := isolateTerminalLookup(t)
	environment.install(t, "otherterm")
	environment.declareDefault(t, "other.desktop", "Exec=otherterm --login %u")
	t.Setenv("TERMINAL", "not-installed")

	assertArgv(t, terminalCommand(), []string{"otherterm", "--login"})
}

func TestTerminalCommandIgnoresADefaultThatIsNotInstalled(t *testing.T) {
	environment := isolateTerminalLookup(t)
	environment.declareDefault(t, "missing.desktop", "Exec=missingterm %u")

	argv := terminalCommand()
	if argv != nil {
		t.Fatalf("terminalCommand = %q, want nothing when no candidate is installed", argv)
	}
}

func TestFirstInstalledSkipsMissingCandidates(t *testing.T) {
	environment := isolateTerminalLookup(t)
	environment.install(t, "thirdterm")

	assertArgv(t, firstInstalled([]string{"missingterm", "thirdterm"}), []string{"thirdterm"})
}

func TestOpenTerminalRejectsAFile(t *testing.T) {
	path := filepath.Join(t.TempDir(), "note.txt")
	writeFile(t, path, "text")

	if OpenTerminal(path) == nil {
		t.Fatal("a file is not a working directory")
	}
}

// terminalEnvironment is the isolated PATH and XDG tree one test resolves its
// terminal from, so the developer's own terminal cannot decide the outcome.
type terminalEnvironment struct {
	binaries     string
	applications string
}

func isolateTerminalLookup(t *testing.T) terminalEnvironment {
	t.Helper()
	root := t.TempDir()
	environment := terminalEnvironment{
		binaries:     filepath.Join(root, "bin"),
		applications: filepath.Join(root, "data", "applications"),
	}
	mkdir(t, environment.binaries)
	mkdir(t, environment.applications)
	t.Setenv("PATH", environment.binaries)
	t.Setenv("TERMINAL", "")
	t.Setenv("XDG_DATA_HOME", filepath.Join(root, "data"))
	t.Setenv("XDG_DATA_DIRS", filepath.Join(root, "empty"))
	return environment
}

func (environment terminalEnvironment) install(t *testing.T, name string) {
	t.Helper()
	err := os.WriteFile(filepath.Join(environment.binaries, name), []byte("#!/bin/sh\n"), 0o755)
	if err != nil {
		t.Fatalf("install %s: %v", name, err)
	}
}

func (environment terminalEnvironment) declareDefault(t *testing.T, desktopID, execLine string) {
	t.Helper()
	writeFile(t, filepath.Join(environment.applications, "mimeapps.list"),
		"[Default Applications]\n"+terminalSchemeType+"="+desktopID+";\n")
	writeFile(t, filepath.Join(environment.applications, desktopID),
		"[Desktop Entry]\nType=Application\nName=Terminal\n"+execLine+"\n")
}
