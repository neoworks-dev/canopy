// Package diskusage measures what a folder actually holds. Walking a tree can
// take minutes, so a measurement runs in the background, reports what it has
// counted so far, and can be called off when nobody is waiting for it any more.
package diskusage

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sync"
	"time"
)

// How often a running measurement reports what it has counted so far.
const publishInterval = 120 * time.Millisecond

// Event names the scanner emits. The frontend subscribes to these.
const (
	EventProgress = "usage:progress"
	EventFinished = "usage:finished"
)

var (
	ErrNotADirectory      = errors.New("only a folder can be measured")
	ErrUnknownMeasurement = errors.New("unknown measurement")
)

// Measurement is what a walk has counted: a running total while it goes, and
// everything the folder holds once it is finished. Sizes are what the files
// claim rather than what they occupy, which is the same measure the operation
// engine reports progress in.
type Measurement struct {
	ID          string `json:"id"`
	Path        string `json:"path"`
	Bytes       int64  `json:"bytes"`
	Files       int64  `json:"files"`
	Directories int64  `json:"directories"`
	// Entries the walk could not read, which contribute nothing to the total.
	Unreadable int64 `json:"unreadable"`
	Cancelled  bool  `json:"cancelled"`
}

// Emitter delivers an event to the frontend. Keeping it a function leaves this
// package free of any Wails dependency.
type Emitter func(name string, data any)

// Scanner runs the measurements and keeps the handle each one is cancelled by.
type Scanner struct {
	emit Emitter

	mutex    sync.Mutex
	running  map[string]chan struct{}
	sequence int
}

func New(emit Emitter) *Scanner {
	return &Scanner{emit: emit, running: map[string]chan struct{}{}}
}

// Start begins measuring a folder and reports the id its events carry.
func (scanner *Scanner) Start(path string) (string, error) {
	info, err := os.Stat(path)
	if err != nil {
		return "", err
	}
	if !info.IsDir() {
		return "", ErrNotADirectory
	}
	id, stop := scanner.register()
	go scanner.walk(id, path, stop)
	return id, nil
}

// Cancel stops a running measurement, which reports what it had counted.
func (scanner *Scanner) Cancel(id string) error {
	scanner.mutex.Lock()
	defer scanner.mutex.Unlock()
	stop, found := scanner.running[id]
	if !found {
		return ErrUnknownMeasurement
	}
	close(stop)
	delete(scanner.running, id)
	return nil
}

func (scanner *Scanner) register() (string, chan struct{}) {
	scanner.mutex.Lock()
	defer scanner.mutex.Unlock()
	scanner.sequence++
	id := fmt.Sprintf("usage-%d", scanner.sequence)
	stop := make(chan struct{})
	scanner.running[id] = stop
	return id, stop
}

func (scanner *Scanner) forget(id string) {
	scanner.mutex.Lock()
	defer scanner.mutex.Unlock()
	delete(scanner.running, id)
}

func (scanner *Scanner) walk(id, root string, stop chan struct{}) {
	measurement := Measurement{ID: id, Path: root}
	// The zero time is long enough ago that the first entry publishes, so a pane
	// shows it is counting rather than sitting empty for the first interval.
	published := time.Time{}
	filepath.WalkDir(root, func(path string, entry fs.DirEntry, walkErr error) error {
		if isStopped(stop) {
			measurement.Cancelled = true
			return filepath.SkipAll
		}
		count(&measurement, path, root, entry, walkErr)
		published = scanner.publishEvery(measurement, published)
		return nil
	})
	scanner.forget(id)
	scanner.emit(EventFinished, measurement)
}

// publishEvery keeps the events coarse: a walk visits far more entries than a
// pane can usefully redraw for.
func (scanner *Scanner) publishEvery(measurement Measurement, published time.Time) time.Time {
	if time.Since(published) < publishInterval {
		return published
	}
	scanner.emit(EventProgress, measurement)
	return time.Now()
}

// count folds one visited entry into the totals. The root is the folder being
// measured rather than something it holds, so it is left out.
func count(measurement *Measurement, path, root string, entry fs.DirEntry, walkErr error) {
	if walkErr != nil {
		measurement.Unreadable++
		return
	}
	if path == root {
		return
	}
	if entry.IsDir() {
		measurement.Directories++
		return
	}
	info, err := entry.Info()
	if err != nil {
		measurement.Unreadable++
		return
	}
	measurement.Files++
	measurement.Bytes += info.Size()
}

func isStopped(stop chan struct{}) bool {
	select {
	case <-stop:
		return true
	default:
		return false
	}
}
