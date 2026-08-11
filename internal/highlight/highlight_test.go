package highlight

import (
	"strings"
	"testing"
)

func TestHTMLMarksUpASourceFile(t *testing.T) {
	marked := HTML("main.go", "package main\n")

	if !strings.Contains(marked, `<span class="`+ClassPrefix) {
		t.Fatalf("expected prefixed spans, got %q", marked)
	}
	if !strings.Contains(marked, "package") {
		t.Errorf("expected the source text to survive, got %q", marked)
	}
}

func TestHTMLPicksTheLanguageFromTheFileName(t *testing.T) {
	for _, fileName := range []string{"script.py", "styles.css", "Makefile", "notes.md"} {
		if HTML(fileName, "a = 1\n") == "" {
			t.Errorf("expected %q to be highlighted", fileName)
		}
	}
}

func TestHTMLLeavesAnUnknownFileTypeAlone(t *testing.T) {
	if marked := HTML("archive.qqq", "some words\n"); marked != "" {
		t.Fatalf("expected no markup for an unknown file type, got %q", marked)
	}
}

// The markup is inserted as HTML, so a file that contains markup of its own has
// to come back as text rather than as tags.
func TestHTMLEscapesTheSourceText(t *testing.T) {
	marked := HTML("page.py", `x = "<img src=x onerror=alert(1)>"`)

	if strings.Contains(marked, "<img") {
		t.Fatalf("expected the tag to be escaped, got %q", marked)
	}
	if !strings.Contains(marked, "&lt;img") {
		t.Errorf("expected the text to survive escaped, got %q", marked)
	}
}
