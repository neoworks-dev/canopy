package launcher

import (
	"mime"
	"net/http"
	"os"
	"path/filepath"
	"strings"
)

const (
	directoryMimeType = "inode/directory"
	fallbackMimeType  = "application/octet-stream"
	sniffLimit        = 512
)

// MimeTypeOf names the type of a path, preferring the extension because it is
// cheap and matches what the XDG associations are keyed on, and sniffing the
// contents only when the extension says nothing.
func MimeTypeOf(path string) string {
	info, err := os.Stat(path)
	if err == nil && info.IsDir() {
		return directoryMimeType
	}
	byExtension := mime.TypeByExtension(filepath.Ext(path))
	if byExtension != "" {
		return stripMimeParameters(byExtension)
	}
	return sniffMimeType(path)
}

func sniffMimeType(path string) string {
	file, err := os.Open(path)
	if err != nil {
		return fallbackMimeType
	}
	defer file.Close()
	header := make([]byte, sniffLimit)
	readCount, err := file.Read(header)
	if err != nil && readCount == 0 {
		return fallbackMimeType
	}
	return stripMimeParameters(http.DetectContentType(header[:readCount]))
}

func stripMimeParameters(mimeType string) string {
	semicolon := strings.Index(mimeType, ";")
	if semicolon < 0 {
		return strings.TrimSpace(mimeType)
	}
	return strings.TrimSpace(mimeType[:semicolon])
}
