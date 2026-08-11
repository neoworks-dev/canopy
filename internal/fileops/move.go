package fileops

import (
	"errors"
	"os"
	"path/filepath"
	"syscall"
)

// moveInto moves source into destinationDirectory and reports where it landed,
// which is what the undo step needs to put it back.
func moveInto(source, destinationDirectory string, control *control) (string, error) {
	target, err := control.resolveTarget(source, filepath.Join(destinationDirectory, filepath.Base(source)))
	if err != nil {
		return "", err
	}
	return target, moveResolved(source, target, control)
}

func moveResolved(source, target string, control *control) error {
	err := control.waitWhilePaused()
	if err != nil {
		return err
	}
	control.setCurrent(source)
	if isDirectory(source) && isDirectory(target) {
		return mergeDirectory(source, target, control)
	}
	err = clearDirectoryTarget(target)
	if err != nil {
		return err
	}
	return renameOrCopy(source, target, control)
}

// clearDirectoryTarget removes a directory standing where a non-directory is
// about to land, a combination rename refuses.
func clearDirectoryTarget(target string) error {
	if !isDirectory(target) {
		return nil
	}
	return os.RemoveAll(target)
}

// renameOrCopy falls back to copy-then-delete when source and target live on
// different filesystems, where rename cannot work.
func renameOrCopy(source, target string, control *control) error {
	err := os.Rename(source, target)
	if err == nil {
		return nil
	}
	if !errors.Is(err, syscall.EXDEV) {
		return err
	}
	err = copyResolved(source, target, control)
	if err != nil {
		return err
	}
	return os.RemoveAll(source)
}

func mergeDirectory(source, target string, control *control) error {
	children, err := os.ReadDir(source)
	if err != nil {
		return err
	}
	err = moveChildren(source, target, children, control)
	if err != nil {
		return err
	}
	return removeEmptied(source)
}

func moveChildren(source, target string, children []os.DirEntry, control *control) error {
	for _, child := range children {
		_, err := moveInto(filepath.Join(source, child.Name()), target, control)
		if err != nil {
			return err
		}
	}
	return nil
}

// removeEmptied tolerates leftovers: a skipped child legitimately keeps the
// source directory alive after a merge.
func removeEmptied(source string) error {
	err := os.Remove(source)
	if errors.Is(err, syscall.ENOTEMPTY) {
		return nil
	}
	return err
}
