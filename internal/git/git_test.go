package git

import (
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

func TestDescribeReportsNoRepositoryOutsideAWorkingTree(t *testing.T) {
	report := Describe(t.TempDir())

	if report.IsRepo {
		t.Fatalf("a plain directory is not a repository")
	}
	if report.Statuses == nil {
		t.Fatalf("expected an empty set of statuses rather than nothing")
	}
}

func TestDescribeReportsTheCheckedOutBranch(t *testing.T) {
	root := newRepository(t)

	report := Describe(root)

	if !report.IsRepo || report.Repository.Root != root {
		t.Fatalf("got %+v, want the repository at %q", report, root)
	}
	if report.Repository.Branch != "trunk" {
		t.Fatalf("branch = %q, want trunk", report.Repository.Branch)
	}
}

func TestDescribeReportsABranchFromASubdirectory(t *testing.T) {
	root := newRepository(t)
	writeFile(t, root, "src/main.go", "package main")

	report := Describe(filepath.Join(root, "src"))

	if report.Repository.Root != root || report.Repository.Branch != "trunk" {
		t.Fatalf("got %+v", report.Repository)
	}
}

func TestDescribeMarksUntrackedAndModifiedEntries(t *testing.T) {
	root := newRepository(t)
	writeFile(t, root, "tracked.txt", "first")
	commitEverything(t, root)
	writeFile(t, root, "tracked.txt", "second")
	writeFile(t, root, "fresh.txt", "new")

	statuses := Describe(root).Statuses

	if statuses["tracked.txt"] != "modified" {
		t.Errorf("tracked.txt = %q, want modified", statuses["tracked.txt"])
	}
	if statuses["fresh.txt"] != "untracked" {
		t.Errorf("fresh.txt = %q, want untracked", statuses["fresh.txt"])
	}
}

// A folder shows what is going on underneath it, or a change buried three
// levels down would be invisible.
func TestDescribeFoldsDeepChangesIntoTheFolderHoldingThem(t *testing.T) {
	root := newRepository(t)
	writeFile(t, root, "src/deep/notes.txt", "first")
	commitEverything(t, root)
	writeFile(t, root, "src/deep/notes.txt", "second")

	if got := Describe(root).Statuses["src"]; got != "modified" {
		t.Fatalf("src = %q, want modified", got)
	}
}

func TestDescribeReportsIgnoredEntries(t *testing.T) {
	root := newRepository(t)
	writeFile(t, root, ".gitignore", "build/\n")
	writeFile(t, root, "build/output.bin", "binary")
	commitEverything(t, root)

	if got := Describe(root).Statuses["build"]; got != "ignored" {
		t.Fatalf("build = %q, want ignored", got)
	}
}

// The status is scoped to the directory being listed, so a change elsewhere in
// the repository does not label an unrelated entry.
func TestDescribeOnlyCoversTheDirectoryBeingListed(t *testing.T) {
	root := newRepository(t)
	writeFile(t, root, "src/main.go", "package main")
	writeFile(t, root, "docs/readme.md", "hello")
	commitEverything(t, root)
	writeFile(t, root, "docs/readme.md", "changed")

	statuses := Describe(filepath.Join(root, "src")).Statuses

	if len(statuses) != 0 {
		t.Fatalf("got %+v, want nothing in src to be marked", statuses)
	}
}

func TestDescribeMarksAStagedAddition(t *testing.T) {
	root := newRepository(t)
	writeFile(t, root, "seed.txt", "seed")
	commitEverything(t, root)
	writeFile(t, root, "added.txt", "added")
	run(t, root, "git", "add", "added.txt")

	if got := Describe(root).Statuses["added.txt"]; got != "added" {
		t.Fatalf("added.txt = %q, want added", got)
	}
}

func newRepository(t *testing.T) string {
	t.Helper()
	if _, err := exec.LookPath("git"); err != nil {
		t.Skip("git is not installed")
	}
	root := t.TempDir()
	run(t, root, "git", "init", "--initial-branch", "trunk")
	run(t, root, "git", "config", "user.email", "tests@example.invalid")
	run(t, root, "git", "config", "user.name", "Tests")
	return root
}

func commitEverything(t *testing.T, root string) {
	t.Helper()
	run(t, root, "git", "add", "-A")
	run(t, root, "git", "commit", "-m", "state")
}

func run(t *testing.T, directory string, name string, arguments ...string) {
	t.Helper()
	command := exec.Command(name, arguments...)
	command.Dir = directory
	if output, err := command.CombinedOutput(); err != nil {
		t.Fatalf("%s %v: %v\n%s", name, arguments, err, output)
	}
}

func writeFile(t *testing.T, root string, name string, contents string) {
	t.Helper()
	path := filepath.Join(root, name)
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(contents), 0o600); err != nil {
		t.Fatal(err)
	}
}
