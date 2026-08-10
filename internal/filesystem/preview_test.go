package filesystem

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestReadPreviewReadsText(t *testing.T) {
	path := writeTemporaryFile(t, "notes.md", []byte("# Title\n\nBody text.\n"))

	preview, err := ReadPreview(path)
	if err != nil {
		t.Fatalf("ReadPreview: %v", err)
	}

	if preview.Kind != "text" {
		t.Fatalf("kind = %q, want text", preview.Kind)
	}
	if preview.Text != "# Title\n\nBody text.\n" {
		t.Errorf("text = %q, want the file contents", preview.Text)
	}
	if preview.Truncated {
		t.Error("small file reported as truncated")
	}
}

func TestReadPreviewTruncatesLongText(t *testing.T) {
	path := writeTemporaryFile(t, "big.log", []byte(strings.Repeat("a", textPreviewLimit+2048)))

	preview, err := ReadPreview(path)
	if err != nil {
		t.Fatalf("ReadPreview: %v", err)
	}

	if preview.Kind != "text" {
		t.Fatalf("kind = %q, want text", preview.Kind)
	}
	if len(preview.Text) != textPreviewLimit {
		t.Errorf("text length = %d, want %d", len(preview.Text), textPreviewLimit)
	}
	if !preview.Truncated {
		t.Error("long file not reported as truncated")
	}
}

func TestReadPreviewKeepsTruncatedMultibyteTextReadable(t *testing.T) {
	// The limit lands inside the final rune, which must not make the file look
	// binary.
	contents := []byte(strings.Repeat("ä", textPreviewLimit))
	path := writeTemporaryFile(t, "umlauts.txt", contents)

	preview, err := ReadPreview(path)
	if err != nil {
		t.Fatalf("ReadPreview: %v", err)
	}

	if preview.Kind != "text" {
		t.Fatalf("kind = %q, want text", preview.Kind)
	}
	if strings.HasSuffix(preview.Text, "�") {
		t.Error("text ends in a broken rune")
	}
}

func TestReadPreviewRejectsBinaryContent(t *testing.T) {
	path := writeTemporaryFile(t, "program.bin", []byte{0x7f, 'E', 'L', 'F', 0x00, 0x01})

	preview, err := ReadPreview(path)
	if err != nil {
		t.Fatalf("ReadPreview: %v", err)
	}

	if preview.Kind != "unsupported" {
		t.Fatalf("kind = %q, want unsupported", preview.Kind)
	}
	if preview.Text != "" {
		t.Error("binary file returned text")
	}
}

func TestReadPreviewEncodesImages(t *testing.T) {
	path := writeTemporaryFile(t, "pixel.png", []byte{0x89, 'P', 'N', 'G'})

	preview, err := ReadPreview(path)
	if err != nil {
		t.Fatalf("ReadPreview: %v", err)
	}

	if preview.Kind != "image" {
		t.Fatalf("kind = %q, want image", preview.Kind)
	}
	if !strings.HasPrefix(preview.DataURL, "data:image/png;base64,") {
		t.Errorf("dataUrl = %q, want a png data URL", preview.DataURL)
	}
}

func TestReadPreviewReportsDirectories(t *testing.T) {
	preview, err := ReadPreview(t.TempDir())
	if err != nil {
		t.Fatalf("ReadPreview: %v", err)
	}

	if preview.Kind != "directory" {
		t.Fatalf("kind = %q, want directory", preview.Kind)
	}
}

func TestReadPreviewFailsOnMissingPath(t *testing.T) {
	_, err := ReadPreview(filepath.Join(t.TempDir(), "absent.txt"))
	if err == nil {
		t.Fatal("expected an error for a missing file")
	}
}

func writeTemporaryFile(t *testing.T, name string, contents []byte) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), name)
	if err := os.WriteFile(path, contents, 0o644); err != nil {
		t.Fatalf("writing %s: %v", name, err)
	}
	return path
}
