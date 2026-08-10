package filesystem

import (
	"bytes"
	"encoding/base64"
	"io"
	"os"
	"path/filepath"
	"strings"
	"unicode/utf8"
)

// Preview is what the explorer can show about a single file without handing it
// to another application. Kind tells the frontend which field to render:
// "text", "image", "directory", or "unsupported".
type Preview struct {
	Path      string `json:"path"`
	Kind      string `json:"kind"`
	Text      string `json:"text"`
	DataURL   string `json:"dataUrl"`
	Truncated bool   `json:"truncated"`
	Size      int64  `json:"size"`
}

// Enough text to scroll through without pushing a huge file across the Wails
// bridge. Base64 inflates images by a third, so the image cap is the tighter one.
const (
	textPreviewLimit  = 128 * 1024
	imagePreviewLimit = 4 * 1024 * 1024
)

var imageMediaTypes = map[string]string{
	".png":  "image/png",
	".jpg":  "image/jpeg",
	".jpeg": "image/jpeg",
	".gif":  "image/gif",
	".webp": "image/webp",
	".avif": "image/avif",
	".bmp":  "image/bmp",
	".ico":  "image/x-icon",
	".svg":  "image/svg+xml",
}

// ReadPreview inspects a single path. Anything too large or binary comes back
// as an unsupported preview rather than an error, since the pane still shows
// the file's metadata.
func ReadPreview(requestedPath string) (Preview, error) {
	absolutePath, err := ResolvePath(requestedPath)
	if err != nil {
		return Preview{}, err
	}
	info, err := os.Stat(absolutePath)
	if err != nil {
		return Preview{}, err
	}
	if info.IsDir() {
		return Preview{Path: absolutePath, Kind: "directory"}, nil
	}
	return previewOfFile(absolutePath, info.Size())
}

func previewOfFile(absolutePath string, size int64) (Preview, error) {
	extension := strings.ToLower(filepath.Ext(absolutePath))
	mediaType, isImage := imageMediaTypes[extension]
	if isImage {
		return imagePreview(absolutePath, size, mediaType)
	}
	return textPreview(absolutePath, size)
}

func imagePreview(absolutePath string, size int64, mediaType string) (Preview, error) {
	preview := Preview{Path: absolutePath, Size: size, Kind: "unsupported"}
	if size > imagePreviewLimit {
		return preview, nil
	}
	contents, err := os.ReadFile(absolutePath)
	if err != nil {
		return Preview{}, err
	}
	preview.Kind = "image"
	preview.DataURL = "data:" + mediaType + ";base64," + base64.StdEncoding.EncodeToString(contents)
	return preview, nil
}

func textPreview(absolutePath string, size int64) (Preview, error) {
	preview := Preview{Path: absolutePath, Size: size, Kind: "unsupported"}
	contents, err := readAtMost(absolutePath, textPreviewLimit)
	if err != nil {
		return Preview{}, err
	}
	readable := dropPartialRune(contents)
	if !isTextual(readable) {
		return preview, nil
	}
	preview.Kind = "text"
	preview.Text = string(readable)
	preview.Truncated = size > int64(len(contents))
	return preview, nil
}

func readAtMost(absolutePath string, limit int64) ([]byte, error) {
	file, err := os.Open(absolutePath)
	if err != nil {
		return nil, err
	}
	defer file.Close()
	return io.ReadAll(io.LimitReader(file, limit))
}

// isTextual applies the heuristic most editors use: valid UTF-8 with no NUL
// bytes is text, everything else is binary.
func isTextual(contents []byte) bool {
	if bytes.IndexByte(contents, 0) >= 0 {
		return false
	}
	return utf8.Valid(contents)
}

// dropPartialRune removes a multi-byte rune the read limit cut in half, so a
// truncated text file is not mistaken for binary.
func dropPartialRune(contents []byte) []byte {
	start := lastRuneStart(contents)
	if start < 0 || utf8.Valid(contents[start:]) {
		return contents
	}
	return contents[:start]
}

func lastRuneStart(contents []byte) int {
	lowestIndex := len(contents) - utf8.UTFMax
	for index := len(contents) - 1; index >= 0 && index >= lowestIndex; index-- {
		if utf8.RuneStart(contents[index]) {
			return index
		}
	}
	return -1
}
