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

	if err := watcher.Watch(directory); err != nil {
		t.Fatalf("watch: %v", err)
	}
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

	if err := watcher.Watch(directory); err != nil {
		t.Fatalf("watch: %v", err)
	}
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

	if err := watcher.Watch(previous); err != nil {
		t.Fatalf("watch previous: %v", err)
	}
	if err := watcher.Watch(current); err != nil {
		t.Fatalf("watch current: %v", err)
	}
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

func TestWatchingAMissingDirectoryFails(t *testing.T) {
	watcher := New(func(string) {})
	defer watcher.Stop()

	err := watcher.Watch(filepath.Join(t.TempDir(), "absent"))
	if err == nil {
		t.Fatal("expected watching a missing directory to fail")
	}
}

func writeFile(t *testing.T, directory string, name string) {
	t.Helper()
	path := filepath.Join(directory, filepath.Base(name))
	if err := os.WriteFile(path, []byte("contents"), 0o644); err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}
