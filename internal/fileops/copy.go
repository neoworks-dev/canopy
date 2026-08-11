package fileops

import (
	"io"
	"io/fs"
	"os"
	"path/filepath"
)

const copyBufferSize = 256 * 1024

// copyInto copies source into destinationDirectory under its own base name,
// asking control what to do when the target is already taken.
func copyInto(source, destinationDirectory string, control *control) error {
	target, err := control.resolveTarget(source, filepath.Join(destinationDirectory, filepath.Base(source)))
	if err != nil {
		return err
	}
	return copyResolved(source, target, control)
}

func copyResolved(source, target string, control *control) error {
	err := control.waitWhilePaused()
	if err != nil {
		return err
	}
	control.setCurrent(source)
	info, err := os.Lstat(source)
	if err != nil {
		return err
	}
	if info.Mode()&os.ModeSymlink != 0 {
		return copySymlink(source, target)
	}
	if info.IsDir() {
		return copyDirectory(source, target, info, control)
	}
	return copyFile(source, target, info, control)
}

func copySymlink(source, target string) error {
	linkTarget, err := os.Readlink(source)
	if err != nil {
		return err
	}
	err = os.RemoveAll(target)
	if err != nil {
		return err
	}
	return os.Symlink(linkTarget, target)
}

// copyDirectory merges into an existing target rather than replacing it, so a
// conflict on a directory never destroys entries the source does not carry.
func copyDirectory(source, target string, info fs.FileInfo, control *control) error {
	err := os.MkdirAll(target, info.Mode().Perm())
	if err != nil {
		return err
	}
	children, err := os.ReadDir(source)
	if err != nil {
		return err
	}
	err = copyChildren(source, target, children, control)
	if err != nil {
		return err
	}
	return applyMetadata(target, info)
}

func copyChildren(source, target string, children []os.DirEntry, control *control) error {
	for _, child := range children {
		err := copyInto(filepath.Join(source, child.Name()), target, control)
		if err != nil {
			return err
		}
	}
	return nil
}

func copyFile(source, target string, info fs.FileInfo, control *control) error {
	sourceFile, err := os.Open(source)
	if err != nil {
		return err
	}
	defer sourceFile.Close()
	targetFile, err := os.OpenFile(target, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, info.Mode().Perm())
	if err != nil {
		return err
	}
	err = writeContents(targetFile, sourceFile, control)
	if err != nil {
		return abandonTarget(targetFile, target, err)
	}
	err = targetFile.Close()
	if err != nil {
		return err
	}
	return applyMetadata(target, info)
}

// abandonTarget removes a half-written file so a cancelled or failed copy does
// not leave a truncated one behind.
func abandonTarget(targetFile *os.File, target string, cause error) error {
	targetFile.Close()
	os.Remove(target)
	return cause
}

// writeContents copies in chunks so pause and cancel take effect during a large
// file rather than only between files, and so the byte counter advances inside
// one file instead of jumping when it completes.
func writeContents(target io.Writer, source io.Reader, control *control) error {
	buffer := make([]byte, copyBufferSize)
	for {
		err := control.waitWhilePaused()
		if err != nil {
			return err
		}
		readCount, readErr := source.Read(buffer)
		err = writeChunk(target, buffer[:readCount])
		if err != nil {
			return err
		}
		control.addBytes(int64(readCount))
		if readErr != nil {
			return endOfContents(readErr)
		}
	}
}

func writeChunk(target io.Writer, chunk []byte) error {
	if len(chunk) == 0 {
		return nil
	}
	_, err := target.Write(chunk)
	return err
}

func endOfContents(readErr error) error {
	if readErr == io.EOF {
		return nil
	}
	return readErr
}

func applyMetadata(path string, info fs.FileInfo) error {
	err := os.Chmod(path, info.Mode().Perm())
	if err != nil {
		return err
	}
	modifiedAt := info.ModTime()
	return os.Chtimes(path, modifiedAt, modifiedAt)
}
