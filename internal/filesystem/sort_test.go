package filesystem

import (
	"testing"
	"time"
)

func entryAt(name string, isDir bool, size int64, modified string) Entry {
	timestamp, err := time.Parse(time.RFC3339, modified)
	if err != nil {
		panic(err)
	}
	return Entry{Name: name, IsDir: isDir, Size: size, modifiedAt: timestamp, Mode: "-rw-r--r--"}
}

func sortedNames(entries []Entry, order Order) []string {
	sortEntries(entries, order)
	names := make([]string, 0, len(entries))
	for _, entry := range entries {
		names = append(names, entry.Name)
	}
	return names
}

func assertOrder(t *testing.T, actual []string, expected []string) {
	t.Helper()
	if len(actual) != len(expected) {
		t.Fatalf("got %v, want %v", actual, expected)
	}
	for index, name := range expected {
		if actual[index] != name {
			t.Fatalf("got %v, want %v", actual, expected)
		}
	}
}

func sample() []Entry {
	return []Entry{
		entryAt("banana.txt", false, 300, "2026-01-03T10:00:00Z"),
		entryAt("Apple.txt", false, 100, "2026-01-02T10:00:00Z"),
		entryAt("cherry.txt", false, 200, "2026-01-01T10:00:00Z"),
		entryAt("zebra", true, 0, "2026-01-04T10:00:00Z"),
		entryAt("Alpha", true, 0, "2026-01-05T10:00:00Z"),
	}
}

func TestDefaultOrderListsDirectoriesFirstThenByName(t *testing.T) {
	actual := sortedNames(sample(), DefaultOrder())
	assertOrder(t, actual, []string{"Alpha", "zebra", "Apple.txt", "banana.txt", "cherry.txt"})
}

func TestDescendingReversesTheKeyButNotTheDirectoryGrouping(t *testing.T) {
	order := Order{Key: SortByName, Descending: true, DirectoriesFirst: true}
	actual := sortedNames(sample(), order)
	assertOrder(t, actual, []string{"zebra", "Alpha", "cherry.txt", "banana.txt", "Apple.txt"})
}

func TestSortingBySize(t *testing.T) {
	order := Order{Key: SortBySize, DirectoriesFirst: false}
	actual := sortedNames(sample(), order)
	assertOrder(t, actual, []string{"Alpha", "zebra", "Apple.txt", "cherry.txt", "banana.txt"})
}

func TestSortingByModifiedTime(t *testing.T) {
	order := Order{Key: SortByModified, DirectoriesFirst: false}
	actual := sortedNames(sample(), order)
	assertOrder(t, actual, []string{"cherry.txt", "Apple.txt", "banana.txt", "zebra", "Alpha"})
}

func TestTurningOffDirectoriesFirstMixesThemIn(t *testing.T) {
	order := Order{Key: SortByName, DirectoriesFirst: false}
	actual := sortedNames(sample(), order)
	assertOrder(t, actual, []string{"Alpha", "Apple.txt", "banana.txt", "cherry.txt", "zebra"})
}

func TestTiesFallBackToTheName(t *testing.T) {
	entries := []Entry{
		entryAt("second.bin", false, 10, "2026-01-01T10:00:00Z"),
		entryAt("first.bin", false, 10, "2026-01-01T10:00:00Z"),
	}
	order := Order{Key: SortBySize, Descending: true, DirectoriesFirst: false}
	assertOrder(t, sortedNames(entries, order), []string{"first.bin", "second.bin"})
}

func TestAnUnknownKeyListsByName(t *testing.T) {
	order := Order{Key: "nonsense", DirectoriesFirst: true}
	actual := sortedNames(sample(), order)
	assertOrder(t, actual, []string{"Alpha", "zebra", "Apple.txt", "banana.txt", "cherry.txt"})
}

func TestSortingByPermissions(t *testing.T) {
	entries := []Entry{
		{Name: "wide", Mode: "-rwxrwxrwx"},
		{Name: "narrow", Mode: "-r--------"},
	}
	order := Order{Key: SortByPermissions, DirectoriesFirst: false}
	assertOrder(t, sortedNames(entries, order), []string{"narrow", "wide"})
}
