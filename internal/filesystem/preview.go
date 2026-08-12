package filesystem

import (
	"bytes"
	"encoding/base64"
	"io"
	"os"
	"path/filepath"
	"strings"
	"unicode/utf8"

	"canopy/internal/audio"
	"canopy/internal/highlight"
	"canopy/internal/thumbnail"
)

// Preview is what the explorer can show about a single file without handing it
// to another application. Kind tells the frontend which field to render:
// "text", "image", "audio", "directory", or "unsupported".
// Tokens is the same text split into coloured runs, empty when the file has no
// highlighting, in which case the pane draws Text as it is.
// DataURL carries the drawing for an image, and the cover art for a sound file.
type Preview struct {
	Path      string            `json:"path"`
	Kind      string            `json:"kind"`
	Text      string            `json:"text"`
	Tokens    []highlight.Token `json:"tokens"`
	DataURL   string            `json:"dataUrl"`
	Truncated bool              `json:"truncated"`
	Size      int64             `json:"size"`
	Audio     audio.Details     `json:"audio"`
}

// Enough text to scroll through without pushing a huge file across the Wails
// bridge. Base64 inflates images by a third, so the image cap is the tighter one.
// Highlighting takes whatever the text limit leaves, since the canvas draws the
// lines on screen rather than all of them.
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
	if audio.Supports(absolutePath) {
		return audioPreview(absolutePath, size)
	}
	if thumbnail.Supports(absolutePath) {
		return renderedPreview(absolutePath, size)
	}
	return textPreview(absolutePath, size)
}

// A sound file has nothing to draw of its own, so the pane gets what the
// container says about it and the shape of the sound. Cover art comes from the
// same renderer images use, since embedded art is a video stream to ffmpeg.
func audioPreview(absolutePath string, size int64) (Preview, error) {
	details, readable := audio.Read(absolutePath)
	if !readable {
		return Preview{Path: absolutePath, Size: size, Kind: "unsupported"}, nil
	}
	return Preview{
		Path:    absolutePath,
		Size:    size,
		Kind:    "audio",
		Audio:   details,
		DataURL: thumbnail.RenderPreview(absolutePath),
	}, nil
}

// A PDF or a video has no bytes worth showing as they are, so the pane gets the
// same drawing the tiles use, at the size it has room for. A file nothing can
// draw falls back to the metadata every preview carries.
func renderedPreview(absolutePath string, size int64) (Preview, error) {
	dataURL := thumbnail.RenderPreview(absolutePath)
	if dataURL == "" {
		return Preview{Path: absolutePath, Size: size, Kind: "unsupported"}, nil
	}
	return Preview{Path: absolutePath, Size: size, Kind: "image", DataURL: dataURL}, nil
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
	preview.Tokens = highlight.Tokens(filepath.Base(absolutePath), preview.Text)
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
