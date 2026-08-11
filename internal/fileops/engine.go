package fileops

import (
	"fmt"
	"os"
	"sync"
)

const queueCapacity = 64

// Engine runs every mutating operation on one worker, so copy, move, trash and
// delete share the same progress reporting, conflict handling and undo history.
type Engine struct {
	emit  Emitter
	queue chan *operation

	past history

	mutex      sync.Mutex
	operations map[string]*operation
	sequence   int
}

func New(emit Emitter) *Engine {
	engine := &Engine{
		emit:       emit,
		queue:      make(chan *operation, queueCapacity),
		operations: map[string]*operation{},
	}
	go engine.run()
	return engine
}

func (engine *Engine) run() {
	for pending := range engine.queue {
		engine.execute(pending)
	}
}

// Enqueue validates a request and hands it to the worker, reporting the id the
// frontend needs to answer conflicts or cancel.
func (engine *Engine) Enqueue(kind Kind, sources []string, destination string) (string, error) {
	return engine.queueRequest(kind, sources, destination, false)
}

// enqueueReplay runs an operation again for a redo. It records no history of its
// own, because the step being redone goes back on the undo stack already.
func (engine *Engine) enqueueReplay(kind Kind, sources []string, destination string) error {
	_, err := engine.queueRequest(kind, sources, destination, true)
	return err
}

func (engine *Engine) queueRequest(kind Kind, sources []string, destination string, replaying bool) (string, error) {
	err := validateRequest(kind, sources, destination)
	if err != nil {
		return "", err
	}
	pending := engine.newOperation(kind, sources, destination)
	pending.replaying = replaying
	engine.publish(pending)
	engine.queue <- pending
	return pending.id, nil
}

// Resolve answers the conflict prompt an operation is blocked on.
func (engine *Engine) Resolve(operationID string, resolution Resolution, applyToAll bool) error {
	pending, found := engine.lookup(operationID)
	if !found {
		return ErrUnknownRequest
	}
	pending.answers <- conflictAnswer{resolution: resolution, applyToAll: applyToAll}
	return nil
}

func (engine *Engine) Cancel(operationID string) error {
	pending, found := engine.lookup(operationID)
	if !found {
		return ErrUnknownRequest
	}
	pending.cancel()
	return nil
}

func (engine *Engine) Pause(operationID string) error {
	pending, found := engine.lookup(operationID)
	if !found {
		return ErrUnknownRequest
	}
	pending.pause()
	engine.publish(pending)
	return nil
}

func (engine *Engine) Resume(operationID string) error {
	pending, found := engine.lookup(operationID)
	if !found {
		return ErrUnknownRequest
	}
	pending.resume()
	engine.publish(pending)
	return nil
}

// Rename moves one entry to a new name in its own directory.
func (engine *Engine) Rename(path, newName string) (string, error) {
	target, err := FreeName(parentOf(path), newName)
	if err != nil {
		return "", err
	}
	err = os.Rename(path, target)
	if err != nil {
		return "", err
	}
	engine.past.record(historyStep{
		label:  fmt.Sprintf("Rename to %s", newName),
		revert: func() error { return os.Rename(target, path) },
		replay: func() error { return os.Rename(path, target) },
	})
	return target, nil
}

// CreateFolder makes an empty directory and reports its path.
func (engine *Engine) CreateFolder(directory, name string) (string, error) {
	target, err := FreeName(directory, name)
	if err != nil {
		return "", err
	}
	err = os.Mkdir(target, 0o755)
	if err != nil {
		return "", err
	}
	engine.past.record(creationStep(fmt.Sprintf("Create folder %s", name), target, makeDirectory))
	return target, nil
}

// CreateFile makes an empty file and reports its path.
func (engine *Engine) CreateFile(directory, name string) (string, error) {
	target, err := FreeName(directory, name)
	if err != nil {
		return "", err
	}
	err = createEmptyFile(target)
	if err != nil {
		return "", err
	}
	engine.past.record(creationStep(fmt.Sprintf("Create file %s", name), target, createEmptyFile))
	return target, nil
}

// RestoreTrashed puts one trashed item back and reports where it landed.
func (engine *Engine) RestoreTrashed(trashPath string) (string, error) {
	restored, err := Restore(trashPath)
	if err != nil {
		return "", err
	}
	engine.past.record(restoreStep(restored, trashPath))
	return restored, nil
}

