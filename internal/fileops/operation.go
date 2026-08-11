package fileops

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sync"
)

// How much must be copied before another progress event is worth emitting.
const publishByteInterval = 4 * 1024 * 1024

type conflictAnswer struct {
	resolution Resolution
	applyToAll bool
}

// movedItem remembers where something came from so a move can be reversed.
type movedItem struct {
	from string
	to   string
}

// operation is one queued request and everything the worker, the conflict
// prompts and the undo stack need to know about it.
type operation struct {
	engine      *Engine
	id          string
	kind        Kind
	sources     []string
	destination string

	cancelled chan struct{}
	answers   chan conflictAnswer

	mutex       sync.Mutex
	status      Status
	current     string
	done        int
	failures    []Failure
	blanket     Resolution
	resumed     chan struct{}
	sourceSizes []int64
	totalBytes  int64
	doneBytes   int64
	// Byte count at the last emitted event, so events stay coarse.
	publishedBytes int64

	created []string
	moved   []movedItem
	trashed []TrashedItem
}

func newOperation(engine *Engine, id string, kind Kind, sources []string, destination string) *operation {
	running := make(chan struct{})
	close(running)
	return &operation{
		engine:      engine,
		id:          id,
		kind:        kind,
		sources:     append([]string{}, sources...),
		destination: destination,
		cancelled:   make(chan struct{}),
		answers:     make(chan conflictAnswer, 1),
		status:      StatusQueued,
		failures:    []Failure{},
		resumed:     running,
	}
}

func (op *operation) runAll() {
	op.measure()
	for index, source := range op.sources {
		err := op.runOne(source)
		if errors.Is(err, ErrCancelled) {
			return
		}
		op.recordOutcome(index, source, err)
	}
}

// measure sizes the sources up front so progress can be reported in bytes.
// Trashing and deleting move no data, so they keep the item counter alone.
func (op *operation) measure() {
	if op.kind != KindCopy && op.kind != KindMove {
		return
	}
	handle := op.control()
	sizes := make([]int64, 0, len(op.sources))
	for _, source := range op.sources {
		size, err := treeSize(source, handle)
		if err != nil {
			return
		}
		sizes = append(sizes, size)
	}
	op.setSizes(sizes)
}

func (op *operation) setSizes(sizes []int64) {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	op.sourceSizes = sizes
	op.totalBytes = prefixSum(sizes, len(sizes))
}

func (op *operation) runOne(source string) error {
	handle := op.control()
	switch op.kind {
	case KindCopy:
		return op.runCopy(source, handle)
	case KindMove:
		return op.runMove(source, handle)
	case KindTrash:
		return op.runTrash(source, handle)
	case KindDelete:
		return op.runDelete(source, handle)
	}
	return ErrUnknownRequest
}

func (op *operation) runCopy(source string, handle *control) error {
	target, err := op.resolveTarget(source, op.targetIn(source))
	if err != nil {
		return err
	}
	err = copyResolved(source, target, handle)
	if err != nil {
		return err
	}
	op.rememberCreated(target)
	return nil
}

func (op *operation) runMove(source string, handle *control) error {
	target, err := op.resolveTarget(source, op.targetIn(source))
	if err != nil {
		return err
	}
	err = moveResolved(source, target, handle)
	if err != nil {
		return err
	}
	op.rememberMoved(source, target)
	return nil
}

func (op *operation) runTrash(source string, handle *control) error {
	err := handle.waitWhilePaused()
	if err != nil {
		return err
	}
	handle.setCurrent(source)
	item, err := Trash(source)
	if err != nil {
		return err
	}
	op.rememberTrashed(item)
	return nil
}

func (op *operation) runDelete(source string, handle *control) error {
	err := handle.waitWhilePaused()
	if err != nil {
		return err
	}
	handle.setCurrent(source)
	return os.RemoveAll(source)
}

func (op *operation) targetIn(source string) string {
	return filepath.Join(op.destination, filepath.Base(source))
}

// resolveTarget decides where source may land. Two directories merge silently,
// because merging never destroys entries the source does not carry; anything
// else asks the frontend.
func (op *operation) resolveTarget(source, target string) (string, error) {
	if !exists(target) {
		return target, nil
	}
	if isDirectory(source) && isDirectory(target) {
		return target, nil
	}
	resolution, err := op.askConflict(source, target)
	if err != nil {
		return "", err
	}
	return op.applyResolution(target, resolution)
}

func (op *operation) applyResolution(target string, resolution Resolution) (string, error) {
	if resolution == ResolutionReplace {
		return target, nil
	}
	if resolution == ResolutionSkip {
		return "", ErrSkipped
	}
	if resolution == ResolutionKeepBoth {
		return keepBothTarget(target)
	}
	op.cancel()
	return "", ErrCancelled
}

func keepBothTarget(target string) (string, error) {
	directory := filepath.Dir(target)
	name, err := uniqueName(directory, filepath.Base(target))
	if err != nil {
		return "", err
	}
	return filepath.Join(directory, name), nil
}

func (op *operation) askConflict(source, target string) (Resolution, error) {
	blanket := op.blanketResolution()
	if blanket != "" {
		return blanket, nil
	}
	op.engine.emit(EventConflict, ConflictPrompt{
		OperationID:     op.id,
		SourcePath:      source,
		DestinationPath: target,
		SourceIsDir:     isDirectory(source),
	})
	select {
	case <-op.cancelled:
		return ResolutionCancel, ErrCancelled
	case answer := <-op.answers:
		return op.acceptAnswer(answer), nil
	}
}

func (op *operation) acceptAnswer(answer conflictAnswer) Resolution {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	if answer.applyToAll {
		op.blanket = answer.resolution
	}
	return answer.resolution
}

