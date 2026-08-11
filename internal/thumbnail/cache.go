package thumbnail

import (
	"crypto/md5"
	"encoding/hex"
	"net/url"
	"os"
	"path/filepath"
	"strconv"
)

// The shared thumbnail directory every desktop file manager reads, so an image
// rendered here is one Dolphin and Nautilus do not have to render again.
const (
	cacheDirectoryName = "thumbnails"
	sizeDirectoryName  = "large"
)

// sourceURI is the identity a cached thumbnail is filed under, and the value
// stored in the image so a hash collision can be told apart from a hit.
func sourceURI(absolutePath string) string {
	location := url.URL{Scheme: "file", Path: absolutePath}
	return location.String()
}

func modifiedStamp(info os.FileInfo) string {
	return strconv.FormatInt(info.ModTime().Unix(), 10)
}

// cachePath is the standard location for one source: the MD5 of its URI, which
// is a naming rule rather than a security claim.
func cachePath(uri string) (string, error) {
	directory, err := cacheDirectory()
	if err != nil {
		return "", err
	}
	sum := md5.Sum([]byte(uri))
	return filepath.Join(directory, hex.EncodeToString(sum[:])+".png"), nil
}

func cacheDirectory() (string, error) {
	base, err := os.UserCacheDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(base, cacheDirectoryName, sizeDirectoryName), nil
}

// readCached returns a stored thumbnail that was made from this exact source at
// this exact modification time, and nothing otherwise.
func readCached(path string, uri string, modifiedAt string) []byte {
	stored, err := os.ReadFile(path)
	if err != nil {
		return nil
	}
	storedURI, storedModifiedAt := thumbnailMetadata(stored)
	if storedURI != uri || storedModifiedAt != modifiedAt {
		return nil
	}
	return stored
}

// writeCached stages the image through a temporary file, so a reader that
// arrives mid-write sees either the old thumbnail or the new one. A cache that
// cannot be written is not worth failing a render over.
func writeCached(path string, image []byte) {
	if os.MkdirAll(filepath.Dir(path), 0o700) != nil {
		return
	}
	temporary, err := os.CreateTemp(filepath.Dir(path), ".thumbnail-*")
	if err != nil {
		return
	}
	defer os.Remove(temporary.Name())
	if !writeAndClose(temporary, image) {
		return
	}
	os.Rename(temporary.Name(), path)
}

func writeAndClose(file *os.File, image []byte) bool {
	_, err := file.Write(image)
	if file.Close() != nil || err != nil {
		return false
	}
	return os.Chmod(file.Name(), 0o600) == nil
}
