package zoxide

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestParseListReadsScoresAndPaths(t *testing.T) {
	entries := parseList(" 112.0 /home/moritz/Documents\n  54.0 /home/moritz/git-worktree-editor\n")

	if len(entries) != 2 {
		t.Fatalf("parsed %d entries, want 2", len(entries))
	}
	if entries[0].Score != 112 || entries[0].Path != "/home/moritz/Documents" {
		t.Fatalf("first entry %+v", entries[0])
	}
}

// A query stops once it has filled its limit, so the dump has to be ranked
// before anything reads it.
func TestParseListRanksByScore(t *testing.T) {
	entries := parseList("  10.0 /home/moritz/notes\n  90.0 /home/moritz/projects\n  50.0 /home/moritz/archive\n")

	scores := []float64{}
	for _, entry := range entries {
		scores = append(scores, entry.Score)
	}
	if len(scores) != 3 || scores[0] != 90 || scores[1] != 50 || scores[2] != 10 {
		t.Fatalf("scores %v, want them descending", scores)
	}
}

func TestParseListKeepsPathsContainingSpaces(t *testing.T) {
	entries := parseList("  10.5 /home/moritz/My Documents/old stuff\n")

	if len(entries) != 1 || entries[0].Path != "/home/moritz/My Documents/old stuff" {
		t.Fatalf("entries %+v, want the path intact", entries)
	}
}

// A line zoxide did not write must not cost the rest of the dump.
func TestParseListSkipsUnreadableLines(t *testing.T) {
	entries := parseList("not a score line\n\n 12.0 /home/moritz\nalso bad\n")

	if len(entries) != 1 || entries[0].Path != "/home/moritz" {
		t.Fatalf("entries %+v, want only the readable line", entries)
	}
}

func TestMatchesKeywordsFollowsZoxideRules(t *testing.T) {
	const path = "/home/moritz/Documents/file-explorer-neoworks"

	cases := []struct {
		text     string
		expected bool
	}{
		{"", true},
		{"neo", true},
		{"NEO", true},
		{"doc neo", true},
		{"neo doc", false},
		{"moritz explorer", true},
		// The last keyword has to land in the final component, which is what makes
		// a query select a directory rather than its parent.
		{"documents", false},
		{"nothing", false},
	}

	for _, testCase := range cases {
		if got := matchesKeywords(path, splitKeywords(testCase.text)); got != testCase.expected {
			t.Fatalf("%q matched %v, want %v", testCase.text, got, testCase.expected)
		}
	}
}

// Two keywords must not both match the same stretch of the path.
func TestMatchesKeywordsConsumesEachKeywordInTurn(t *testing.T) {
	if matchesKeywords("/home/moritz/reports", splitKeywords("reports reports")) {
		t.Fatal("one occurrence satisfied two keywords")
	}
	if !matchesKeywords("/home/reports/reports", splitKeywords("reports reports")) {
		t.Fatal("two occurrences did not satisfy two keywords")
	}
}

func TestQueryRanksMatchesAndHonoursTheLimit(t *testing.T) {
	root := t.TempDir()
	database := databaseOver(t, []Entry{
		{Path: makeDirectory(t, root, "notes"), Score: 10},
		{Path: makeDirectory(t, root, "projects"), Score: 90},
		{Path: makeDirectory(t, root, "project-archive"), Score: 50},
	})

	found := database.Query("project", "", 10)
	if len(found) != 2 {
		t.Fatalf("found %d, want the two project directories", len(found))
	}
	if found[0].Score != 90 {
		t.Fatalf("first hit scored %v, want the most frecent first", found[0].Score)
	}
	if len(database.Query("project", "", 1)) != 1 {
		t.Fatal("limit not applied")
	}
}

func TestQueryLeavesOutTheExcludedDirectory(t *testing.T) {
	root := t.TempDir()
	here := makeDirectory(t, root, "projects")
	database := databaseOver(t, []Entry{{Path: here, Score: 90}})

	if len(database.Query("project", here, 10)) != 0 {
		t.Fatal("the excluded directory came back as a candidate")
	}
}

// A directory zoxide remembers may since have been deleted.
func TestQuerySkipsDirectoriesThatAreGone(t *testing.T) {
	database := databaseOver(t, []Entry{{Path: "/definitely/not/here/projects", Score: 90}})

	if len(database.Query("project", "", 10)) != 0 {
		t.Fatal("a missing directory came back as a candidate")
	}
}

func TestAnEmptyQueryReturnsTheTopOfTheList(t *testing.T) {
	root := t.TempDir()
	database := databaseOver(t, []Entry{
		{Path: makeDirectory(t, root, "notes"), Score: 10},
		{Path: makeDirectory(t, root, "projects"), Score: 90},
	})

	if len(database.Query("", "", 10)) != 2 {
		t.Fatal("an empty query should offer everything")
	}
}

// Without zoxide installed the explorer has to carry on and say nothing.
func TestAMissingBinaryAnswersEmpty(t *testing.T) {
	t.Setenv("PATH", t.TempDir())
	database := New()

	if len(database.Query("anything", "", 10)) != 0 {
		t.Fatal("a query answered without zoxide installed")
	}
	if database.Available() {
		t.Fatal("reported available without zoxide installed")
	}
}

// databaseOver seeds the cache so a test never depends on the machine's own
// zoxide database.
func databaseOver(t *testing.T, entries []Entry) *Database {
	t.Helper()
	return &Database{entries: entries, readAt: time.Now(), installed: true, checked: true}
}

func makeDirectory(t *testing.T, root string, name string) string {
	t.Helper()
	path := filepath.Join(root, name)
	if err := os.MkdirAll(path, 0o700); err != nil {
		t.Fatalf("make %s: %v", path, err)
	}
	return path
}
