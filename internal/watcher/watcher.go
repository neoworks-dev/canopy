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

// Watcher follows the directories on screen. Split panes and tabs mean there
// can be several at once, so the set is replaced as a whole and the watches
// that carry over are left running.
type Watcher struct {
	notify func(path string)

	mutex    sync.Mutex
	sessions map[string]*session
}

func New(notify func(path string)) *Watcher {
	return &Watcher{notify: notify, sessions: map[string]*session{}}
}

// Watch follows exactly the given directories, dropping any others. A directory
// that cannot be watched is skipped: the pane showing it stays a snapshot,
// which is better than the rest going unwatched with it.
func (watcher *Watcher) Watch(paths []string) {
	watcher.mutex.Lock()
	defer watcher.mutex.Unlock()
	watcher.stopSessionsOutside(paths)
	for _, path := range paths {
		watcher.startSessionFor(path)
	}
}

// Stop drops every watch. Watching nothing is a valid state: the trash, tag and
// search panes show no directory.
func (watcher *Watcher) Stop() {
	watcher.mutex.Lock()
	defer watcher.mutex.Unlock()
	watcher.stopSessionsOutside(nil)
}

func (watcher *Watcher) startSessionFor(path string) {
	if watcher.sessions[path] != nil {
		return
	}
	started, err := startSession(path, watcher.notify)
	if err != nil {
		return
	}
	watcher.sessions[path] = started
}

func (watcher *Watcher) stopSessionsOutside(paths []string) {
	wanted := map[string]bool{}
	for _, path := range paths {
		wanted[path] = true
	}
	for path, current := range watcher.sessions {
		watcher.stopSessionUnless(path, current, wanted[path])
	}
}

func (watcher *Watcher) stopSessionUnless(path string, current *session, keep bool) {
	if keep {
		return
	}
	current.stop()
	delete(watcher.sessions, path)
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
