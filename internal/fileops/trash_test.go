package fileops

import (
	"path/filepath"
	"strings"
	"testing"
)

func TestTrashWritesASpecCompliantRecord(t *testing.T) {
	dataHome := t.TempDir()
	t.Setenv("XDG_DATA_HOME", dataHome)
	directory := t.TempDir()
	original := filepath.Join(directory, "holiday photos.txt")
	writeFile(t, original, "body")

	item, err := Trash(original)
	if err != nil {
		t.Fatalf("Trash: %v", err)
	}
	if exists(original) {
		t.Fatal("Trash must move the file out of its directory")
	}
	if readFile(t, item.TrashPath) != "body" {
		t.Fatal("the trashed copy must keep the contents")
	}

	record := readFile(t, filepath.Join(dataHome, "Trash", "info", "holiday photos.txt.trashinfo"))
	if !strings.HasPrefix(record, "[Trash Info]\n") {
		t.Fatalf("record does not start with the spec header: %q", record)
	}
	if !strings.Contains(record, "Path="+strings.ReplaceAll(original, " ", "%20")) {
		t.Fatalf("record does not carry the escaped original path: %q", record)
	}
	if !strings.Contains(record, "DeletionDate=") {
		t.Fatalf("record has no deletion date: %q", record)
	}
}

func TestTrashGivesCollidingNamesTheirOwnRecord(t *testing.T) {
	t.Setenv("XDG_DATA_HOME", t.TempDir())
	first, second := t.TempDir(), t.TempDir()
	writeFile(t, filepath.Join(first, "note.txt"), "first")
	writeFile(t, filepath.Join(second, "note.txt"), "second")

	firstItem, err := Trash(filepath.Join(first, "note.txt"))
	if err != nil {
		t.Fatalf("Trash: %v", err)
	}
	secondItem, err := Trash(filepath.Join(second, "note.txt"))
	if err != nil {
		t.Fatalf("Trash: %v", err)
	}
	if firstItem.TrashPath == secondItem.TrashPath {
		t.Fatal("two files with the same name must not share one trash path")
	}
	if readFile(t, secondItem.TrashPath) != "second" {
		t.Fatal("the second file was overwritten in the trash")
	}
}

func TestRestorePutsAnItemBackAndDropsItsRecord(t *testing.T) {
	dataHome := t.TempDir()
	t.Setenv("XDG_DATA_HOME", dataHome)
	directory := t.TempDir()
	original := filepath.Join(directory, "note.txt")
	writeFile(t, original, "body")

	item, err := Trash(original)
	if err != nil {
		t.Fatalf("Trash: %v", err)
	}
	restored, err := Restore(item.TrashPath)
	if err != nil {
		t.Fatalf("Restore: %v", err)
	}
	if restored != original {
		t.Fatalf("restored to %s, want %s", restored, original)
	}
	if exists(filepath.Join(dataHome, "Trash", "info", "note.txt.trashinfo")) {
		t.Fatal("Restore must drop the .trashinfo record")
	}
}

func TestRestoreDoesNotOverwriteANewOccupant(t *testing.T) {
	t.Setenv("XDG_DATA_HOME", t.TempDir())
	directory := t.TempDir()
	original := filepath.Join(directory, "note.txt")
	writeFile(t, original, "old")

	item, err := Trash(original)
	if err != nil {
		t.Fatalf("Trash: %v", err)
	}
	writeFile(t, original, "written since")

	restored, err := Restore(item.TrashPath)
	if err != nil {
		t.Fatalf("Restore: %v", err)
	}
	if restored == original {
		t.Fatal("Restore must not reclaim an occupied name")
	}
	if readFile(t, original) != "written since" {
		t.Fatal("Restore overwrote the newer file")
	}
	if readFile(t, restored) != "old" {
		t.Fatal("the restored file lost its contents")
	}
}

func TestListTrashedReportsWhatWasTrashed(t *testing.T) {
	t.Setenv("XDG_DATA_HOME", t.TempDir())
	directory := t.TempDir()
	original := filepath.Join(directory, "note.txt")
	writeFile(t, original, "body")

	_, err := Trash(original)
	if err != nil {
		t.Fatalf("Trash: %v", err)
	}
	items, err := ListTrashed()
	if err != nil {
		t.Fatalf("ListTrashed: %v", err)
	}
	if len(items) != 1 {
		t.Fatalf("ListTrashed returned %d items, want 1", len(items))
	}
	if items[0].OriginalPath != original {
		t.Fatalf("OriginalPath = %s, want %s", items[0].OriginalPath, original)
	}
}

func TestEmptyTrashDiscardsEverything(t *testing.T) {
	t.Setenv("XDG_DATA_HOME", t.TempDir())
	directory := t.TempDir()
	writeFile(t, filepath.Join(directory, "note.txt"), "body")

	_, err := Trash(filepath.Join(directory, "note.txt"))
	if err != nil {
		t.Fatalf("Trash: %v", err)
	}
	err = EmptyTrash()
	if err != nil {
		t.Fatalf("EmptyTrash: %v", err)
	}
	items, err := ListTrashed()
	if err != nil {
		t.Fatalf("ListTrashed: %v", err)
	}
	if len(items) != 0 {
		t.Fatalf("trash still holds %d items", len(items))
	}
}

func TestParseTrashInfoReadsPathAndDate(t *testing.T) {
	path, deletedAt := parseTrashInfo("[Trash Info]\nPath=/home/user/a%20b.txt\nDeletionDate=2026-08-10T22:15:00\n")
	if path != "/home/user/a b.txt" {
		t.Fatalf("path = %q, want the unescaped original", path)
	}
	if deletedAt.Year() != 2026 || deletedAt.Minute() != 15 {
		t.Fatalf("deletion date = %v, want 2026-08-10T22:15:00", deletedAt)
	}
}

func TestParseMountPointsDecodesEscapes(t *testing.T) {
	points := parseMountPoints("/dev/sda1 /run/media/my\\040drive ext4 rw 0 0\nproc /proc proc rw 0 0\n")
	if len(points) != 2 {
		t.Fatalf("parsed %d mount points, want 2", len(points))
	}
	if points[0] != "/run/media/my drive" {
		t.Fatalf("mount point = %q, want the decoded name", points[0])
	}
}

func TestTopDirectoryOfStopsAtAMountBoundary(t *testing.T) {
	directory := t.TempDir()
	topDirectory, err := topDirectoryOf(filepath.Join(directory, "note.txt"))
	if err != nil {
		t.Fatalf("topDirectoryOf: %v", err)
	}
	if !strings.HasPrefix(directory, topDirectory) {
		t.Fatalf("top directory %s is not an ancestor of %s", topDirectory, directory)
	}
	if !isDirectory(topDirectory) {
		t.Fatal("top directory must exist")
	}
}

func TestVolumeTrashDirectoryFallsBackToTheOwnedOne(t *testing.T) {
	mountPoint := t.TempDir()
	directory := volumeTrashDirectory(mountPoint)
	if filepath.Dir(directory) != mountPoint {
		t.Fatalf("volume trash %s is not directly under %s", directory, mountPoint)
	}
	if !strings.HasPrefix(filepath.Base(directory), ".Trash-") {
		t.Fatalf("without a sticky .Trash the per-user directory is expected, got %s", directory)
	}
}
