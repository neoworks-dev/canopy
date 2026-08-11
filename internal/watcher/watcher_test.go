package watcher

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"
	"time"
)

// waitForNotification reports the paths notified within the timeout.
func waitForNotification(t *testing.T, notifications <-chan string, timeout time.Duration) string {
	t.Helper()
	select {
	case path := <-notifications:
		return path
	case <-time.After(timeout):
		t.Fatal("expected a change notification")
		return ""
	}
}

func TestWatchReportsACreatedFile(t *testing.T) {
	directory := t.TempDir()
	notifications := make(chan string, 8)
	watcher := New(func(path string) { notifications <- path })
	defer watcher.Stop()

	watcher.Watch([]string{directory})
	writeFile(t, directory, "created.txt")

	notified := waitForNotification(t, notifications, time.Second)
	if notified != directory {
		t.Fatalf("notified about %q, want %q", notified, directory)
	}
}

func TestBurstOfWritesCollapsesIntoOneNotification(t *testing.T) {
	directory := t.TempDir()
	notifications := make(chan string, 64)
	watcher := New(func(path string) { notifications <- path })
	defer watcher.Stop()

	watcher.Watch([]string{directory})
	for index := range 20 {
		writeFile(t, directory, fmt.Sprintf("burst-%d.txt", index))
	}

	waitForNotification(t, notifications, time.Second)
	time.Sleep(2 * coalesceWindow)
	if len(notifications) != 0 {
		t.Fatalf("burst produced %d extra notifications, want 0", len(notifications))
	}
}

func TestWatchingAnotherDirectoryDropsThePreviousOne(t *testing.T) {
	previous := t.TempDir()
	current := t.TempDir()
	notifications := make(chan string, 8)
	watcher := New(func(path string) { notifications <- path })
	defer watcher.Stop()

	watcher.Watch([]string{previous})
	watcher.Watch([]string{current})
	writeFile(t, previous, "ignored.txt")
	time.Sleep(4 * coalesceWindow)
	if len(notifications) != 0 {
		t.Fatalf("dropped directory still notified %d times, want 0", len(notifications))
	}

	writeFile(t, current, "seen.txt")
	notified := waitForNotification(t, notifications, time.Second)
	if notified != current {
		t.Fatalf("notified about %q, want %q", notified, current)
	}
}

// One pane showing a directory that has since been deleted must not stop the
// others from being watched.
func TestAMissingDirectoryIsSkippedAndTheRestAreWatched(t *testing.T) {
	directory := t.TempDir()
	notifications := make(chan string, 8)
	watcher := New(func(path string) { notifications <- path })
	defer watcher.Stop()

	watcher.Watch([]string{filepath.Join(t.TempDir(), "absent"), directory})
	writeFile(t, directory, "seen.txt")

	if notified := waitForNotification(t, notifications, time.Second); notified != directory {
		t.Fatalf("notified about %q, want %q", notified, directory)
	}
}

func TestSeveralDirectoriesAreWatchedAtOnce(t *testing.T) {
	first := t.TempDir()
	second := t.TempDir()
	notifications := make(chan string, 8)
	watcher := New(func(path string) { notifications <- path })
	defer watcher.Stop()

	watcher.Watch([]string{first, second})
	writeFile(t, second, "seen.txt")

	if notified := waitForNotification(t, notifications, time.Second); notified != second {
		t.Fatalf("notified about %q, want %q", notified, second)
	}
}

func writeFile(t *testing.T, directory string, name string) {
	t.Helper()
	path := filepath.Join(directory, filepath.Base(name))
	if err := os.WriteFile(path, []byte("contents"), 0o644); err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}
