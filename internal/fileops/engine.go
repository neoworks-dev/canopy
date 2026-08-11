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

	mutex      sync.Mutex
	operations map[string]*operation
	undoStack  []undoStep
	sequence   int
}

// undoStep is one reversible action. Permanent deletion never produces one.
type undoStep struct {
	label  string
	revert func() error
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
	err := validateRequest(kind, sources, destination)
	if err != nil {
		return "", err
	}
	pending := engine.newOperation(kind, sources, destination)
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
	engine.pushUndo(undoStep{
		label:  fmt.Sprintf("Rename to %s", newName),
		revert: func() error { return os.Rename(target, path) },
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
	engine.pushUndo(removalUndo(fmt.Sprintf("Create folder %s", name), target))
	return target, nil
}

// CreateFile makes an empty file and reports its path.
func (engine *Engine) CreateFile(directory, name string) (string, error) {
	target, err := FreeName(directory, name)
	if err != nil {
		return "", err
	}
	file, err := os.OpenFile(target, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o644)
	if err != nil {
		return "", err
	}
	err = file.Close()
	if err != nil {
		return "", err
	}
	engine.pushUndo(removalUndo(fmt.Sprintf("Create file %s", name), target))
	return target, nil
}

// RestoreTrashed puts one trashed item back and reports where it landed.
func (engine *Engine) RestoreTrashed(trashPath string) (string, error) {
	restored, err := Restore(trashPath)
	if err != nil {
		return "", err
	}
	engine.pushUndo(undoStep{
		label:  fmt.Sprintf("Restore %s", nameOf(restored)),
		revert: func() error { return discardRestored(restored) },
	})
	return restored, nil
}

// Undo reverses the most recent reversible action and reports its label.
func (engine *Engine) Undo() (string, error) {
	step, found := engine.popUndo()
	if !found {
		return "", ErrNothingToUndo
	}
	err := step.revert()
	if err != nil {
		return "", err
	}
	return step.label, nil
}

// UndoLabel describes what Undo would reverse, or is empty when the stack is.
func (engine *Engine) UndoLabel() string {
	engine.mutex.Lock()
	defer engine.mutex.Unlock()
	if len(engine.undoStack) == 0 {
		return ""
	}
	return engine.undoStack[len(engine.undoStack)-1].label
}

func (engine *Engine) execute(pending *operation) {
	pending.setStatus(StatusRunning)
	engine.publish(pending)
	pending.runAll()
	engine.finish(pending)
}

func (engine *Engine) finish(pending *operation) {
	pending.setStatus(pending.finalStatus())
	engine.pushOperationUndo(pending)
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

func (engine *Engine) pushOperationUndo(pending *operation) {
	step, reversible := pending.undoStep()
	if !reversible {
		return
	}
	engine.pushUndo(step)
}

func (engine *Engine) pushUndo(step undoStep) {
	engine.mutex.Lock()
	defer engine.mutex.Unlock()
	engine.undoStack = append(engine.undoStack, step)
}

func (engine *Engine) popUndo() (undoStep, bool) {
	engine.mutex.Lock()
	defer engine.mutex.Unlock()
	if len(engine.undoStack) == 0 {
		return undoStep{}, false
	}
	step := engine.undoStack[len(engine.undoStack)-1]
	engine.undoStack = engine.undoStack[:len(engine.undoStack)-1]
	return step, true
}

func removalUndo(label, target string) undoStep {
	return undoStep{
		label:  label,
		revert: func() error { return os.RemoveAll(target) },
	}
}

// discardRestored sends a restored item straight back to the trash, which is the
// only honest inverse of a restore.
func discardRestored(path string) error {
	_, err := Trash(path)
	return err
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
