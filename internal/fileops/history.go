package fileops

import "sync"

// historyStep is one action the user can take back and put again: revert undoes
// what it did, replay does it a second time.
type historyStep struct {
	label  string
	revert func() error
	replay func() error
}

// history is what can be undone and what a preceding undo made redoable.
type history struct {
	mutex    sync.Mutex
	undoable []historyStep
	redoable []historyStep
}

// record files a fresh action. Anything the user had undone stops being
// redoable, because the state it would return to no longer exists.
func (past *history) record(step historyStep) {
	past.mutex.Lock()
	defer past.mutex.Unlock()
	past.undoable = append(past.undoable, step)
	past.redoable = nil
}

func (past *history) pushUndoable(step historyStep) {
	past.mutex.Lock()
	defer past.mutex.Unlock()
	past.undoable = append(past.undoable, step)
}

func (past *history) pushRedoable(step historyStep) {
	past.mutex.Lock()
	defer past.mutex.Unlock()
	past.redoable = append(past.redoable, step)
}

func (past *history) takeUndoable() (historyStep, bool) {
	past.mutex.Lock()
	defer past.mutex.Unlock()
	remaining, step, found := popStep(past.undoable)
	past.undoable = remaining
	return step, found
}

func (past *history) takeRedoable() (historyStep, bool) {
	past.mutex.Lock()
	defer past.mutex.Unlock()
	remaining, step, found := popStep(past.redoable)
	past.redoable = remaining
	return step, found
}

func (past *history) undoLabel() string {
	past.mutex.Lock()
	defer past.mutex.Unlock()
	return lastLabel(past.undoable)
}

func (past *history) redoLabel() string {
	past.mutex.Lock()
	defer past.mutex.Unlock()
	return lastLabel(past.redoable)
}

func popStep(stack []historyStep) ([]historyStep, historyStep, bool) {
	if len(stack) == 0 {
		return stack, historyStep{}, false
	}
	last := len(stack) - 1
	return stack[:last], stack[last], true
}

func lastLabel(stack []historyStep) string {
	if len(stack) == 0 {
		return ""
	}
	return stack[len(stack)-1].label
}
