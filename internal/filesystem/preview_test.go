package filesystem

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"canopy/internal/highlight"
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

func TestReadPreviewHighlightsSource(t *testing.T) {
	path := writeTemporaryFile(t, "main.go", []byte("package main\n"))

	preview, err := ReadPreview(path)
	if err != nil {
		t.Fatalf("ReadPreview: %v", err)
	}

	if len(preview.Tokens) == 0 {
		t.Fatal("expected a source file to come back highlighted")
	}
	if preview.Text != "package main\n" {
		t.Errorf("text = %q, want the plain contents alongside the markup", preview.Text)
	}
}

// A file past the read limit is highlighted as far as it was read, so the
// tokens and the text the pane holds describe the same thing.
func TestReadPreviewHighlightsAsMuchOfALargeFileAsItReads(t *testing.T) {
	source := []byte("package main\n" + strings.Repeat("// a comment\n", textPreviewLimit/13))
	path := writeTemporaryFile(t, "big.go", source)

	preview, err := ReadPreview(path)
	if err != nil {
		t.Fatalf("ReadPreview: %v", err)
	}

	if !preview.Truncated {
		t.Fatalf("the sample is %d bytes, which is within the read limit", len(source))
	}
	if tokenText(preview.Tokens) != preview.Text {
		t.Error("expected the tokens to spell exactly the text that was read")
	}
}

func tokenText(tokens []highlight.Token) string {
	text := strings.Builder{}
	for _, token := range tokens {
		text.WriteString(token.Text)
	}
	return text.String()
}

func TestReadPreviewLeavesPlainTextUnhighlighted(t *testing.T) {
	path := writeTemporaryFile(t, "notes.unknownext", []byte("just some words\n"))

	preview, err := ReadPreview(path)
	if err != nil {
		t.Fatalf("ReadPreview: %v", err)
	}

	if preview.Tokens != nil {
		t.Errorf("tokens = %v, want none for a file no lexer covers", preview.Tokens)
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

// A PDF holds nothing worth showing as bytes, so it is drawn rather than read.
// One that no renderer can make sense of still reports its metadata.
func TestReadPreviewDrawsRatherThanReadsAPDF(t *testing.T) {
	path := writeTemporaryFile(t, "broken.pdf", []byte("%PDF-1.7 and then nothing usable"))

	preview, err := ReadPreview(path)
	if err != nil {
		t.Fatalf("ReadPreview: %v", err)
	}

	if preview.Kind != "unsupported" {
		t.Fatalf("kind = %q, want unsupported", preview.Kind)
	}
	if preview.Text != "" {
		t.Errorf("text = %q, want a PDF not to be read as text", preview.Text)
	}
}
