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

func TestScopeBoundsCoverEverythingBelowADirectory(t *testing.T) {
	lower, upper := scopeBounds("/home/moritz/Downloads")

	inside := []string{"/home/moritz/Downloads/a.txt", "/home/moritz/Downloads/deep/b.txt"}
	for _, path := range inside {
		if path < lower || path >= upper {
			t.Fatalf("%q falls outside [%q, %q)", path, lower, upper)
		}
	}

	// A sibling sharing the prefix must not be swept in, and neither should the
	// directory itself.
	outside := []string{"/home/moritz/Downloads2/a.txt", "/home/moritz/Downloads", "/home/moritz/a"}
	for _, path := range outside {
		if path >= lower && path < upper {
			t.Fatalf("%q falls inside [%q, %q)", path, lower, upper)
		}
	}
}

func TestScopeBoundsCoverEveryAbsolutePathAtTheRoot(t *testing.T) {
	lower, upper := scopeBounds("/")
	for _, path := range []string{"/a", "/home/moritz/x", "/zzz"} {
		if path < lower || path >= upper {
			t.Fatalf("%q falls outside [%q, %q)", path, lower, upper)
		}
	}
}
