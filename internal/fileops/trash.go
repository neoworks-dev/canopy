package fileops

import (
	"errors"
	"fmt"
	"net/url"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"
)

const (
	trashFilesDirectory = "files"
	trashInfoDirectory  = "info"
	trashInfoSuffix     = ".trashinfo"
	trashDateLayout     = "2006-01-02T15:04:05"
)

var ErrNotTrashed = errors.New("no trash record for this path")

// TrashedItem is one entry in a freedesktop.org trash directory.
type TrashedItem struct {
	Name         string `json:"name"`
	TrashPath    string `json:"trashPath"`
	OriginalPath string `json:"originalPath"`
	DeletedAt    string `json:"deletedAt"`
	Size         int64  `json:"size"`
	IsDir        bool   `json:"isDir"`
}

// trashLocation is one trash directory. topDirectory is empty for the home
// trash, where the spec records absolute original paths, and is the mount point
// for a per-volume trash, where it records paths relative to that mount.
type trashLocation struct {
	directory    string
	topDirectory string
}

// Trash moves a path into the trash directory that serves its filesystem,
// writing the .trashinfo record that makes it restorable by any compliant tool.
func Trash(path string) (TrashedItem, error) {
	absolutePath, err := filepath.Abs(path)
	if err != nil {
		return TrashedItem{}, err
	}
	location, err := trashLocationFor(absolutePath)
	if err != nil {
		return TrashedItem{}, err
	}
	name, infoFile, err := reserveTrashName(location.directory, filepath.Base(absolutePath))
	if err != nil {
		return TrashedItem{}, err
	}
	return completeTrashing(absolutePath, name, infoFile, location)
}

func completeTrashing(absolutePath, name string, infoFile *os.File, location trashLocation) (TrashedItem, error) {
	deletedAt := time.Now()
	recordedPath := recordedOriginalPath(absolutePath, location.topDirectory)
	err := writeTrashInfo(infoFile, recordedPath, deletedAt)
	if err != nil {
		return TrashedItem{}, abandonTrashRecord(location.directory, name, err)
	}
	trashPath := filepath.Join(location.directory, trashFilesDirectory, name)
	err = os.Rename(absolutePath, trashPath)
	if err != nil {
		return TrashedItem{}, abandonTrashRecord(location.directory, name, err)
	}
	return describeTrashed(trashPath, absolutePath, deletedAt), nil
}

// abandonTrashRecord drops the reserved .trashinfo file so a failed trashing
// leaves no orphan record behind.
func abandonTrashRecord(trashDirectory, name string, cause error) error {
	os.Remove(filepath.Join(trashDirectory, trashInfoDirectory, name+trashInfoSuffix))
	return cause
}

func describeTrashed(trashPath, originalPath string, deletedAt time.Time) TrashedItem {
	item := TrashedItem{
		Name:         filepath.Base(originalPath),
		TrashPath:    trashPath,
		OriginalPath: originalPath,
		DeletedAt:    deletedAt.Format(time.RFC3339),
	}
	info, err := os.Lstat(trashPath)
	if err != nil {
		return item
	}
	item.Size = info.Size()
	item.IsDir = info.IsDir()
	return item
}

// Restore puts a trashed item back where it came from, reporting the path it
// ended up at.
func Restore(trashPath string) (string, error) {
	infoPath := trashInfoPathFor(trashPath)
	originalPath, _, err := readTrashInfo(infoPath, topDirectoryForTrashPath(trashPath))
	if err != nil {
		return "", err
	}
	target, err := restoreTarget(originalPath)
	if err != nil {
		return "", err
	}
	err = os.Rename(trashPath, target)
	if err != nil {
		return "", err
	}
	return target, os.Remove(infoPath)
}

// restoreTarget never overwrites: something else may have taken the original
// name since the item was trashed.
func restoreTarget(originalPath string) (string, error) {
	parent := filepath.Dir(originalPath)
	err := os.MkdirAll(parent, 0o755)
	if err != nil {
		return "", err
	}
	if !exists(originalPath) {
		return originalPath, nil
	}
	name, err := uniqueName(parent, filepath.Base(originalPath))
	if err != nil {
		return "", err
	}
	return filepath.Join(parent, name), nil
}

// ListTrashed reads every trash directory reachable from this machine, newest
// deletion first.
func ListTrashed() ([]TrashedItem, error) {
	locations, err := trashLocations()
	if err != nil {
		return nil, err
	}
	items := []TrashedItem{}
	for _, location := range locations {
		items = append(items, itemsIn(location)...)
	}
	sortByDeletionDate(items)
	return items, nil
}