// Undo reverses the most recent reversible action and reports its label.
func (engine *Engine) Undo() (string, error) {
	step, found := engine.past.takeUndoable()
	if !found {
		return "", ErrNothingToUndo
	}
	err := step.revert()
	if err != nil {
		return "", err
	}
	engine.past.pushRedoable(step)
	return step.label, nil
}

// Redo performs the action the last undo took back and reports its label.
func (engine *Engine) Redo() (string, error) {
	step, found := engine.past.takeRedoable()
	if !found {
		return "", ErrNothingToRedo
	}
	err := step.replay()
	if err != nil {
		return "", err
	}
	engine.past.pushUndoable(step)
	return step.label, nil
}

// UndoLabel describes what Undo would reverse, or is empty when the stack is.
func (engine *Engine) UndoLabel() string {
	return engine.past.undoLabel()
}

// RedoLabel describes what Redo would perform, or is empty when the stack is.
func (engine *Engine) RedoLabel() string {
	return engine.past.redoLabel()
}

func (engine *Engine) execute(pending *operation) {
	pending.setStatus(StatusRunning)
	engine.publish(pending)
	pending.runAll()
	engine.finish(pending)
}

func (engine *Engine) finish(pending *operation) {
	pending.setStatus(pending.finalStatus())
	engine.recordOperation(pending)
	engine.emit(EventFinished, pending.snapshot())
	engine.forget(pending.id)
}

func (engine *Engine) publish(pending *operation) {
	engine.emit(EventProgress, pending.snapshot())
}

func (engine *Engine) newOperation(kind Kind, sources []string, destination string) *operation {
	engine.mutex.Lock()
	defer engine.mutex.Unlock()
	engine.sequence++
	pending := newOperation(engine, fmt.Sprintf("op-%d", engine.sequence), kind, sources, destination)
	engine.operations[pending.id] = pending
	return pending
}

func (engine *Engine) lookup(operationID string) (*operation, bool) {
	engine.mutex.Lock()
	defer engine.mutex.Unlock()
	pending, found := engine.operations[operationID]
	return pending, found
}

func (engine *Engine) forget(operationID string) {
	engine.mutex.Lock()
	defer engine.mutex.Unlock()
	delete(engine.operations, operationID)
}

// recordOperation files what a finished batch actually did. A replay is left
// out: the step it belongs to is already back on the undo stack.
func (engine *Engine) recordOperation(pending *operation) {
	if pending.replaying {
		return
	}
	step, reversible := pending.historyStep()
	if !reversible {
		return
	}
	engine.past.record(step)
}

// creationStep reverses a new entry by removing it, and puts it back by making
// it a second time.
func creationStep(label, target string, create func(path string) error) historyStep {
	return historyStep{
		label:  label,
		revert: func() error { return os.RemoveAll(target) },
		replay: func() error { return create(target) },
	}
}

// restoreStep follows the item into the trash and out again: taking a restore
// back trashes the item afresh, under a trash name only that moment knows.
func restoreStep(restored, trashPath string) historyStep {
	current := trashPath
	revert := func() error {
		trashed, err := Trash(restored)
		if err != nil {
			return err
		}
		current = trashed.TrashPath
		return nil
	}
	replay := func() error {
		_, err := Restore(current)
		return err
	}
	return historyStep{label: fmt.Sprintf("Restore %s", nameOf(restored)), revert: revert, replay: replay}
}

func makeDirectory(path string) error {
	return os.Mkdir(path, 0o755)
}

func createEmptyFile(path string) error {
	file, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o644)
	if err != nil {
		return err
	}
	return file.Close()
}

func validateRequest(kind Kind, sources []string, destination string) error {
	if !knownKind(kind) {
		return ErrUnknownRequest
	}
	if len(sources) == 0 {
		return ErrUnknownRequest
	}
	if kind == KindTrash || kind == KindDelete {
		return nil
	}
	if !isDirectory(destination) {
		return fmt.Errorf("destination %s is not a directory", destination)
	}
	return checkNotIntoItself(sources, destination)
}

func knownKind(kind Kind) bool {
	return kind == KindCopy || kind == KindMove || kind == KindTrash || kind == KindDelete
}

func checkNotIntoItself(sources []string, destination string) error {
	for _, source := range sources {
		if isDirectory(source) && containsPath(source, destination) {
			return ErrIntoItself
		}
	}
	return nil
}
