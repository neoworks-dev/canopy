package main

import (
	"context"
	"os"
	"sync"

	"github.com/wailsapp/wails/v2/pkg/runtime"

	"file-explorer-neoworks/internal/clipboard"
	"file-explorer-neoworks/internal/fileops"
	"file-explorer-neoworks/internal/filesystem"
	"file-explorer-neoworks/internal/launcher"
	"file-explorer-neoworks/internal/settings"
	"file-explorer-neoworks/internal/watcher"
	"file-explorer-neoworks/internal/zoxide"
)

// Enough ranked directories to pick from without turning the prompt into a list
// to read.
const zoxideResultLimit = 12

// EventDirectoryChanged carries the path of a watched directory whose contents
// changed underneath the listing.
const EventDirectoryChanged = "watch:changed"

// FileService is the Wails-bound bridge between the frontend and the filesystem,
// operation engine and launcher packages.
type FileService struct {
	engine   *fileops.Engine
	watcher  *watcher.Watcher
	settings *settings.Store
	zoxide   *zoxide.Database

	// The last state the frontend saved, so the window geometry captured at
	// shutdown can be written alongside it.
	settingsMutex sync.Mutex
	current       settings.Settings

	contextMutex sync.RWMutex
	context      context.Context
}

func NewFileService() *FileService {
	service := &FileService{}
	service.engine = fileops.New(service.emit)
	service.watcher = watcher.New(service.emitDirectoryChanged)
	service.settings = settings.Open()
	service.current = service.settings.Load()
	service.zoxide = zoxide.New()
	return service
}

// ZoxideQuery ranks the directories zoxide remembers against a query, leaving
// out the one the window is already showing. An empty result covers both "no
// match" and "zoxide is not installed": neither is a failure the user has to be
// told about, so the prompt simply has nothing to offer.
func (service *FileService) ZoxideQuery(query string, exclude string) []zoxide.Entry {
	return service.zoxide.Query(query, exclude, zoxideResultLimit)
}

// ZoxideAvailable reports whether zoxide is installed, so a prompt that could
// never answer can say why instead of looking broken.
func (service *FileService) ZoxideAvailable() bool {
	return service.zoxide.Available()
}

func (service *FileService) emitDirectoryChanged(path string) {
	service.emit(EventDirectoryChanged, path)
}

// startup captures the context Wails needs to deliver events to the frontend.
func (service *FileService) startup(applicationContext context.Context) {
	service.contextMutex.Lock()
	defer service.contextMutex.Unlock()
	service.context = applicationContext
}

// emit drops events raised before the window exists rather than panicking on a
// missing context.
func (service *FileService) emit(name string, data any) {
	service.contextMutex.RLock()
	defer service.contextMutex.RUnlock()
	if service.context == nil {
		return
	}
	runtime.EventsEmit(service.context, name, data)
}

// ListDirectory reads the directory at path in the given order. An empty path
// means the home directory.
func (service *FileService) ListDirectory(path string, order filesystem.Order) (filesystem.Listing, error) {
	return filesystem.ReadDirectory(path, order)
}

// LoadSettings reports the preferences this launch started from.
func (service *FileService) LoadSettings() settings.Settings {
	service.settingsMutex.Lock()
	defer service.settingsMutex.Unlock()
	return service.current
}

// SaveSettings records the frontend's state. The write itself is debounced, so
// saving on every change costs nothing.
func (service *FileService) SaveSettings(next settings.Settings) {
	service.settingsMutex.Lock()
	// The frontend has no business knowing where the window sits, so it keeps
	// whatever geometry the last shutdown recorded.
	next.Window = service.current.Window
	service.current = next
	service.settingsMutex.Unlock()
	service.settings.Save(next)
}

// captureWindow records the window's geometry and writes everything out, which
// is the last thing that happens before the window closes.
func (service *FileService) captureWindow(applicationContext context.Context) {
	width, height := runtime.WindowGetSize(applicationContext)
	x, y := runtime.WindowGetPosition(applicationContext)
	service.settingsMutex.Lock()
	service.current.Window = settings.Window{Width: width, Height: height, X: x, Y: y}
	saved := service.current
	service.settingsMutex.Unlock()
	service.settings.Save(saved)
	service.settings.Flush()
}

