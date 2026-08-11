package fileops

import (
	"os"
	"path/filepath"
	"sync"
	"testing"
	"time"
)

const finishTimeout = 5 * time.Second

// harness collects the events one engine emits and answers conflict prompts the
// way a test asks it to.
type harness struct {
	engine     *Engine
	finished   chan Progress
	onConflict func(engine *Engine, prompt ConflictPrompt)

	mutex     sync.Mutex
	conflicts []ConflictPrompt
}

func newHarness(onConflict func(engine *Engine, prompt ConflictPrompt)) *harness {
	testHarness := &harness{finished: make(chan Progress, 1), onConflict: onConflict}
	testHarness.engine = New(testHarness.emit)
	return testHarness
}

func (testHarness *harness) emit(name string, data any) {
	if name == EventConflict {
		testHarness.handleConflict(data.(ConflictPrompt))
		return
	}
	if name == EventFinished {
		testHarness.finished <- data.(Progress)
	}
}

func (testHarness *harness) handleConflict(prompt ConflictPrompt) {
	testHarness.mutex.Lock()
	testHarness.conflicts = append(testHarness.conflicts, prompt)
	testHarness.mutex.Unlock()
	testHarness.onConflict(testHarness.engine, prompt)
}

func (testHarness *harness) conflictCount() int {
	testHarness.mutex.Lock()
	defer testHarness.mutex.Unlock()
	return len(testHarness.conflicts)
}

func (testHarness *harness) run(t *testing.T, kind Kind, sources []string, destination string) Progress {
	t.Helper()
	_, err := testHarness.engine.Enqueue(kind, sources, destination)
	if err != nil {
		t.Fatalf("Enqueue: %v", err)
	}
	select {
	case progress := <-testHarness.finished:
		return progress
	case <-time.After(finishTimeout):
		t.Fatal("operation did not finish in time")
	}
	return Progress{}
}

func answerWith(resolution Resolution, applyToAll bool) func(*Engine, ConflictPrompt) {
	return func(engine *Engine, prompt ConflictPrompt) {
		engine.Resolve(prompt.OperationID, resolution, applyToAll)
	}
}

func refuseConflicts(t *testing.T) func(*Engine, ConflictPrompt) {
	return func(engine *Engine, prompt ConflictPrompt) {
		t.Errorf("unexpected conflict prompt for %s", prompt.DestinationPath)
		engine.Resolve(prompt.OperationID, ResolutionCancel, false)
	}
}

func TestCopyDuplicatesATreeWithItsPermissions(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	tree := filepath.Join(source, "project")
	mkdir(t, filepath.Join(tree, "nested"))
	writeFile(t, filepath.Join(tree, "nested", "script.sh"), "echo hi")
	chmod(t, filepath.Join(tree, "nested", "script.sh"), 0o755)

	testHarness := newHarness(refuseConflicts(t))
	progress := testHarness.run(t, KindCopy, []string{tree}, destination)

	if len(progress.Failures) != 0 {
		t.Fatalf("copy reported failures: %v", progress.Failures)
	}
	copied := filepath.Join(destination, "project", "nested", "script.sh")
	if readFile(t, copied) != "echo hi" {
		t.Fatal("nested file was not copied")
	}
	if modeOf(t, copied) != 0o755 {
		t.Fatalf("copied mode = %o, want 755", modeOf(t, copied))
	}
	if readFile(t, filepath.Join(tree, "nested", "script.sh")) != "echo hi" {
		t.Fatal("copy must leave the source in place")
	}
}

func TestCopyKeepsBothOnConflict(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	writeFile(t, filepath.Join(source, "note.txt"), "new")
	writeFile(t, filepath.Join(destination, "note.txt"), "old")

	testHarness := newHarness(answerWith(ResolutionKeepBoth, false))
	testHarness.run(t, KindCopy, []string{filepath.Join(source, "note.txt")}, destination)

	if readFile(t, filepath.Join(destination, "note.txt")) != "old" {
		t.Fatal("keep both must not touch the existing file")
	}
	if readFile(t, filepath.Join(destination, "note (1).txt")) != "new" {
		t.Fatal("keep both must write the incoming file beside it")
	}
}