// EmptyTrash discards every trashed item permanently.
func EmptyTrash() error {
	locations, err := trashLocations()
	if err != nil {
		return err
	}
	for _, location := range locations {
		err = emptyLocation(location)
		if err != nil {
			return err
		}
	}
	return nil
}

func emptyLocation(location trashLocation) error {
	err := clearDirectoryContents(filepath.Join(location.directory, trashFilesDirectory))
	if err != nil {
		return err
	}
	return clearDirectoryContents(filepath.Join(location.directory, trashInfoDirectory))
}

func clearDirectoryContents(directory string) error {
	children, err := os.ReadDir(directory)
	if err != nil {
		return err
	}
	for _, child := range children {
		err = os.RemoveAll(filepath.Join(directory, child.Name()))
		if err != nil {
			return err
		}
	}
	return nil
}

func itemsIn(location trashLocation) []TrashedItem {
	filesDirectory := filepath.Join(location.directory, trashFilesDirectory)
	children, err := os.ReadDir(filesDirectory)
	if err != nil {
		return nil
	}
	items := []TrashedItem{}
	for _, child := range children {
		items = appendTrashedItem(items, filepath.Join(filesDirectory, child.Name()), location)
	}
	return items
}

// appendTrashedItem skips anything without a readable record: a file in files/
// with no .trashinfo is not restorable and does not belong in the list.
func appendTrashedItem(items []TrashedItem, trashPath string, location trashLocation) []TrashedItem {
	originalPath, deletedAt, err := readTrashInfo(trashInfoPathFor(trashPath), location.topDirectory)
	if err != nil {
		return items
	}
	item := describeTrashed(trashPath, originalPath, deletedAt)
	item.TrashPath = trashPath
	return append(items, item)
}

func sortByDeletionDate(items []TrashedItem) {
	sort.Slice(items, func(firstIndex, secondIndex int) bool {
		return items[firstIndex].DeletedAt > items[secondIndex].DeletedAt
	})
}

