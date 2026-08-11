// Package mediaserver hands local files to the webview over HTTP.
//
// A data URL cannot carry a track: the whole file would have to cross the
// bridge before a single second played, and seeking needs ranges the browser can
// ask for. Serving the file instead gives the audio and video elements what they
// expect, and http.ServeFile answers the range requests on its own.
package mediaserver

import (
	"net/http"
	"os"

	"file-explorer-neoworks/internal/filesystem"
)

// MediaRoute is the path the frontend asks for a file on, with the file's own
// path in the "path" query parameter.
const MediaRoute = "/media"

// Handler serves the files the webview asks for by path. It is reached only
// from the webview, which is the same process boundary the bound methods sit
// behind, so a request is as trusted as a method call.
func Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc(MediaRoute, serveFile)
	return mux
}

func serveFile(writer http.ResponseWriter, request *http.Request) {
	absolutePath, err := filesystem.ResolvePath(request.URL.Query().Get("path"))
	if err != nil {
		http.Error(writer, "no such file", http.StatusBadRequest)
		return
	}
	info, err := os.Stat(absolutePath)
	if err != nil || !info.Mode().IsRegular() {
		http.Error(writer, "no such file", http.StatusNotFound)
		return
	}
	http.ServeFile(writer, request, absolutePath)
}
