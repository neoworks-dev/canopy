// Package thumbnail renders preview images for the files a listing shows and
// files them in the shared thumbnail cache every desktop file manager reads.
//
// Rendering is best effort throughout: a file type nothing here can draw, a
// helper program that is not installed and a file that fails to decode all come
// back as an empty thumbnail, because none of them is worth putting an error in
// front of somebody browsing a folder.
package thumbnail

import (
	"context"
	"encoding/base64"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// Rendering a folder of video files would otherwise start one helper process
// per visible tile.
const concurrentRenders = 4

// Long enough for a helper to open a large video, short enough that a file it
// cannot make sense of does not hold a slot for the rest of the session.
const renderTimeout = 20 * time.Second

var renderSlots = make(chan struct{}, concurrentRenders)

// Thumbnail is one preview image, ready to be used as an image source. An empty
// DataURL means there is no preview for this file, which is not a failure.
type Thumbnail struct {
	Path    string `json:"path"`
	DataURL string `json:"dataUrl"`
}

// Render returns the preview image for one file, from the shared cache when it
// holds a current one and by rendering it otherwise.
func Render(absolutePath string) Thumbnail {
	empty := Thumbnail{Path: absolutePath}
	renderer := rendererFor(strings.ToLower(filepath.Ext(absolutePath)))
	if renderer == nil {
		return empty
	}
	info, err := os.Stat(absolutePath)
	if err != nil || info.IsDir() {
		return empty
	}
	return Thumbnail{Path: absolutePath, DataURL: dataURLFor(absolutePath, info, renderer)}
}

// Supports reports whether a path is one Render could produce an image for,
// which lets a caller skip the request entirely for plain files.
func Supports(path string) bool {
	return CanRender(strings.ToLower(filepath.Ext(path)))
}

func dataURLFor(absolutePath string, info os.FileInfo, render renderer) string {
	uri := sourceURI(absolutePath)
	modifiedAt := modifiedStamp(info)
	path, err := cachePath(uri)
	if err != nil {
		return ""
	}
	cached := readCached(path, uri, modifiedAt)
	if cached != nil {
		return encodeDataURL(cached)
	}
	return encodeDataURL(renderAndCache(absolutePath, path, uri, modifiedAt, render))
}

func renderAndCache(
	absolutePath string,
	path string,
	uri string,
	modifiedAt string,
	render renderer,
) []byte {
	image := renderWithinLimits(absolutePath, render)
	if image == nil {
		return nil
	}
	annotated, err := withThumbnailMetadata(image, uri, modifiedAt)
	if err != nil {
		return nil
	}
	writeCached(path, annotated)
	return annotated
}

func renderWithinLimits(absolutePath string, render renderer) []byte {
	renderSlots <- struct{}{}
	defer func() { <-renderSlots }()

	ctx, cancel := context.WithTimeout(context.Background(), renderTimeout)
	defer cancel()
	image, err := render(ctx, absolutePath)
	if err != nil {
		return nil
	}
	return image
}

func encodeDataURL(image []byte) string {
	if image == nil {
		return ""
	}
	return "data:image/png;base64," + base64.StdEncoding.EncodeToString(image)
}
