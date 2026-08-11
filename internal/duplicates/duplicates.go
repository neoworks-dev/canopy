// Package duplicates finds files under a folder that hold the same bytes.
//
// Reading every file to compare it would be far more work than the answer is
// worth, so the search narrows in three passes: files that share a size, then
// files that share a signature taken from each end, and only then the full
// contents of what is left.
package duplicates

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"sync"
	"time"
)

// How often a running search reports where it has got to.
const publishInterval = 120 * time.Millisecond

// Event names the scanner emits. The frontend subscribes to these.
const (
	EventProgress = "duplicates:progress"
	EventFinished = "duplicates:finished"
)

// What a search is busy with, which is what the pane says while it waits.
const (
	StageWalking   = "walking"
	StageComparing = "comparing"
	StageDone      = "done"
)

var (
	ErrNotADirectory = errors.New("only a folder can be searched")
	ErrUnknownSearch = errors.New("unknown search")
)

// Group is one set of files holding the same bytes. The first path is no more
// original than the rest; they are in the order the walk found them.
type Group struct {
	Size  int64    `json:"size"`
	Paths []string `json:"paths"`
}

// Report is where a search has got to, and everything it found once it is done.
type Report struct {
	ID    string `json:"id"`
	Root  string `json:"root"`
	Stage string `json:"stage"`
	// Files the walk has seen, and of those, the ones that share a size with
	// another and are therefore worth reading.
	Scanned    int64 `json:"scanned"`
	Candidates int64 `json:"candidates"`
	Compared   int64 `json:"compared"`
	// Files that could not be read, which are left out of the comparison.
	Unreadable int64 `json:"unreadable"`
	// What deleting all but one file of every group would free.
	RecoverableBytes int64   `json:"recoverableBytes"`
	Groups           []Group `json:"groups"`
	Cancelled        bool    `json:"cancelled"`
}

// Emitter delivers an event to the frontend. Keeping it a function leaves this
// package free of any Wails dependency.
type Emitter func(name string, data any)

// Scanner runs the searches and keeps the handle each one is cancelled by.
type Scanner struct {
	emit Emitter

	mutex    sync.Mutex
	running  map[string]chan struct{}
	sequence int
}

// candidate is one file the walk found, kept with the information the identity
// check needs so hard links are not counted as copies.
type candidate struct {
	path string
	info os.FileInfo
}

func New(emit Emitter) *Scanner {
	return &Scanner{emit: emit, running: map[string]chan struct{}{}}
}

// Start begins searching a folder and reports the id its events carry.
func (scanner *Scanner) Start(root string) (string, error) {
	info, err := os.Stat(root)
	if err != nil {
		return "", err
	}
	if !info.IsDir() {
		return "", ErrNotADirectory
	}
	id, stop := scanner.register()
	go scanner.search(id, root, stop)
	return id, nil
}

// Cancel stops a running search, which reports what it had found so far.
func (scanner *Scanner) Cancel(id string) error {
	scanner.mutex.Lock()
	defer scanner.mutex.Unlock()
	stop, found := scanner.running[id]
	if !found {
		return ErrUnknownSearch
	}
	close(stop)
	delete(scanner.running, id)
	return nil
}

func (scanner *Scanner) register() (string, chan struct{}) {
	scanner.mutex.Lock()
	defer scanner.mutex.Unlock()
	scanner.sequence++
	id := fmt.Sprintf("duplicates-%d", scanner.sequence)
	stop := make(chan struct{})
	scanner.running[id] = stop
	return id, stop
}

func (scanner *Scanner) forget(id string) {
	scanner.mutex.Lock()
	defer scanner.mutex.Unlock()
	delete(scanner.running, id)
}

func (scanner *Scanner) search(id, root string, stop chan struct{}) {
	run := &searchRun{
		report:  Report{ID: id, Root: root, Stage: StageWalking, Groups: []Group{}},
		publish: scanner.publisher(),
		stop:    stop,
		bySize:  map[int64][]candidate{},
	}
	run.walk(root)
	run.compare()
	scanner.forget(id)
	run.report.Stage = StageDone
	scanner.emit(EventFinished, run.report)
}

// publisher hands a run a way to report progress that stays coarse: a search
// visits far more files than a pane can usefully redraw for.
func (scanner *Scanner) publisher() func(Report) {
	published := time.Time{}
	return func(report Report) {
		if time.Since(published) < publishInterval {
			return
		}
		published = time.Now()
		scanner.emit(EventProgress, withoutGroups(report))
	}
}

// withoutGroups keeps the progress events small: the groups are only worth
// sending once, with the finished report.
func withoutGroups(report Report) Report {
	report.Groups = nil
	return report
}

