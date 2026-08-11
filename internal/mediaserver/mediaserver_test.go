package mediaserver

import (
	"io"
	"net/http"
	"os"
	"path/filepath"
	"testing"
)

func TestServerServesAFileByPath(t *testing.T) {
	server := startForTest(t)
	path := writeFile(t, "song.mp3", "sound")

	response := get(t, server.MediaURL(path))
	defer response.Body.Close()

	if response.StatusCode != http.StatusOK {
		t.Fatalf("status = %d, want 200", response.StatusCode)
	}
	if readBody(t, response) != "sound" {
		t.Fatal("the response did not carry the file's contents")
	}
}

// http.ServeFile answers ranges on its own, which is what a media element needs
// to seek without reading the whole file first.
func TestServerAnswersARangeRequest(t *testing.T) {
	server := startForTest(t)
	path := writeFile(t, "song.mp3", "0123456789")

	request := newRequest(t, server.MediaURL(path))
	request.Header.Set("Range", "bytes=2-4")
	response := send(t, request)
	defer response.Body.Close()

	if response.StatusCode != http.StatusPartialContent {
		t.Fatalf("status = %d, want 206", response.StatusCode)
	}
	if readBody(t, response) != "234" {
		t.Fatal("the response did not carry the requested range")
	}
}

func TestServerRefusesARequestWithoutTheToken(t *testing.T) {
	server := startForTest(t)
	path := writeFile(t, "song.mp3", "sound")

	response := get(t, server.baseURL+mediaRoute+"?path="+path)
	defer response.Body.Close()

	if response.StatusCode != http.StatusForbidden {
		t.Fatalf("status = %d, want 403 without the token", response.StatusCode)
	}
}

func TestServerRefusesADirectory(t *testing.T) {
	server := startForTest(t)

	response := get(t, server.MediaURL(t.TempDir()))
	defer response.Body.Close()

	if response.StatusCode != http.StatusNotFound {
		t.Fatalf("status = %d, want 404 for a directory", response.StatusCode)
	}
}

func TestServerRefusesAFileThatIsNotThere(t *testing.T) {
	server := startForTest(t)

	response := get(t, server.MediaURL(filepath.Join(t.TempDir(), "gone.mp3")))
	defer response.Body.Close()

	if response.StatusCode != http.StatusNotFound {
		t.Fatalf("status = %d, want 404", response.StatusCode)
	}
}

func TestAServerThatNeverStartedOffersNoAddress(t *testing.T) {
	silent := Server{}
	if silent.MediaURL("/music/song.mp3") != "" {
		t.Fatal("a server that is not listening must offer no address to play from")
	}
}

func TestMediaURLEscapesThePath(t *testing.T) {
	server := Server{baseURL: "http://127.0.0.1:1", token: "secret"}

	address := server.MediaURL("/music/two words & more.mp3")

	if address != "http://127.0.0.1:1/media?path=%2Fmusic%2Ftwo+words+%26+more.mp3&token=secret" {
		t.Fatalf("address = %q", address)
	}
}

func startForTest(t *testing.T) Server {
	t.Helper()
	server := Start()
	if server.baseURL == "" {
		t.Fatal("the media server did not come up")
	}
	return server
}

func get(t *testing.T, address string) *http.Response {
	t.Helper()
	return send(t, newRequest(t, address))
}

func newRequest(t *testing.T, address string) *http.Request {
	t.Helper()
	request, err := http.NewRequest(http.MethodGet, address, nil)
	if err != nil {
		t.Fatalf("request %s: %v", address, err)
	}
	return request
}

func send(t *testing.T, request *http.Request) *http.Response {
	t.Helper()
	response, err := http.DefaultClient.Do(request)
	if err != nil {
		t.Fatalf("send: %v", err)
	}
	return response
}

func readBody(t *testing.T, response *http.Response) string {
	t.Helper()
	body, err := io.ReadAll(response.Body)
	if err != nil {
		t.Fatalf("read body: %v", err)
	}
	return string(body)
}

func writeFile(t *testing.T, name string, contents string) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), name)
	err := os.WriteFile(path, []byte(contents), 0o644)
	if err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
	return path
}
