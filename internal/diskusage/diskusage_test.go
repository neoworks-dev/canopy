package diskusage

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

const finishTimeout = 5 * time.Second

// collector keeps the finished measurement one scanner reports, and can react to
// the progress along the way. Both callbacks run on the walking goroutine, so a
// collector that cancels does so between two entries rather than racing them.
type collector struct {
	scanner    *Scanner
	finished   chan Measurement
	onProgress func(scanner *Scanner, measurement Measurement)
}

func newCollector(onProgress func(scanner *Scanner, measurement Measurement)) *collector {
	gathered := &collector{finished: make(chan Measurement, 1), onProgress: onProgress}
	gathered.scanner = New(gathered.emit)
	return gathered
}

func (gathered *collector) emit(name string, data any) {
	if name == EventProgress && gathered.onProgress != nil {
		gathered.onProgress(gathered.scanner, data.(Measurement))
	}
	if name == EventFinished {
		gathered.finished <- data.(Measurement)
	}
}

func (gathered *collector) measure(t *testing.T, path string) Measurement {
	t.Helper()
	_, err := gathered.scanner.Start(path)
	if err != nil {
		t.Fatalf("Start: %v", err)
	}
	select {
	case measurement := <-gathered.finished:
		return measurement
	case <-time.After(finishTimeout):
		t.Fatal("measurement did not finish in time")
	}
	return Measurement{}
}

func ignoreProgress(*Scanner, Measurement) {}

func TestMeasuringCountsTheWholeTree(t *testing.T) {
	root := t.TempDir()
	mkdir(t, filepath.Join(root, "nested", "deeper"))
	writeFile(t, filepath.Join(root, "top.txt"), "0123456789")
	writeFile(t, filepath.Join(root, "nested", "middle.txt"), "abcde")
	writeFile(t, filepath.Join(root, "nested", "deeper", "bottom.txt"), "xyz")

	measurement := newCollector(ignoreProgress).measure(t, root)

	if measurement.Bytes != 18 {
		t.Fatalf("Bytes = %d, want the whole tree", measurement.Bytes)
	}
	if measurement.Files != 3 {
		t.Fatalf("Files = %d, want 3", measurement.Files)
	}
	if measurement.Directories != 2 {
		t.Fatalf("Directories = %d, want the two below the root", measurement.Directories)
	}
	if measurement.Cancelled {
		t.Fatal("a measurement that ran to the end is not cancelled")
	}
}

func TestAnEmptyFolderMeasuresZero(t *testing.T) {
	measurement := newCollector(ignoreProgress).measure(t, t.TempDir())

	if measurement.Bytes != 0 || measurement.Files != 0 || measurement.Directories != 0 {
		t.Fatalf("empty folder measured %+v", measurement)
	}
}

func TestAnUnreadableFolderIsCountedRatherThanFatal(t *testing.T) {
	if os.Geteuid() == 0 {
		t.Skip("root reads a directory whatever its mode says")
	}
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "top.txt"), "0123456789")
	closed := filepath.Join(root, "closed")
	mkdir(t, closed)
	writeFile(t, filepath.Join(closed, "hidden.txt"), "secret")
	chmod(t, closed, 0o000)
	t.Cleanup(func() { chmod(t, closed, 0o755) })

	measurement := newCollector(ignoreProgress).measure(t, root)

	if measurement.Unreadable == 0 {
		t.Fatal("a folder the walk cannot open must be reported as unreadable")
	}
	if measurement.Bytes != 10 {
		t.Fatalf("Bytes = %d, want what could actually be read", measurement.Bytes)
	}
}

func TestCancellingStopsTheWalkAndSaysSo(t *testing.T) {
	root := t.TempDir()
	mkdir(t, filepath.Join(root, "nested"))
	writeFile(t, filepath.Join(root, "nested", "note.txt"), "body")

	cancelAtOnce := func(scanner *Scanner, measurement Measurement) {
		scanner.Cancel(measurement.ID)
	}
	measurement := newCollector(cancelAtOnce).measure(t, root)

	if !measurement.Cancelled {
		t.Fatal("a cancelled measurement must say it was cut short")
	}
	if measurement.Files != 0 {
		t.Fatalf("Files = %d, want nothing counted after the first entry", measurement.Files)
	}
}

func TestMeasuringAFileIsRefused(t *testing.T) {
	path := filepath.Join(t.TempDir(), "note.txt")
	writeFile(t, path, "body")

	_, err := newCollector(ignoreProgress).scanner.Start(path)
	if err != ErrNotADirectory {
		t.Fatalf("Start on a file = %v, want ErrNotADirectory", err)
	}
}

func TestCancellingAMeasurementNobodyStartedIsRefused(t *testing.T) {
	err := newCollector(ignoreProgress).scanner.Cancel("usage-404")
	if err != ErrUnknownMeasurement {
		t.Fatalf("Cancel of an unknown id = %v, want ErrUnknownMeasurement", err)
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

func chmod(t *testing.T, path string, mode os.FileMode) {
	t.Helper()
	err := os.Chmod(path, mode)
	if err != nil {
		t.Fatalf("chmod %s: %v", path, err)
	}
}
