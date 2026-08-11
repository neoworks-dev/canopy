// Package watcher reports filesystem changes for the directory the explorer is
// currently showing, so a listing does not go stale when something else writes
// to it.
package watcher

import (
	"sync"
	"time"

	"github.com/fsnotify/fsnotify"
)

// A burst of writes — an extraction, a build — collapses into one notification
// instead of a reload per file.
const coalesceWindow = 150 * time.Millisecond

// Watcher follows one directory at a time. Watching another one replaces the
// previous watch, which is all a single listing pane needs.
type Watcher struct {
	notify func(path string)

	mutex   sync.Mutex
	session *session
}

func New(notify func(path string)) *Watcher {
	return &Watcher{notify: notify}
}

// Watch replaces the current watch with one on path.
func (watcher *Watcher) Watch(path string) error {
	watcher.mutex.Lock()
	defer watcher.mutex.Unlock()
	watcher.stopSession()
	started, err := startSession(path, watcher.notify)
	if err != nil {
		return err
	}
	watcher.session = started
	return nil
}

// Stop drops the current watch. Watching nothing is a valid state: the trash
// and search panes show no directory.
func (watcher *Watcher) Stop() {
	watcher.mutex.Lock()
	defer watcher.mutex.Unlock()
	watcher.stopSession()
}

func (watcher *Watcher) stopSession() {
	if watcher.session == nil {
		return
	}
	watcher.session.stop()
	watcher.session = nil
}

// session owns one fsnotify watch and the goroutine draining it.
type session struct {
	path    string
	notify  func(path string)
	watcher *fsnotify.Watcher
	done    chan struct{}

	// pending is the coalescing window's channel, nil while no window is open.
	// A nil channel blocks forever in select, which is exactly the idle state.
	pending <-chan time.Time
	timer   *time.Timer
}

func startSession(path string, notify func(path string)) (*session, error) {
	fsWatcher, err := fsnotify.NewWatcher()
	if err != nil {
		return nil, err
	}
	err = fsWatcher.Add(path)
	if err != nil {
		fsWatcher.Close()
		return nil, err
	}
	started := &session{path: path, notify: notify, watcher: fsWatcher, done: make(chan struct{})}
	go started.run()
	return started, nil
}

func (current *session) stop() {
	close(current.done)
	current.watcher.Close()
}

func (current *session) run() {
	for {
		if !current.step() {
			return
		}
	}
}

// step handles one channel receive and reports whether the session continues.
func (current *session) step() bool {
	select {
	case _, open := <-current.watcher.Events:
		return current.recordChange(open)
	case _, open := <-current.watcher.Errors:
		return open
	case <-current.pending:
		return current.flush()
	case <-current.done:
		return false
	}
}

func (current *session) recordChange(open bool) bool {
	if !open {
		return false
	}
	current.openWindow()
	return true
}

// The window is trailing-edge and never extended, so a directory under constant
// writes still reports at a steady rate instead of starving.
func (current *session) openWindow() {
	if current.pending != nil {
		return
	}
	current.timer = time.NewTimer(coalesceWindow)
	current.pending = current.timer.C
}

func (current *session) flush() bool {
	current.pending = nil
	current.notify(current.path)
	return true
}
