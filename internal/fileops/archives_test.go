package fileops

import (
	"os"
	"path/filepath"
	"testing"
)

// packedTree writes a small tree and packs it into an archive beside it,
// reporting where the archive landed.
func packedTree(t *testing.T, root string, archiveName string) string {
	t.Helper()
	source := filepath.Join(root, "project")
	mkdir(t, filepath.Join(source, "nested"))
	writeFile(t, filepath.Join(source, "top.txt"), "top")
	writeFile(t, filepath.Join(source, "nested", "deep.txt"), "deep")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindCompress, []string{source}, root)
	return filepath.Join(root, archiveName)
}

func TestPackingASelectionLandsBesideIt(t *testing.T) {
	root := t.TempDir()
	archivePath := packedTree(t, root, "project.zip")

	if !exists(archivePath) {
		t.Fatal("packing one folder must name the archive after it")
	}
}

func TestPackingSeveralEntriesGivesOneArchive(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "one.txt"), "one")
	writeFile(t, filepath.Join(root, "two.txt"), "two")

	testHarness := newHarness(refuseConflicts(t))
	sources := []string{filepath.Join(root, "one.txt"), filepath.Join(root, "two.txt")}
	testHarness.run(t, KindCompress, sources, root)

	if !exists(filepath.Join(root, "Archive.zip")) {
		t.Fatal("packing several entries must produce one archive of its own name")
	}
}

func TestExtractionUnpacksTheWholeTree(t *testing.T) {
	root := t.TempDir()
	archivePath := packedTree(t, root, "project.zip")
	destination := filepath.Join(root, "unpacked")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindExtract, []string{archivePath}, destination)

	if readFile(t, filepath.Join(destination, "project", "top.txt")) != "top" {
		t.Fatal("extraction must write the files the archive holds")
	}
	if readFile(t, filepath.Join(destination, "project", "nested", "deep.txt")) != "deep" {
		t.Fatal("extraction must write the nested files too")
	}
}

func TestUndoingAnExtractionRemovesTheFolderItMade(t *testing.T) {
	root := t.TempDir()
	archivePath := packedTree(t, root, "project.zip")
	destination := filepath.Join(root, "unpacked")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindExtract, []string{archivePath}, destination)
	_, err := testHarness.engine.Undo()
	if err != nil {
		t.Fatalf("Undo: %v", err)
	}

	if exists(destination) {
		t.Fatal("undo of an extraction must remove the folder it unpacked into")
	}
	if !exists(archivePath) {
		t.Fatal("undo of an extraction must leave the archive alone")
	}
}

func TestRedoingAnExtractionUnpacksItAgain(t *testing.T) {
	root := t.TempDir()
	archivePath := packedTree(t, root, "project.zip")
	destination := filepath.Join(root, "unpacked")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindExtract, []string{archivePath}, destination)
	undo(t, testHarness.engine)
	redo(t, testHarness.engine)
	testHarness.awaitFinished(t)

	if readFile(t, filepath.Join(destination, "project", "top.txt")) != "top" {
		t.Fatal("redo of an extraction must unpack the archive a second time")
	}
}

func TestUndoingPackingRemovesTheArchive(t *testing.T) {
	root := t.TempDir()
	source := filepath.Join(root, "project")
	mkdir(t, source)
	writeFile(t, filepath.Join(source, "top.txt"), "top")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindCompress, []string{source}, root)
	_, err := testHarness.engine.Undo()
	if err != nil {
		t.Fatalf("Undo: %v", err)
	}

	if exists(filepath.Join(root, "project.zip")) {
		t.Fatal("undo of a packing must remove the archive")
	}
	if !exists(filepath.Join(source, "top.txt")) {
		t.Fatal("undo of a packing must leave what was packed alone")
	}
}

// An extraction into a folder that already holds entries records what it added
// rather than the folder, so undoing it does not take the folder with it.
func TestExtractingIntoAnExistingFolderOnlyUndoesWhatItAdded(t *testing.T) {
	root := t.TempDir()
	archivePath := packedTree(t, root, "project.zip")
	destination := filepath.Join(root, "existing")
	mkdir(t, destination)
	writeFile(t, filepath.Join(destination, "mine.txt"), "mine")

	testHarness := newHarness(refuseConflicts(t))
	testHarness.run(t, KindExtract, []string{archivePath}, destination)
	_, err := testHarness.engine.Undo()
	if err != nil {
		t.Fatalf("Undo: %v", err)
	}

	if exists(filepath.Join(destination, "project")) {
		t.Fatal("undo must remove what the extraction added")
	}
	if readFile(t, filepath.Join(destination, "mine.txt")) != "mine" {
		t.Fatal("undo must leave what was already in the folder")
	}
}

func TestExtractingSomethingThatIsNotAnArchiveIsRefused(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "note.txt"), "body")

	engine := New(func(string, any) {})
	_, err := engine.Enqueue(KindExtract, []string{filepath.Join(root, "note.txt")}, filepath.Join(root, "out"))

	if err == nil {
		t.Fatal("a plain file cannot be extracted")
	}
}

func TestAnExtractionNeedsAFolderToUnpackInto(t *testing.T) {
	root := t.TempDir()
	archivePath := packedTree(t, root, "project.zip")

	engine := New(func(string, any) {})
	_, err := engine.Enqueue(KindExtract, []string{archivePath}, "")

	if err == nil {
		t.Fatal("an extraction without a destination cannot be queued")
	}
}

func TestTheSuggestedExtractionFolderIsNamedAfterTheArchive(t *testing.T) {
	root := t.TempDir()
	archivePath := filepath.Join(root, "photos.tar.gz")
	writeFile(t, archivePath, "not really an archive")

	folder := ExtractionFolderFor(archivePath)

	if folder != filepath.Join(root, "photos") {
		t.Fatalf("folder = %q, want the archive's name without its suffix", folder)
	}
}

func TestTheSuggestedExtractionFolderStepsAsideForOneThatExists(t *testing.T) {
	root := t.TempDir()
	archivePath := filepath.Join(root, "photos.zip")
	writeFile(t, archivePath, "not really an archive")
	err := os.Mkdir(filepath.Join(root, "photos"), 0o755)
	if err != nil {
		t.Fatalf("mkdir: %v", err)
	}

	folder := ExtractionFolderFor(archivePath)

	if folder != filepath.Join(root, "photos (1)") {
		t.Fatalf("folder = %q, want a name that is free", folder)
	}
}
