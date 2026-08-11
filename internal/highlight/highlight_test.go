package highlight

import (
	"strings"
	"testing"
)

func TestTokensSplitASourceFile(t *testing.T) {
	tokens := Tokens("main.go", "package main\n")

	if len(tokens) == 0 {
		t.Fatal("expected a source file to come back tokenised")
	}
	if joined(tokens) != "package main\n" {
		t.Errorf("tokens spell %q, want the source back", joined(tokens))
	}
	if !classPresent(tokens, ClassPrefix+"kn") {
		t.Errorf("expected the keyword to carry its class, got %v", tokens)
	}
}

func TestTokensPickTheLanguageFromTheFileName(t *testing.T) {
	for _, fileName := range []string{"script.py", "styles.css", "Makefile", "notes.md"} {
		if len(Tokens(fileName, "a = 1\n")) == 0 {
			t.Errorf("expected %q to be highlighted", fileName)
		}
	}
}

func TestTokensLeaveAnUnknownFileTypeAlone(t *testing.T) {
	if tokens := Tokens("archive.qqq", "some words\n"); tokens != nil {
		t.Fatalf("expected no tokens for an unknown file type, got %v", tokens)
	}
}

// Every class the tokens carry has to be one the generated stylesheet colours,
// or the run is drawn in a colour nothing chose.
func TestTokenClassesAreAllPrefixed(t *testing.T) {
	for _, token := range Tokens("script.py", "def f(x):\n\treturn \"a\" + 1  # note\n") {
		if token.Class == "" {
			continue
		}
		if !strings.HasPrefix(token.Class, ClassPrefix) {
			t.Errorf("class %q is missing the prefix", token.Class)
		}
	}
}

func joined(tokens []Token) string {
	text := strings.Builder{}
	for _, token := range tokens {
		text.WriteString(token.Text)
	}
	return text.String()
}

func classPresent(tokens []Token, class string) bool {
	for _, token := range tokens {
		if token.Class == class {
			return true
		}
	}
	return false
}
