package mediaserver

import (
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
)

func TestHandlerServesAFileByPath(t *testing.T) {
	path := writeFile(t, "song.mp3", "sound")

	response := get(t, MediaRoute+"?path="+path)

	if response.Code != http.StatusOK {
		t.Fatalf("status = %d, want 200", response.Code)
	}
	if response.Body.String() != "sound" {
		t.Fatalf("body = %q, want the file's contents", response.Body.String())
	}
}

// http.ServeFile answers ranges on its own, which is what a media element needs
// to seek without reading the whole file first.
func TestHandlerAnswersARangeRequest(t *testing.T) {
	path := writeFile(t, "song.mp3", "0123456789")

	request := httptest.NewRequest(http.MethodGet, MediaRoute+"?path="+path, nil)
	request.Header.Set("Range", "bytes=2-4")
	response := httptest.NewRecorder()
	Handler().ServeHTTP(response, request)

	if response.Code != http.StatusPartialContent {
		t.Fatalf("status = %d, want 206", response.Code)
	}
	if response.Body.String() != "234" {
		t.Fatalf("body = %q, want the requested range", response.Body.String())
	}
}

func TestHandlerRefusesADirectory(t *testing.T) {
	response := get(t, MediaRoute+"?path="+t.TempDir())

	if response.Code != http.StatusNotFound {
		t.Fatalf("status = %d, want 404 for a directory", response.Code)
	}
}

func TestHandlerRefusesAFileThatIsNotThere(t *testing.T) {
	response := get(t, MediaRoute+"?path="+filepath.Join(t.TempDir(), "gone.mp3"))

	if response.Code != http.StatusNotFound {
		t.Fatalf("status = %d, want 404", response.Code)
	}
}

func get(t *testing.T, target string) *httptest.ResponseRecorder {
	t.Helper()
	response := httptest.NewRecorder()
	Handler().ServeHTTP(response, httptest.NewRequest(http.MethodGet, target, nil))
	return response
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
