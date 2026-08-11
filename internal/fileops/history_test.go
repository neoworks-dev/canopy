package fileops

import (
	"path/filepath"
	"testing"
	"time"
)

func TestRedoOfAMovePutsItBackAtTheDestination(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	original := filepath.Join(source, "note.txt")
	writeFile(t, original, "body")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindMove, []string{original}, destination)
	undo(t, testHarness.engine)
	redo(t, testHarness.engine)

	if exists(original) {
		t.Fatal("redo of a move must take the file away again")
	}
	if readFile(t, filepath.Join(destination, "note.txt")) != "body" {
		t.Fatal("redo of a move must put the file back at the destination")
	}
}

func TestRedoOfACopyMakesTheCopyAgain(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	writeFile(t, filepath.Join(source, "note.txt"), "body")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindCopy, []string{filepath.Join(source, "note.txt")}, destination)
	undo(t, testHarness.engine)
	redo(t, testHarness.engine)
	testHarness.awaitFinished(t)

	if readFile(t, filepath.Join(destination, "note.txt")) != "body" {
		t.Fatal("redo of a copy must copy the file a second time")
	}
}

// A replayed copy is queued like any other, but it belongs to the step already
// back on the undo stack rather than to a new one.
func TestARedoneCopyDoesNotStackASecondUndo(t *testing.T) {
	source, destination := t.TempDir(), t.TempDir()
	writeFile(t, filepath.Join(source, "note.txt"), "body")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindCopy, []string{filepath.Join(source, "note.txt")}, destination)
	undo(t, testHarness.engine)
	redo(t, testHarness.engine)
	testHarness.awaitFinished(t)

	undo(t, testHarness.engine)
	if exists(filepath.Join(destination, "note.txt")) {
		t.Fatal("one undo after a redo must remove the copy again")
	}
	if testHarness.engine.UndoLabel() != "" {
		t.Fatal("a replay must not record a history step of its own")
	}
}

func TestRedoOfATrashingTracksTheNewTrashPath(t *testing.T) {
	t.Setenv("XDG_DATA_HOME", t.TempDir())
	directory := t.TempDir()
	original := filepath.Join(directory, "note.txt")
	writeFile(t, original, "body")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindTrash, []string{original}, "")
	undo(t, testHarness.engine)
	redo(t, testHarness.engine)

	if exists(original) {
		t.Fatal("redo of a trashing must take the file away again")
	}
	undo(t, testHarness.engine)
	if readFile(t, original) != "body" {
		t.Fatal("undo after a redo must restore the file from where it landed the second time")
	}
}

func TestRedoOfARestoreTakesTheItemOutOfTheTrashAgain(t *testing.T) {
	t.Setenv("XDG_DATA_HOME", t.TempDir())
	directory := t.TempDir()
	original := filepath.Join(directory, "note.txt")
	writeFile(t, original, "body")

	engine := New(func(string, any) {})
	trashed, err := Trash(original)
	if err != nil {
		t.Fatalf("Trash: %v", err)
	}
	_, err = engine.RestoreTrashed(trashed.TrashPath)
	if err != nil {
		t.Fatalf("RestoreTrashed: %v", err)
	}

	undo(t, engine)
	if exists(original) {
		t.Fatal("undo of a restore must trash the item again")
	}
	redo(t, engine)
	if readFile(t, original) != "body" {
		t.Fatal("redo of a restore must bring the item back")
	}
}

func TestRedoOfACreatedFolderMakesItAgain(t *testing.T) {
	directory := t.TempDir()
	engine := New(func(string, any) {})

	created, err := engine.CreateFolder(directory, "reports")
	if err != nil {
		t.Fatalf("CreateFolder: %v", err)
	}
	undo(t, engine)
	if exists(created) {
		t.Fatal("undo of a new folder must remove it")
	}
	redo(t, engine)
	if !isDirectory(created) {
		t.Fatal("redo of a new folder must create it again")
	}
}

func TestAFreshActionDropsWhatWasRedoable(t *testing.T) {
	directory := t.TempDir()
	writeFile(t, filepath.Join(directory, "note.txt"), "body")
	engine := New(func(string, any) {})

	_, err := engine.Rename(filepath.Join(directory, "note.txt"), "renamed.txt")
	if err != nil {
		t.Fatalf("Rename: %v", err)
	}
	undo(t, engine)
	if engine.RedoLabel() == "" {
		t.Fatal("an undone rename must be redoable")
	}

	_, err = engine.CreateFile(directory, "other.txt")
	if err != nil {
		t.Fatalf("CreateFile: %v", err)
	}
	if engine.RedoLabel() != "" {
		t.Fatal("a fresh action must drop the redo stack")
	}
}

func TestRedoWithoutAnUndoReportsNothingToRedo(t *testing.T) {
	engine := New(func(string, any) {})
	_, err := engine.Redo()
	if err != ErrNothingToRedo {
		t.Fatalf("Redo on an empty stack = %v, want ErrNothingToRedo", err)
	}
}

func undo(t *testing.T, engine *Engine) {
	t.Helper()
	label, err := engine.Undo()
	if err != nil {
		t.Fatalf("Undo: %v", err)
	}
	if label == "" {
		t.Fatal("undo must describe what it reversed")
	}
}

func redo(t *testing.T, engine *Engine) {
	t.Helper()
	label, err := engine.Redo()
	if err != nil {
		t.Fatalf("Redo: %v", err)
	}
	if label == "" {
		t.Fatal("redo must describe what it performed")
	}
}

// awaitFinished waits for a replayed operation, which runs through the queue and
// so finishes after Redo has already returned.
func (testHarness *harness) awaitFinished(t *testing.T) {
	t.Helper()
	select {
	case <-testHarness.finished:
	case <-time.After(finishTimeout):
		t.Fatal("replayed operation did not finish in time")
	}
}
