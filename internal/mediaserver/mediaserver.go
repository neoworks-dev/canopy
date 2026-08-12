// Package mediaserver hands local files to the webview over HTTP.
//
// A data URL cannot carry a track: the whole file would have to cross the bridge
// before a single second played, and seeking needs ranges the browser can ask
// for.
//
// It is a listener of its own rather than a route on the asset server because
// Wails serves that one from a single goroutine on Linux. A media element holds
// its request open for as long as it is loaded, which would block every asset
// queued behind it for the rest of the session.
package mediaserver

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"net"
	"net/http"
	"net/url"
	"os"

	"canopy/internal/filesystem"
)

// mediaRoute is the path a file is asked for on, with the file's own path in the
// "path" query parameter.
const mediaRoute = "/media"

// The listener is on the loopback interface, where any process on the machine
// could reach it, so a request carries a secret minted at startup.
const tokenBytes = 32

// Server is the running media listener. A zero Server serves nothing, which is
// what the frontend sees as "this file cannot be played".
type Server struct {
	baseURL string
	token   string
}

// Start opens a listener on a port the system picks and serves until the process
// ends. A failure to listen is not worth stopping the explorer for: previews
// still draw, and only playback is lost.
func Start() Server {
	token := newToken()
	if token == "" {
		return Server{}
	}
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return Server{}
	}
	server := Server{baseURL: fmt.Sprintf("http://%s", listener.Addr().String()), token: token}
	go http.Serve(listener, server.handler())
	return server
}

// MediaURL is the address the webview plays one file from, or an empty string
// when no listener came up.
func (server Server) MediaURL(absolutePath string) string {
	if server.baseURL == "" {
		return ""
	}
	query := url.Values{"path": {absolutePath}, "token": {server.token}}
	return server.baseURL + mediaRoute + "?" + query.Encode()
}

func (server Server) handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc(mediaRoute, server.serveFile)
	return mux
}

func (server Server) serveFile(writer http.ResponseWriter, request *http.Request) {
	if request.URL.Query().Get("token") != server.token {
		http.Error(writer, "not for you", http.StatusForbidden)
		return
	}
	absolutePath, err := filesystem.ResolvePath(request.URL.Query().Get("path"))
	if err != nil {
		http.Error(writer, "no such file", http.StatusBadRequest)
		return
	}
	serveRegularFile(writer, request, absolutePath)
}

func serveRegularFile(writer http.ResponseWriter, request *http.Request, absolutePath string) {
	info, err := os.Stat(absolutePath)
	if err != nil || !info.Mode().IsRegular() {
		http.Error(writer, "no such file", http.StatusNotFound)
		return
	}
	http.ServeFile(writer, request, absolutePath)
}

func newToken() string {
	secret := make([]byte, tokenBytes)
	_, err := rand.Read(secret)
	if err != nil {
		return ""
	}
	return hex.EncodeToString(secret)
}
