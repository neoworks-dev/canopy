package searches

import (
	"os"
	"path/filepath"
	"testing"
)

func TestSavedSearchesSurviveReopening(t *testing.T) {
	isolateConfig(t)
	store := Open()

	saveOne(t, store, Search{Name: "Invoices", Query: "invoice", Scope: "/home/user/docs"})

	reopened := Open().List()
	if len(reopened) != 1 {
		t.Fatalf("saved = %v, want the one that was written", reopened)
	}
	if reopened[0].Query != "invoice" || reopened[0].Scope != "/home/user/docs" {
		t.Fatalf("saved = %+v, want the query and the scope it was saved with", reopened[0])
	}
}

// A saved search keeps the query rather than its results, because the index
// moves on and running it again is the point.
func TestSavingUnderANameInUseReplacesIt(t *testing.T) {
	isolateConfig(t)
	store := Open()

	saveOne(t, store, Search{Name: "Invoices", Query: "invoice"})
	saveOne(t, store, Search{Name: "Invoices", Query: "receipt"})

	saved := store.List()
	if len(saved) != 1 {
		t.Fatalf("saved = %v, want one search under the name", saved)
	}
	if saved[0].Query != "receipt" {
		t.Fatalf("query = %q, want the one saved last", saved[0].Query)
	}
}

func TestAnEmptyQueryIsNotASearch(t *testing.T) {
	isolateConfig(t)
	store := Open()

	saveOne(t, store, Search{Name: "Nothing", Query: "  "})

	if len(store.List()) != 0 {
		t.Fatal("a search with no query must not be saved")
	}
}

func TestAnUnnamedSearchIsNotSaved(t *testing.T) {
	isolateConfig(t)
	store := Open()

	saveOne(t, store, Search{Name: "", Query: "invoice"})

	if len(store.List()) != 0 {
		t.Fatal("a search with no name has nothing to be listed under")
	}
}

func TestNamesAndQueriesAreTrimmed(t *testing.T) {
	isolateConfig(t)
	store := Open()

	saveOne(t, store, Search{Name: "  Invoices  ", Query: "  invoice  "})

	saved := store.List()
	if saved[0].Name != "Invoices" || saved[0].Query != "invoice" {
		t.Fatalf("saved = %+v, want both trimmed", saved[0])
	}
}

func TestForgettingASearchLeavesTheRest(t *testing.T) {
	isolateConfig(t)
	store := Open()
	saveOne(t, store, Search{Name: "Invoices", Query: "invoice"})
	saveOne(t, store, Search{Name: "Photos", Query: "jpg"})

	err := store.Forget("Invoices")
	if err != nil {
		t.Fatalf("Forget: %v", err)
	}

	saved := Open().List()
	if len(saved) != 1 || saved[0].Name != "Photos" {
		t.Fatalf("saved = %v, want only the one that was kept", saved)
	}
}

func TestForgettingSomethingUnsavedIsNotAFailure(t *testing.T) {
	isolateConfig(t)
	store := Open()

	if store.Forget("Never saved") != nil {
		t.Fatal("forgetting a search that was never saved must not fail")
	}
}

func TestRenamingKeepsTheQueryAndTheScope(t *testing.T) {
	isolateConfig(t)
	store := Open()
	saveOne(t, store, Search{Name: "Invoices", Query: "invoice", Scope: "/home/user/docs"})

	err := store.Rename("Invoices", "Bills")
	if err != nil {
		t.Fatalf("Rename: %v", err)
	}

	saved := Open().List()
	if len(saved) != 1 {
		t.Fatalf("saved = %v, want the renamed search alone", saved)
	}
	if saved[0].Name != "Bills" || saved[0].Query != "invoice" || saved[0].Scope != "/home/user/docs" {
		t.Fatalf("saved = %+v, want the same search under the new name", saved[0])
	}
}

func TestRenamingSomethingUnsavedChangesNothing(t *testing.T) {
	isolateConfig(t)
	store := Open()
	saveOne(t, store, Search{Name: "Photos", Query: "jpg"})

	err := store.Rename("Invoices", "Bills")
	if err != nil {
		t.Fatalf("Rename: %v", err)
	}

	if len(store.List()) != 1 {
		t.Fatal("renaming a search that was never saved must not add one")
	}
}

// The file is written by hand as easily as by the explorer, so a broken one
// must not stop the sidebar from loading.
func TestAnUnreadableFileReadsAsNothingSaved(t *testing.T) {
	directory := isolateConfig(t)
	writeSearchFile(t, directory, "not json")

	if len(Open().List()) != 0 {
		t.Fatal("a file that is not a list of searches must read as nothing saved")
	}
}

func TestAnEntryWithoutAQueryIsDroppedOnReading(t *testing.T) {
	directory := isolateConfig(t)
	writeSearchFile(t, directory, `[{"name":"Broken"},{"name":"Photos","query":"jpg"}]`)

	saved := Open().List()
	if len(saved) != 1 || saved[0].Name != "Photos" {
		t.Fatalf("saved = %v, want only the usable one", saved)
	}
}

func TestListDoesNotHandOutTheStoresOwnSlice(t *testing.T) {
	isolateConfig(t)
	store := Open()
	saveOne(t, store, Search{Name: "Photos", Query: "jpg"})

	store.List()[0].Name = "Changed"

	if store.List()[0].Name != "Photos" {
		t.Fatal("a caller must not be able to rewrite the store through the list it was given")
	}
}

func isolateConfig(t *testing.T) string {
	t.Helper()
	directory := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", directory)
	return directory
}

func saveOne(t *testing.T, store *Store, search Search) {
	t.Helper()
	err := store.Save(search)
	if err != nil {
		t.Fatalf("Save: %v", err)
	}
}

func writeSearchFile(t *testing.T, configHome string, contents string) {
	t.Helper()
	directory := filepath.Join(configHome, "neoworks-files")
	err := os.MkdirAll(directory, 0o700)
	if err != nil {
		t.Fatalf("mkdir: %v", err)
	}
	err = os.WriteFile(filepath.Join(directory, fileName), []byte(contents), 0o600)
	if err != nil {
		t.Fatalf("write: %v", err)
	}
}
