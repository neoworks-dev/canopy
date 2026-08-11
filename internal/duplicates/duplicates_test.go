package duplicates

import (
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"
	"time"
)

const finishTimeout = 10 * time.Second

// collector keeps the finished report one scanner produces, and can react to the
// progress along the way. Both callbacks run on the searching goroutine, so a
// collector that cancels does so between two files rather than racing them.
type collector struct {
	scanner    *Scanner
	finished   chan Report
	onProgress func(scanner *Scanner, report Report)
}

func newCollector() *collector {
	return collectorWatching(nil)
}

func collectorWatching(onProgress func(scanner *Scanner, report Report)) *collector {
	gathered := &collector{finished: make(chan Report, 1), onProgress: onProgress}
	gathered.scanner = New(gathered.emit)
	return gathered
}

func (gathered *collector) emit(name string, data any) {
	if name == EventProgress && gathered.onProgress != nil {
		gathered.onProgress(gathered.scanner, data.(Report))
	}
	if name == EventFinished {
		gathered.finished <- data.(Report)
	}
}

func (gathered *collector) search(t *testing.T, root string) Report {
	t.Helper()
	_, err := gathered.scanner.Start(root)
	if err != nil {
		t.Fatalf("Start: %v", err)
	}
	select {
	case report := <-gathered.finished:
		return report
	case <-time.After(finishTimeout):
		t.Fatal("search did not finish in time")
	}
	return Report{}
}

func TestFilesWithTheSameContentsAreFound(t *testing.T) {
	root := t.TempDir()
	mkdir(t, filepath.Join(root, "nested"))
	writeFile(t, filepath.Join(root, "one.txt"), "the same bytes")
	writeFile(t, filepath.Join(root, "nested", "two.txt"), "the same bytes")
	writeFile(t, filepath.Join(root, "other.txt"), "different bytes")

	report := newCollector().search(t, root)

	if len(report.Groups) != 1 {
		t.Fatalf("Groups = %v, want the one pair", report.Groups)
	}
	if len(report.Groups[0].Paths) != 2 {
		t.Fatalf("Paths = %v, want both copies", report.Groups[0].Paths)
	}
	if report.RecoverableBytes != int64(len("the same bytes")) {
		t.Fatalf("RecoverableBytes = %d, want one copy's worth", report.RecoverableBytes)
	}
}

func TestFilesOfTheSameSizeThatDifferAreNotAGroup(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "one.txt"), "aaaa")
	writeFile(t, filepath.Join(root, "two.txt"), "bbbb")

	report := newCollector().search(t, root)

	if len(report.Groups) != 0 {
		t.Fatalf("Groups = %v, want none: only the sizes match", report.Groups)
	}
	if report.Candidates != 2 {
		t.Fatalf("Candidates = %d, want the two files worth reading", report.Candidates)
	}
}

// Two files can share both ends and still differ, which is what the third pass
// over the whole contents is for.
func TestFilesThatShareBothEndsAreStillCompared(t *testing.T) {
	root := t.TempDir()
	head := strings.Repeat("h", signatureLength)
	tail := strings.Repeat("t", signatureLength)
	writeFile(t, filepath.Join(root, "one.bin"), head+"middle one"+tail)
	writeFile(t, filepath.Join(root, "two.bin"), head+"middle two"+tail)
	writeFile(t, filepath.Join(root, "three.bin"), head+"middle one"+tail)

	report := newCollector().search(t, root)

	if len(report.Groups) != 1 {
		t.Fatalf("Groups = %v, want only the pair that matches all through", report.Groups)
	}
	if !sameNames(report.Groups[0].Paths, []string{"one.bin", "three.bin"}) {
		t.Fatalf("Paths = %v, want one.bin and three.bin", report.Groups[0].Paths)
	}
}

func TestEmptyFilesAreNotReportedAsCopies(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "one.txt"), "")
	writeFile(t, filepath.Join(root, "two.txt"), "")

	report := newCollector().search(t, root)

	if len(report.Groups) != 0 {
		t.Fatalf("Groups = %v, want none: an empty file matches every other one", report.Groups)
	}
}

func TestASecondNameForOneFileIsNotACopy(t *testing.T) {
	root := t.TempDir()
	original := filepath.Join(root, "one.txt")
	writeFile(t, original, "the same bytes")
	err := os.Link(original, filepath.Join(root, "linked.txt"))
	if err != nil {
		t.Skipf("hard links are unavailable here: %v", err)
	}

	report := newCollector().search(t, root)

	if len(report.Groups) != 0 {
		t.Fatalf("Groups = %v, want none: deleting a hard link frees nothing", report.Groups)
	}
}

func TestASymlinkToADuplicateIsLeftOut(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "one.txt"), "the same bytes")
	writeFile(t, filepath.Join(root, "two.txt"), "the same bytes")
	err := os.Symlink(filepath.Join(root, "one.txt"), filepath.Join(root, "pointer.txt"))
	if err != nil {
		t.Skipf("symlinks are unavailable here: %v", err)
	}

	report := newCollector().search(t, root)

	if len(report.Groups[0].Paths) != 2 {
		t.Fatalf("Paths = %v, want the two files without the link", report.Groups[0].Paths)
	}
}

func TestTheLargestFilesAreComparedFirst(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "small-one.txt"), "aa")
	writeFile(t, filepath.Join(root, "small-two.txt"), "aa")
	writeFile(t, filepath.Join(root, "big-one.txt"), strings.Repeat("b", 400))
	writeFile(t, filepath.Join(root, "big-two.txt"), strings.Repeat("b", 400))

	report := newCollector().search(t, root)

	if len(report.Groups) != 2 {
		t.Fatalf("Groups = %v, want both pairs", report.Groups)
	}
	if report.Groups[0].Size <= report.Groups[1].Size {
		t.Fatal("the pair worth the most space must come first")
	}
}

func TestSearchingAFileIsRefused(t *testing.T) {
	path := filepath.Join(t.TempDir(), "note.txt")
	writeFile(t, path, "body")

	_, err := newCollector().scanner.Start(path)
	if err != ErrNotADirectory {
		t.Fatalf("Start on a file = %v, want ErrNotADirectory", err)
	}
}

func TestCancellingASearchNobodyStartedIsRefused(t *testing.T) {
	err := newCollector().scanner.Cancel("duplicates-404")
	if err != ErrUnknownSearch {
		t.Fatalf("Cancel of an unknown id = %v, want ErrUnknownSearch", err)
	}
}

func TestACancelledSearchStopsAndSaysSo(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "one.txt"), "the same bytes")
	writeFile(t, filepath.Join(root, "two.txt"), "the same bytes")

	cancelAtOnce := func(scanner *Scanner, report Report) {
		scanner.Cancel(report.ID)
	}
	report := collectorWatching(cancelAtOnce).search(t, root)

	if !report.Cancelled {
		t.Fatal("a cancelled search must say it was cut short")
	}
	if len(report.Groups) != 0 {
		t.Fatalf("Groups = %v, want nothing compared after the walk was stopped", report.Groups)
	}
}

func sameNames(paths []string, wanted []string) bool {
	names := make([]string, 0, len(paths))
	for _, path := range paths {
		names = append(names, filepath.Base(path))
	}
	sort.Strings(names)
	sort.Strings(wanted)
	return strings.Join(names, ",") == strings.Join(wanted, ",")
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
