package index

import "testing"

func TestBuildMatchExpressionSplitsOnFilenamePunctuation(t *testing.T) {
	testCases := []struct {
		name     string
		text     string
		expected string
	}{
		{name: "single word", text: "report", expected: `"report"*`},
		{name: "two words", text: "annual report", expected: `"annual"* "report"*`},
		{name: "underscores split", text: "annual_report", expected: `"annual"* "report"*`},
		{name: "extension splits", text: "notes.txt", expected: `"notes"* "txt"*`},
		{name: "path separators split", text: "src/lib", expected: `"src"* "lib"*`},
		{name: "quotes stripped", text: `wei"rd`, expected: `"weird"*`},
		{name: "punctuation only is dropped", text: "!! ??", expected: ""},
		{name: "empty text", text: "   ", expected: ""},
	}
	for _, testCase := range testCases {
		t.Run(testCase.name, func(t *testing.T) {
			expression := buildMatchExpression(testCase.text)
			if expression != testCase.expected {
				t.Fatalf("got %q, want %q", expression, testCase.expected)
			}
		})
	}
}

func TestClampSearchLimitAppliesBounds(t *testing.T) {
	if clampSearchLimit(0) != defaultSearchLimit {
		t.Fatal("zero limit should fall back to the default")
	}
	if clampSearchLimit(-5) != defaultSearchLimit {
		t.Fatal("negative limit should fall back to the default")
	}
	if clampSearchLimit(maximumSearchLimit+1) != maximumSearchLimit {
		t.Fatal("limit above the ceiling should be clamped")
	}
	if clampSearchLimit(10) != 10 {
		t.Fatal("limit within bounds should pass through")
	}
}

func TestOwningRootPrefersTheDeepestRoot(t *testing.T) {
	roots := []RootRecord{
		{RootId: 1, Path: "/home/user"},
		{RootId: 2, Path: "/home/user/projects"},
	}
	root, found := owningRoot(roots, "/home/user/projects/app/main.go")
	if !found {
		t.Fatal("expected a matching root")
	}
	if root.RootId != 2 {
		t.Fatalf("matched root %d, want the deeper root 2", root.RootId)
	}
}

func TestOwningRootRejectsSiblingPrefixes(t *testing.T) {
	roots := []RootRecord{{RootId: 1, Path: "/home/user/docs"}}

	if _, found := owningRoot(roots, "/home/user/docs-backup/file.txt"); found {
		t.Fatal("a sibling directory sharing a name prefix must not match")
	}
}