func reserveTrashName(trashDirectory, name string) (string, *os.File, error) {
	infoDirectory := filepath.Join(trashDirectory, trashInfoDirectory)
	for counter := 0; counter < maxNameAttempts; counter++ {
		candidate := trashCandidateName(name, counter)
		file, err := os.OpenFile(filepath.Join(infoDirectory, candidate+trashInfoSuffix), os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
		if err == nil {
			return candidate, file, nil
		}
		if !errors.Is(err, os.ErrExist) {
			return "", nil, err
		}
	}
	return "", nil, fmt.Errorf("no free trash name for %q", name)
}

func trashCandidateName(name string, counter int) string {
	if counter == 0 {
		return name
	}
	stem, extension := splitExtension(name)
	return fmt.Sprintf("%s (%d)%s", stem, counter, extension)
}

func writeTrashInfo(infoFile *os.File, recordedPath string, deletedAt time.Time) error {
	contents := fmt.Sprintf("[Trash Info]\nPath=%s\nDeletionDate=%s\n", escapeTrashPath(recordedPath), deletedAt.Format(trashDateLayout))
	_, err := infoFile.WriteString(contents)
	if err != nil {
		infoFile.Close()
		return err
	}
	return infoFile.Close()
}

// escapeTrashPath percent-encodes the original path the way the spec asks,
// leaving the separators readable.
func escapeTrashPath(path string) string {
	return (&url.URL{Path: path}).EscapedPath()
}

func readTrashInfo(infoPath, topDirectory string) (string, time.Time, error) {
	contents, err := os.ReadFile(infoPath)
	if err != nil {
		return "", time.Time{}, ErrNotTrashed
	}
	recordedPath, deletedAt := parseTrashInfo(string(contents))
	if recordedPath == "" {
		return "", time.Time{}, ErrNotTrashed
	}
	return absoluteOriginalPath(recordedPath, topDirectory), deletedAt, nil
}

func parseTrashInfo(contents string) (string, time.Time) {
	recordedPath := ""
	deletedAt := time.Time{}
	for _, line := range strings.Split(contents, "\n") {
		recordedPath = valueOrKeep(line, "Path=", recordedPath)
		deletedAt = deletionDateOrKeep(line, deletedAt)
	}
	return unescapeTrashPath(recordedPath), deletedAt
}

func valueOrKeep(line, prefix, current string) string {
	if !strings.HasPrefix(line, prefix) {
		return current
	}
	return strings.TrimSpace(strings.TrimPrefix(line, prefix))
}

func deletionDateOrKeep(line string, current time.Time) time.Time {
	raw := valueOrKeep(line, "DeletionDate=", "")
	if raw == "" {
		return current
	}
	parsed, err := time.ParseInLocation(trashDateLayout, raw, time.Local)
	if err != nil {
		return current
	}
	return parsed
}

func unescapeTrashPath(path string) string {
	unescaped, err := url.PathUnescape(path)
	if err != nil {
		return path
	}
	return unescaped
}

// recordedOriginalPath is absolute in the home trash and relative to the mount
// point in a per-volume trash, so the volume stays movable.
func recordedOriginalPath(absolutePath, topDirectory string) string {
	if topDirectory == "" {
		return absolutePath
	}
	relativePath, err := filepath.Rel(topDirectory, absolutePath)
	if err != nil {
		return absolutePath
	}
	return relativePath
}

func absoluteOriginalPath(recordedPath, topDirectory string) string {
	if filepath.IsAbs(recordedPath) {
		return recordedPath
	}
	return filepath.Join(topDirectory, recordedPath)
}

func trashInfoPathFor(trashPath string) string {
	trashDirectory := filepath.Dir(filepath.Dir(trashPath))
	return filepath.Join(trashDirectory, trashInfoDirectory, filepath.Base(trashPath)+trashInfoSuffix)
}

// topDirectoryForTrashPath recovers the mount point a per-volume trash belongs
// to. The home trash sits below the data directory and yields an empty string,
// which is what absoluteOriginalPath expects for absolute records.
func topDirectoryForTrashPath(trashPath string) string {
	trashDirectory := filepath.Dir(filepath.Dir(trashPath))
	name := filepath.Base(trashDirectory)
	if strings.HasPrefix(name, ".Trash-") {
		return filepath.Dir(trashDirectory)
	}
	if name == strconv.Itoa(os.Getuid()) {
		return filepath.Dir(filepath.Dir(trashDirectory))
	}
	return ""
}

func trashLocations() ([]trashLocation, error) {
	homeTrash, err := homeTrashDirectory()
	if err != nil {
		return nil, err
	}
	err = prepareTrashDirectory(homeTrash)
	if err != nil {
		return nil, err
	}
	return append([]trashLocation{{directory: homeTrash}}, volumeTrashLocations()...), nil
}

func trashLocationFor(path string) (trashLocation, error) {
	homeTrash, err := homeTrashDirectory()
	if err != nil {
		return trashLocation{}, err
	}
	err = prepareTrashDirectory(homeTrash)
	if err != nil {
		return trashLocation{}, err
	}
	sameVolume, err := onSameVolume(filepath.Dir(path), homeTrash)
	if err != nil {
		return trashLocation{}, err
	}
	if sameVolume {
		return trashLocation{directory: homeTrash}, nil
	}
	return volumeTrashLocationFor(path)
}

func volumeTrashLocationFor(path string) (trashLocation, error) {
	topDirectory, err := topDirectoryOf(path)
	if err != nil {
		return trashLocation{}, err
	}
	directory := volumeTrashDirectory(topDirectory)
	return trashLocation{directory: directory, topDirectory: topDirectory}, prepareTrashDirectory(directory)
}

// volumeTrashDirectory prefers the administrator-provided .Trash/$uid when it is
// present and sticky, and otherwise uses the .Trash-$uid the spec lets us create.
func volumeTrashDirectory(topDirectory string) string {
	shared := filepath.Join(topDirectory, ".Trash")
	if isStickyDirectory(shared) {
		return filepath.Join(shared, strconv.Itoa(os.Getuid()))
	}
	return filepath.Join(topDirectory, ".Trash-"+strconv.Itoa(os.Getuid()))
}

func isStickyDirectory(path string) bool {
	info, err := os.Lstat(path)
	if err != nil {
		return false
	}
	if !info.IsDir() {
		return false
	}
	return info.Mode()&os.ModeSticky != 0
}

// topDirectoryOf climbs until the filesystem changes, which is the mount point
// the volume trash belongs to.
func topDirectoryOf(path string) (string, error) {
	current := filepath.Dir(path)
	device, err := deviceOf(current)
	if err != nil {
		return "", err
	}
	for {
		parent := filepath.Dir(current)
		if parent == current || !isSameDevice(parent, device) {
			return current, nil
		}
		current = parent
	}
}

func isSameDevice(path string, device uint64) bool {
	parentDevice, err := deviceOf(path)
	if err != nil {
		return false
	}
	return parentDevice == device
}

func homeTrashDirectory() (string, error) {
	dataHome := os.Getenv("XDG_DATA_HOME")
	if dataHome != "" {
		return filepath.Join(dataHome, "Trash"), nil
	}
	homeDirectory, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(homeDirectory, ".local", "share", "Trash"), nil
}

func prepareTrashDirectory(trashDirectory string) error {
	err := os.MkdirAll(filepath.Join(trashDirectory, trashFilesDirectory), 0o700)
	if err != nil {
		return err
	}
	return os.MkdirAll(filepath.Join(trashDirectory, trashInfoDirectory), 0o700)
}