func TestCopyReplaceOverwritesTheDestination(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	writeFile(t, filepath.Join(source, "note.txt"), "new")
	writeFile(t, filepath.Join(destination, "note.txt"), "old")

	testHarness := newHarness(answerWith(ResolutionReplace, false))
	testHarness.run(t, KindCopy, []string{filepath.Join(source, "note.txt")}, destination)

	if readFile(t, filepath.Join(destination, "note.txt")) != "new" {
		t.Fatal("replace must overwrite the destination")
	}
}

func TestCopySkipLeavesTheDestinationAlone(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	writeFile(t, filepath.Join(source, "note.txt"), "new")
	writeFile(t, filepath.Join(destination, "note.txt"), "old")

	testHarness := newHarness(answerWith(ResolutionSkip, false))
	progress := testHarness.run(t, KindCopy, []string{filepath.Join(source, "note.txt")}, destination)

	if readFile(t, filepath.Join(destination, "note.txt")) != "old" {
		t.Fatal("skip must leave the destination untouched")
	}
	if len(progress.Failures) != 0 {
		t.Fatalf("a skip is not a failure, got %v", progress.Failures)
	}
}

func TestApplyToAllAsksOnlyOnce(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	for _, name := range []string{"first.txt", "second.txt"} {
		writeFile(t, filepath.Join(source, name), "new")
		writeFile(t, filepath.Join(destination, name), "old")
	}

	testHarness := newHarness(answerWith(ResolutionReplace, true))
	testHarness.run(t, KindCopy, []string{filepath.Join(source, "first.txt"), filepath.Join(source, "second.txt")}, destination)

	if testHarness.conflictCount() != 1 {
		t.Fatalf("prompted %d times, want 1", testHarness.conflictCount())
	}
	if readFile(t, filepath.Join(destination, "second.txt")) != "new" {
		t.Fatal("the blanket answer must apply to later conflicts")
	}
}

func TestDirectoriesMergeWithoutPrompting(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	mkdir(t, filepath.Join(source, "shared"))
	writeFile(t, filepath.Join(source, "shared", "incoming.txt"), "new")
	mkdir(t, filepath.Join(destination, "shared"))
	writeFile(t, filepath.Join(destination, "shared", "existing.txt"), "old")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindCopy, []string{filepath.Join(source, "shared")}, destination)

	if readFile(t, filepath.Join(destination, "shared", "existing.txt")) != "old" {
		t.Fatal("merging must keep entries the source does not carry")
	}
	if readFile(t, filepath.Join(destination, "shared", "incoming.txt")) != "new" {
		t.Fatal("merging must add the entries the source carries")
	}
}

func TestMoveRelocatesAndUndoBringsItBack(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	original := filepath.Join(source, "note.txt")
	writeFile(t, original, "body")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindMove, []string{original}, destination)

	if exists(original) {
		t.Fatal("move must remove the source")
	}
	if readFile(t, filepath.Join(destination, "note.txt")) != "body" {
		t.Fatal("move must place the file at the destination")
	}

	label, err := testHarness.engine.Undo()
	if err != nil {
		t.Fatalf("Undo: %v", err)
	}
	if label == "" {
		t.Fatal("undo must describe what it reversed")
	}
	if readFile(t, original) != "body" {
		t.Fatal("undo of a move must put the file back")
	}
}

func TestUndoOfACopyRemovesTheCopies(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	writeFile(t, filepath.Join(source, "note.txt"), "body")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindCopy, []string{filepath.Join(source, "note.txt")}, destination)

	_, err := testHarness.engine.Undo()
	if err != nil {
		t.Fatalf("Undo: %v", err)
	}
	if exists(filepath.Join(destination, "note.txt")) {
		t.Fatal("undo of a copy must remove the copy")
	}
	if !exists(filepath.Join(source, "note.txt")) {
		t.Fatal("undo of a copy must not touch the source")
	}
}

func TestPermanentDeletionIsNotUndoable(t *testing.T) {
	directory := t.TempDir()
	writeFile(t, filepath.Join(directory, "note.txt"), "body")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindDelete, []string{filepath.Join(directory, "note.txt")}, "")

	if exists(filepath.Join(directory, "note.txt")) {
		t.Fatal("delete must remove the file")
	}
	if testHarness.engine.UndoLabel() != "" {
		t.Fatal("a permanent deletion must not offer an undo")
	}
}