// searchRun is the state of one search, which belongs to the goroutine running
// it and to nothing else.
type searchRun struct {
	report  Report
	publish func(Report)
	stop    chan struct{}
	bySize  map[int64][]candidate
}

func (run *searchRun) walk(root string) {
	filepath.WalkDir(root, func(path string, entry fs.DirEntry, walkErr error) error {
		if run.stopped() {
			return filepath.SkipAll
		}
		run.consider(path, entry, walkErr)
		run.publish(run.report)
		return nil
	})
}

// consider folds one visited entry into the search. Only ordinary files with
// something in them can be copies of each other: an empty file matches every
// other empty file, which is noise rather than a finding.
func (run *searchRun) consider(path string, entry fs.DirEntry, walkErr error) {
	if walkErr != nil {
		run.report.Unreadable++
		return
	}
	if entry.IsDir() || !entry.Type().IsRegular() {
		return
	}
	info, err := entry.Info()
	if err != nil {
		run.report.Unreadable++
		return
	}
	run.report.Scanned++
	if info.Size() == 0 {
		return
	}
	run.bySize[info.Size()] = append(run.bySize[info.Size()], candidate{path: path, info: info})
}

func (run *searchRun) compare() {
	run.report.Stage = StageComparing
	for _, size := range sortedSizes(run.bySize) {
		if run.stopped() {
			return
		}
		run.compareSize(size, run.bySize[size])
	}
}

// compareSize takes one set of same-sized files down to the groups that really
// do hold the same bytes.
func (run *searchRun) compareSize(size int64, sharing []candidate) {
	distinct := withoutRepeatedFiles(sharing)
	if len(distinct) < 2 {
		return
	}
	run.report.Candidates += int64(len(distinct))
	for _, bySignature := range run.split(distinct, size, signatureOf) {
		run.collect(size, run.split(bySignature, size, contentsOf))
	}
}

// split divides a set of files by what the digest makes of them, keeping only
// the sets that still have something to compare.
func (run *searchRun) split(
	sharing []candidate,
	size int64,
	digest func(path string, size int64) (string, error),
) [][]candidate {
	byDigest := map[string][]candidate{}
	order := []string{}
	for _, file := range sharing {
		key, ok := run.digestOf(file, size, digest)
		if !ok {
			continue
		}
		order = appendUnseen(order, byDigest, key)
		byDigest[key] = append(byDigest[key], file)
	}
	return groupsWorthKeeping(order, byDigest)
}

func (run *searchRun) digestOf(
	file candidate,
	size int64,
	digest func(path string, size int64) (string, error),
) (string, bool) {
	if run.stopped() {
		return "", false
	}
	key, err := digest(file.path, size)
	if err != nil {
		run.report.Unreadable++
		return "", false
	}
	run.report.Compared++
	run.publish(run.report)
	return key, true
}

func (run *searchRun) collect(size int64, found [][]candidate) {
	for _, matching := range found {
		run.report.Groups = append(run.report.Groups, Group{Size: size, Paths: pathsOf(matching)})
		run.report.RecoverableBytes += size * int64(len(matching)-1)
	}
}

func (run *searchRun) stopped() bool {
	select {
	case <-run.stop:
		run.report.Cancelled = true
		return true
	default:
		return false
	}
}

// withoutRepeatedFiles drops the second name of a hard link. Two names for one
// file are not two copies, and deleting one frees nothing.
func withoutRepeatedFiles(sharing []candidate) []candidate {
	distinct := make([]candidate, 0, len(sharing))
	for _, file := range sharing {
		if containsSameFile(distinct, file) {
			continue
		}
		distinct = append(distinct, file)
	}
	return distinct
}

func containsSameFile(distinct []candidate, file candidate) bool {
	for _, kept := range distinct {
		if os.SameFile(kept.info, file.info) {
			return true
		}
	}
	return false
}

func groupsWorthKeeping(order []string, byDigest map[string][]candidate) [][]candidate {
	kept := [][]candidate{}
	for _, key := range order {
		if len(byDigest[key]) < 2 {
			continue
		}
		kept = append(kept, byDigest[key])
	}
	return kept
}

func appendUnseen(order []string, byDigest map[string][]candidate, key string) []string {
	_, seen := byDigest[key]
	if seen {
		return order
	}
	return append(order, key)
}

func pathsOf(matching []candidate) []string {
	paths := make([]string, 0, len(matching))
	for _, file := range matching {
		paths = append(paths, file.path)
	}
	return paths
}

// sortedSizes puts the largest files first, so the groups worth the most space
// are the ones a cancelled search has already found.
func sortedSizes(bySize map[int64][]candidate) []int64 {
	sizes := make([]int64, 0, len(bySize))
	for size := range bySize {
		sizes = append(sizes, size)
	}
	sort.Slice(sizes, func(first, second int) bool { return sizes[first] > sizes[second] })
	return sizes
}