func (op *operation) control() *control {
	return &control{
		waitWhilePaused: op.waitWhilePaused,
		setCurrent:      op.setCurrent,
		resolveTarget:   op.resolveTarget,
		addBytes:        op.addBytes,
	}
}

// waitWhilePaused is checked between files and between chunks of a large file,
// so both pause and cancel land promptly.
func (op *operation) waitWhilePaused() error {
	select {
	case <-op.cancelled:
		return ErrCancelled
	case <-op.resumeGate():
		return nil
	}
}

func (op *operation) resumeGate() <-chan struct{} {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	return op.resumed
}

func (op *operation) pause() {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	if op.status != StatusRunning {
		return
	}
	op.status = StatusPaused
	op.resumed = make(chan struct{})
}

func (op *operation) resume() {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	if op.status != StatusPaused {
		return
	}
	op.status = StatusRunning
	close(op.resumed)
}

// cancel is safe to call more than once: the frontend may cancel an operation
// that has already answered a conflict with "cancel".
func (op *operation) cancel() {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	select {
	case <-op.cancelled:
		return
	default:
		close(op.cancelled)
	}
}

func (op *operation) isCancelled() bool {
	select {
	case <-op.cancelled:
		return true
	default:
		return false
	}
}

func (op *operation) recordOutcome(index int, source string, err error) {
	op.advance(index)
	if err != nil && !errors.Is(err, ErrSkipped) {
		op.addFailure(Failure{Path: source, Message: err.Error()})
	}
	op.engine.publish(op)
}

// advance snaps the byte counter to the exact total of the sources finished so
// far, which both corrects drift and covers a move that renamed rather than
// copied and therefore streamed no bytes at all.
func (op *operation) advance(index int) {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	op.done++
	op.doneBytes = prefixSum(op.sourceSizes, index+1)
}

func (op *operation) addBytes(count int64) {
	if !op.accumulate(count) {
		return
	}
	op.engine.publish(op)
}

// accumulate reports whether enough has moved to be worth another event. One
// event per copied chunk would be thousands per gigabyte.
func (op *operation) accumulate(count int64) bool {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	op.doneBytes += count
	if op.doneBytes-op.publishedBytes < publishByteInterval {
		return false
	}
	op.publishedBytes = op.doneBytes
	return true
}

func (op *operation) addFailure(failure Failure) {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	op.failures = append(op.failures, failure)
}

func (op *operation) setCurrent(path string) {
	op.mutex.Lock()
	op.current = path
	op.mutex.Unlock()
	op.engine.publish(op)
}

func (op *operation) setStatus(status Status) {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	op.status = status
}

func (op *operation) finalStatus() Status {
	if op.isCancelled() {
		return StatusCancelled
	}
	return StatusDone
}

func (op *operation) blanketResolution() Resolution {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	return op.blanket
}

func (op *operation) snapshot() Progress {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	return Progress{
		ID:          op.id,
		Kind:        op.kind,
		Status:      op.status,
		Destination: op.destination,
		TotalItems:  len(op.sources),
		DoneItems:   op.done,
		TotalBytes:  op.totalBytes,
		DoneBytes:   op.doneBytes,
		CurrentPath: op.current,
		Failures:    append([]Failure{}, op.failures...),
	}
}

func (op *operation) rememberCreated(target string) {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	op.created = append(op.created, target)
}

func (op *operation) rememberMoved(source, target string) {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	op.moved = append(op.moved, movedItem{from: source, to: target})
}

func (op *operation) rememberTrashed(item TrashedItem) {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	op.trashed = append(op.trashed, item)
}

// undoStep reverses only what actually completed. Permanent deletion and a
// batch that did nothing produce no step at all.
func (op *operation) undoStep() (undoStep, bool) {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	if op.kind == KindCopy {
		return copyUndo(op.created)
	}
	if op.kind == KindMove {
		return moveUndo(op.moved)
	}
	if op.kind == KindTrash {
		return trashUndo(op.trashed)
	}
	return undoStep{}, false
}

func copyUndo(created []string) (undoStep, bool) {
	if len(created) == 0 {
		return undoStep{}, false
	}
	targets := append([]string{}, created...)
	return undoStep{
		label:  fmt.Sprintf("Copy of %s", itemCount(len(targets))),
		revert: func() error { return removeAll(targets) },
	}, true
}

func moveUndo(moved []movedItem) (undoStep, bool) {
	if len(moved) == 0 {
		return undoStep{}, false
	}
	items := append([]movedItem{}, moved...)
	return undoStep{
		label:  fmt.Sprintf("Move of %s", itemCount(len(items))),
		revert: func() error { return moveBack(items) },
	}, true
}

func trashUndo(trashed []TrashedItem) (undoStep, bool) {
	if len(trashed) == 0 {
		return undoStep{}, false
	}
	items := append([]TrashedItem{}, trashed...)
	return undoStep{
		label:  fmt.Sprintf("Trashing of %s", itemCount(len(items))),
		revert: func() error { return restoreAll(items) },
	}, true
}

func removeAll(targets []string) error {
	for _, target := range targets {
		err := os.RemoveAll(target)
		if err != nil {
			return err
		}
	}
	return nil
}

func moveBack(items []movedItem) error {
	for _, item := range items {
		err := os.Rename(item.to, item.from)
		if err != nil {
			return err
		}
	}
	return nil
}

func restoreAll(items []TrashedItem) error {
	for _, item := range items {
		_, err := Restore(item.TrashPath)
		if err != nil {
			return err
		}
	}
	return nil
}

func itemCount(count int) string {
	if count == 1 {
		return "1 item"
	}
	return fmt.Sprintf("%d items", count)
}

func parentOf(path string) string {
	return filepath.Dir(path)
}

func nameOf(path string) string {
	return filepath.Base(path)
}