func TestTrashingIsUndoable(t *testing.T) {
	t.Setenv("XDG_DATA_HOME", t.TempDir())
	directory := t.TempDir()
	original := filepath.Join(directory, "note.txt")
	writeFile(t, original, "body")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindTrash, []string{original}, "")

	if exists(original) {
		t.Fatal("trashing must remove the file from its directory")
	}
	_, err := testHarness.engine.Undo()
	if err != nil {
		t.Fatalf("Undo: %v", err)
	}
	if readFile(t, original) != "body" {
		t.Fatal("undo of a trashing must restore the file")
	}
}

func TestOneFailureDoesNotAbandonTheBatch(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	writeFile(t, filepath.Join(source, "good.txt"), "body")

	testHarness := newHarness(refuseConflicts(t))
	sources := []string{filepath.Join(source, "missing.txt"), filepath.Join(source, "good.txt")}
	progress := testHarness.run(t, KindCopy, sources, destination)

	if len(progress.Failures) != 1 {
		t.Fatalf("failures = %v, want exactly the missing source", progress.Failures)
	}
	if !exists(filepath.Join(destination, "good.txt")) {
		t.Fatal("a failure must not stop the rest of the batch")
	}
	if progress.DoneItems != 2 {
		t.Fatalf("DoneItems = %d, want 2", progress.DoneItems)
	}
}

func TestCopyingADirectoryIntoItselfIsRefused(t *testing.T) {
	directory := t.TempDir()
	nested := filepath.Join(directory, "inner")
	mkdir(t, nested)

	testHarness := newHarness(refuseConflicts(t))
	_, err := testHarness.engine.Enqueue(KindCopy, []string{directory}, nested)
	if err != ErrIntoItself {
		t.Fatalf("Enqueue = %v, want ErrIntoItself", err)
	}
}

func TestRenameIsValidatedAndUndoable(t *testing.T) {
	directory := t.TempDir()
	original := filepath.Join(directory, "note.txt")
	writeFile(t, original, "body")

	engine := New(func(string, any) {})
	_, err := engine.Rename(original, "bad/name")
	if err != ErrNameSeparator {
		t.Fatalf("Rename with a separator = %v, want ErrNameSeparator", err)
	}
	renamed, err := engine.Rename(original, "renamed.txt")
	if err != nil {
		t.Fatalf("Rename: %v", err)
	}
	if readFile(t, renamed) != "body" {
		t.Fatal("rename must keep the contents")
	}
	_, err = engine.Undo()
	if err != nil {
		t.Fatalf("Undo: %v", err)
	}
	if !exists(original) {
		t.Fatal("undo of a rename must restore the old name")
	}
}

func TestCreateFolderAndFileRefuseTakenNames(t *testing.T) {
	directory := t.TempDir()
	engine := New(func(string, any) {})

	created, err := engine.CreateFolder(directory, "reports")
	if err != nil {
		t.Fatalf("CreateFolder: %v", err)
	}
	if !isDirectory(created) {
		t.Fatal("CreateFolder must create a directory")
	}
	_, err = engine.CreateFile(directory, "reports")
	if err != ErrNameExists {
		t.Fatalf("CreateFile over an existing name = %v, want ErrNameExists", err)
	}
}

func mkdir(t *testing.T, path string) {
	t.Helper()
	err := os.MkdirAll(path, 0o755)
	if err != nil {
		t.Fatalf("mkdir %s: %v", path, err)
	}
}

func chmod(t *testing.T, path string, mode os.FileMode) {
	t.Helper()
	err := os.Chmod(path, mode)
	if err != nil {
		t.Fatalf("chmod %s: %v", path, err)
	}
}

func readFile(t *testing.T, path string) string {
	t.Helper()
	contents, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	return string(contents)
}

func modeOf(t *testing.T, path string) os.FileMode {
	t.Helper()
	info, err := os.Stat(path)
	if err != nil {
		t.Fatalf("stat %s: %v", path, err)
	}
	return info.Mode().Perm()
}