// WatchDirectory follows one directory and emits EventDirectoryChanged when its
// contents change. It replaces any previous watch.
func (service *FileService) WatchDirectory(path string) error {
	absolutePath, err := filesystem.ResolvePath(path)
	if err != nil {
		return err
	}
	return service.watcher.Watch(absolutePath)
}

// StopWatching drops the current watch, for the panes that show no directory.
func (service *FileService) StopWatching() {
	service.watcher.Stop()
}

// HomeDirectory is the frontend's starting location.
func (service *FileService) HomeDirectory() (string, error) {
	return os.UserHomeDir()
}

// PreviewFile reads as much of one file as the preview pane can show.
func (service *FileService) PreviewFile(path string) (filesystem.Preview, error) {
	return filesystem.ReadPreview(path)
}

// Places lists the sidebar shortcuts to well-known directories.
func (service *FileService) Places() []filesystem.Place {
	return filesystem.Places()
}

// StartOperation queues a copy, move, trash or delete and reports the id that
// identifies it in progress events and conflict prompts.
func (service *FileService) StartOperation(kind string, sources []string, destination string) (string, error) {
	return service.engine.Enqueue(fileops.Kind(kind), sources, destination)
}

// ResolveConflict answers the prompt an operation is blocked on.
func (service *FileService) ResolveConflict(operationID string, resolution string, applyToAll bool) error {
	return service.engine.Resolve(operationID, fileops.Resolution(resolution), applyToAll)
}

func (service *FileService) CancelOperation(operationID string) error {
	return service.engine.Cancel(operationID)
}

func (service *FileService) PauseOperation(operationID string) error {
	return service.engine.Pause(operationID)
}

func (service *FileService) ResumeOperation(operationID string) error {
	return service.engine.Resume(operationID)
}

// Undo reverses the most recent reversible action and reports its label.
func (service *FileService) Undo() (string, error) {
	return service.engine.Undo()
}

// UndoLabel describes what Undo would reverse, empty when there is nothing.
func (service *FileService) UndoLabel() string {
	return service.engine.UndoLabel()
}

// RenameEntry renames one entry inside its own directory.
func (service *FileService) RenameEntry(path string, newName string) (string, error) {
	return service.engine.Rename(path, newName)
}

func (service *FileService) CreateFolder(directory string, name string) (string, error) {
	return service.engine.CreateFolder(directory, name)
}

func (service *FileService) CreateFile(directory string, name string) (string, error) {
	return service.engine.CreateFile(directory, name)
}

// ValidateEntryName reports why a name cannot be used, before anything is
// written, so the inline editor can refuse it.
func (service *FileService) ValidateEntryName(name string) error {
	return fileops.ValidateName(name)
}

// SuggestName picks a free name in a directory, which is how "New folder"
// avoids colliding with one that already exists.
func (service *FileService) SuggestName(directory string, name string) string {
	return fileops.SuggestName(directory, name)
}

func (service *FileService) ListTrash() ([]fileops.TrashedItem, error) {
	return fileops.ListTrashed()
}

func (service *FileService) RestoreFromTrash(trashPath string) (string, error) {
	return service.engine.RestoreTrashed(trashPath)
}

func (service *FileService) EmptyTrash() error {
	return fileops.EmptyTrash()
}

// OpenPath launches a path with the remembered override for its type, or the
// system default.
func (service *FileService) OpenPath(path string) error {
	return launcher.Open(path)
}

// ApplicationsFor lists the desktop entries registered for a path's type.
func (service *FileService) ApplicationsFor(path string) []launcher.Application {
	return launcher.ApplicationsFor(path)
}

func (service *FileService) OpenWith(path string, applicationID string) error {
	return launcher.OpenWith(path, applicationID)
}

// SetDefaultApplication remembers one application for the type of path.
func (service *FileService) SetDefaultApplication(path string, applicationID string) error {
	return launcher.SetDefault(path, applicationID)
}

// CopyPathsToClipboard publishes a selection as a text/uri-list so other
// applications can paste it.
func (service *FileService) CopyPathsToClipboard(paths []string) error {
	return clipboard.WritePaths(paths)
}

// ReadClipboardPaths reads a text/uri-list another application published.
func (service *FileService) ReadClipboardPaths() ([]string, error) {
	return clipboard.ReadPaths()
}
