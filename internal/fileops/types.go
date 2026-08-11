package fileops

import "errors"

// Kind is the family of a queued operation.
type Kind string

const (
	KindCopy     Kind = "copy"
	KindMove     Kind = "move"
	KindTrash    Kind = "trash"
	KindDelete   Kind = "delete"
	KindExtract  Kind = "extract"
	KindCompress Kind = "compress"
)

// Resolution is the caller's answer to a destination conflict.
type Resolution string

const (
	ResolutionReplace  Resolution = "replace"
	ResolutionSkip     Resolution = "skip"
	ResolutionKeepBoth Resolution = "keepBoth"
	ResolutionCancel   Resolution = "cancel"
)

// Status is the lifecycle position of one operation.
type Status string

const (
	StatusQueued    Status = "queued"
	StatusRunning   Status = "running"
	StatusPaused    Status = "paused"
	StatusDone      Status = "done"
	StatusCancelled Status = "cancelled"
)

// Event names the engine emits. The frontend subscribes to these.
const (
	EventProgress = "fileops:progress"
	EventConflict = "fileops:conflict"
	EventFinished = "fileops:finished"
)

var (
	// ErrCancelled unwinds a running operation without recording a failure.
	ErrCancelled = errors.New("operation cancelled")
	// ErrSkipped stops one source without failing the batch.
	ErrSkipped        = errors.New("item skipped")
	ErrUnknownRequest = errors.New("unknown operation")
	ErrNothingToUndo  = errors.New("nothing to undo")
	ErrNothingToRedo  = errors.New("nothing to redo")
	ErrIntoItself     = errors.New("a directory cannot be copied into itself")
)

// Failure records one item that could not be processed. A batch keeps going
// after a failure and reports them all at the end.
type Failure struct {
	Path    string `json:"path"`
	Message string `json:"message"`
}

// Progress is the snapshot the frontend renders for one operation. The byte
// totals are zero for kinds that move no data, where the item counter is the
// only honest measure.
type Progress struct {
	ID          string    `json:"id"`
	Kind        Kind      `json:"kind"`
	Status      Status    `json:"status"`
	Destination string    `json:"destination"`
	TotalItems  int       `json:"totalItems"`
	DoneItems   int       `json:"doneItems"`
	TotalBytes  int64     `json:"totalBytes"`
	DoneBytes   int64     `json:"doneBytes"`
	CurrentPath string    `json:"currentPath"`
	Failures    []Failure `json:"failures"`
}

// ConflictPrompt asks the frontend what to do about an occupied destination.
type ConflictPrompt struct {
	OperationID     string `json:"operationId"`
	SourcePath      string `json:"sourcePath"`
	DestinationPath string `json:"destinationPath"`
	SourceIsDir     bool   `json:"sourceIsDir"`
}

// Emitter delivers an event to the frontend. Keeping it a function leaves this
// package free of any Wails dependency.
type Emitter func(name string, data any)

// control is what a running copy or move sees of its operation: a way to honour
// pause and cancel, report where it is, and resolve a destination collision.
type control struct {
	waitWhilePaused func() error
	setCurrent      func(path string)
	resolveTarget   func(source, target string) (string, error)
	addBytes        func(count int64)
}
