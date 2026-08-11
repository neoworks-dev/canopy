package recents

import (
	"strconv"
	"testing"
)

func TestRecordPutsTheNewestFirst(t *testing.T) {
	store := openInTemporaryConfig(t)

	store.Record("/home/moritz/Projects", true)
	store.Record("/home/moritz/Notes", true)

	listed := store.List()
	if listed[0].Path != "/home/moritz/Notes" || listed[1].Path != "/home/moritz/Projects" {
		t.Fatalf("got %+v", listed)
	}
}

func TestRecordPromotesRatherThanRepeats(t *testing.T) {
	store := openInTemporaryConfig(t)

	store.Record("/a", true)
	store.Record("/b", true)
	store.Record("/a", true)

	listed := store.List()
	if len(listed) != 2 || listed[0].Path != "/a" {
		t.Fatalf("got %+v, want /a promoted to the front", listed)
	}
}

// A watched directory is re-read whenever anything in it changes, which is not
// a visit worth rewriting the file for.
func TestRecordIgnoresWhatIsAlreadyInFront(t *testing.T) {
	store := openInTemporaryConfig(t)

	store.Record("/a", true)
	first := store.List()[0].OpenedAt
	store.Record("/a", true)

	if store.List()[0].OpenedAt != first {
		t.Fatalf("re-reading the same folder should not count as a new visit")
	}
}

func TestRecordKeepsTheListBounded(t *testing.T) {
	store := openInTemporaryConfig(t)

	for index := range limit + 10 {
		store.Record("/folder/"+strconv.Itoa(index), true)
	}

	if len(store.List()) != limit {
		t.Fatalf("got %d entries, want %d", len(store.List()), limit)
	}
}

func TestRecordedEntriesSurviveAReopen(t *testing.T) {
	store := openInTemporaryConfig(t)
	store.Record("/home/moritz/Notes", true)
	store.Record("/home/moritz/report.pdf", false)

	reopened := Open()

	listed := reopened.List()
	if len(listed) != 2 || listed[0].Path != "/home/moritz/report.pdf" || listed[0].IsDir {
		t.Fatalf("got %+v", listed)
	}
}

func TestClearEmptiesTheList(t *testing.T) {
	store := openInTemporaryConfig(t)
	store.Record("/a", true)

	store.Clear()

	if len(store.List()) != 0 || len(Open().List()) != 0 {
		t.Fatalf("expected the list to be empty on disk as well")
	}
}

func openInTemporaryConfig(t *testing.T) *Store {
	t.Helper()
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())
	return Open()
}

// A nil slice crosses the Wails bridge as null, which the sidebar cannot walk.
func TestAnEmptyListIsAListRatherThanNothing(t *testing.T) {
	if openInTemporaryConfig(t).List() == nil {
		t.Fatalf("expected an empty list rather than nothing")
	}
}
