package index

import (
	"context"
	"log/slog"
	"strings"
	"sync"
	"time"

	"github.com/fsnotify/fsnotify"
)

// ChangeHandler receives a coalesced batch of paths that changed on disk.
type ChangeHandler func(paths []string)

// Watcher turns inotify events into debounced batches of changed paths. It adds
// one watch per directory, so it enforces its own ceiling: inotify watches are a
// shared per-user resource and exhausting them would break other applications.
type Watcher struct {
	notifier       *fsnotify.Watcher
	maximumWatches int
	quietPeriod    time.Duration
	handler        ChangeHandler
	logger         *slog.Logger

	mutex        sync.Mutex
	watchedPaths map[string]struct{}
	pendingPaths map[string]struct{}
	limitReached bool
}

func NewWatcher(maximumWatches int, quietPeriod time.Duration, logger *slog.Logger, handler ChangeHandler) (*Watcher, error) {
	notifier, err := fsnotify.NewWatcher()
	if err != nil {
		return nil, err
	}
	return &Watcher{
		notifier:       notifier,
		maximumWatches: maximumWatches,
		quietPeriod:    quietPeriod,
		handler:        handler,
		logger:         logger,
		watchedPaths:   make(map[string]struct{}),
		pendingPaths:   make(map[string]struct{}),
	}, nil
}

// Add starts watching a directory. Exceeding the watch ceiling is not an error:
// the tree stays indexed, it just relies on the periodic rescan for updates.
func (watcher *Watcher) Add(directoryPath string) {
	watcher.mutex.Lock()
	defer watcher.mutex.Unlock()

	if !watcher.canWatchMore(directoryPath) {
		return
	}
	if err := watcher.notifier.Add(directoryPath); err != nil {
		watcher.logger.Debug("watch failed", "path", directoryPath, "error", err)
		return
	}
	watcher.watchedPaths[directoryPath] = struct{}{}
}

// DropSubtree stops watching a removed directory and everything under it.
func (watcher *Watcher) DropSubtree(directoryPath string) {
	watcher.mutex.Lock()
	defer watcher.mutex.Unlock()

	for watchedPath := range watcher.watchedPaths {
		watcher.dropIfWithin(watchedPath, directoryPath)
	}
}

func (watcher *Watcher) WatchedCount() int {
	watcher.mutex.Lock()
	defer watcher.mutex.Unlock()
	return len(watcher.watchedPaths)
}

func (watcher *Watcher) LimitReached() bool {
	watcher.mutex.Lock()
	defer watcher.mutex.Unlock()
	return watcher.limitReached
}

// Run pumps events until the context ends. It owns no state beyond the pending
// set, so it is safe to call Add concurrently.
func (watcher *Watcher) Run(runContext context.Context) {
	flushTicker := time.NewTicker(watcher.quietPeriod)
	defer flushTicker.Stop()

	for watcher.handleNext(runContext, flushTicker.C) {
	}
}

func (watcher *Watcher) Close() error {
	return watcher.notifier.Close()
}

// handleNext processes one event and reports whether the loop should continue.
func (watcher *Watcher) handleNext(runContext context.Context, flushTick <-chan time.Time) bool {
	select {
	case <-runContext.Done():
		return false
	case event, open := <-watcher.notifier.Events:
		return watcher.recordEvent(event, open)
	case watchError, open := <-watcher.notifier.Errors:
		return watcher.recordError(watchError, open)
	case <-flushTick:
		watcher.flushPending()
		return true
	}
}

func (watcher *Watcher) recordEvent(event fsnotify.Event, open bool) bool {
	if !open {
		return false
	}
	watcher.mutex.Lock()
	defer watcher.mutex.Unlock()
	watcher.pendingPaths[event.Name] = struct{}{}
	return true
}

func (watcher *Watcher) recordError(watchError error, open bool) bool {
	if !open {
		return false
	}
	watcher.logger.Warn("watch error", "error", watchError)
	return true
}

func (watcher *Watcher) flushPending() {
	paths := watcher.takePending()
	if len(paths) == 0 {
		return
	}
	watcher.handler(paths)
}

func (watcher *Watcher) takePending() []string {
	watcher.mutex.Lock()
	defer watcher.mutex.Unlock()

	paths := make([]string, 0, len(watcher.pendingPaths))
	for path := range watcher.pendingPaths {
		paths = append(paths, path)
	}
	watcher.pendingPaths = make(map[string]struct{})
	return paths
}

// canWatchMore must be called with the mutex held.
func (watcher *Watcher) canWatchMore(directoryPath string) bool {
	if _, alreadyWatched := watcher.watchedPaths[directoryPath]; alreadyWatched {
		return false
	}
	if len(watcher.watchedPaths) < watcher.maximumWatches {
		return true
	}
	watcher.reportLimitReached()
	return false
}

func (watcher *Watcher) reportLimitReached() {
	if watcher.limitReached {
		return
	}
	watcher.limitReached = true
	watcher.logger.Warn("inotify watch ceiling reached; falling back to periodic rescans",
		"maximumWatches", watcher.maximumWatches)
}

// dropIfWithin must be called with the mutex held.
func (watcher *Watcher) dropIfWithin(watchedPath string, removedPath string) {
	if watchedPath != removedPath && !strings.HasPrefix(watchedPath, removedPath+"/") {
		return
	}
	watcher.notifier.Remove(watchedPath)
	delete(watcher.watchedPaths, watchedPath)
}
