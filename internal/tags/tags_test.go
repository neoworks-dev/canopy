package tags

import (
	"errors"
	"os"
	"path/filepath"
	"syscall"
	"testing"
)

func TestTagsAreReadBackFromTheFile(t *testing.T) {
	path := taggableFile(t)

	if err := Set(path, []string{"work", "invoice"}); err != nil {
		t.Fatal(err)
	}

	if got := Of(path); len(got) != 2 || got[0] != "invoice" || got[1] != "work" {
		t.Fatalf("got %v, want the tags sorted", got)
	}
}

func TestAddKeepsTheTagsAlreadyThere(t *testing.T) {
	path := taggableFile(t)

	mustSucceed(t, Add(path, "work"))
	mustSucceed(t, Add(path, "invoice"))

	if got := Of(path); len(got) != 2 {
		t.Fatalf("got %v, want both tags", got)
	}
}

func TestAddIgnoresATagThatIsAlreadyThere(t *testing.T) {
	path := taggableFile(t)

	mustSucceed(t, Add(path, "work"))
	mustSucceed(t, Add(path, " work "))

	if got := Of(path); len(got) != 1 {
		t.Fatalf("got %v, want one tag", got)
	}
}

func TestAddRefusesATagWithNoName(t *testing.T) {
	path := taggableFile(t)

	if Add(path, "   ") == nil {
		t.Fatalf("expected a blank tag to be refused")
	}
}

func TestRemovingTheLastTagLeavesNothingBehind(t *testing.T) {
	path := taggableFile(t)
	mustSucceed(t, Add(path, "work"))

	mustSucceed(t, Remove(path, "work"))

	if got := Of(path); len(got) != 0 {
		t.Fatalf("got %v, want no tags", got)
	}
	if Has(path, "work") {
		t.Fatalf("expected the tag to be gone")
	}
}

func TestRemovingATagThatIsNotThereIsNotAFailure(t *testing.T) {
	path := taggableFile(t)

	if err := Remove(path, "nothing"); err != nil {
		t.Fatalf("got %v", err)
	}
}

func TestAnUntaggedFileHasNoTags(t *testing.T) {
	if got := Of(taggableFile(t)); len(got) != 0 {
		t.Fatalf("got %v", got)
	}
}

func TestSearchFindsTaggedFilesAndSkipsHiddenDirectories(t *testing.T) {
	root := t.TempDir()
	tagged := writeFile(t, root, "keep/notes.md")
	writeFile(t, root, "keep/other.md")
	hidden := writeFile(t, root, ".cache/notes.md")
	requireTaggable(t, tagged)
	mustSucceed(t, Add(tagged, "work"))
	mustSucceed(t, Add(hidden, "work"))

	found := Search(root, "work")

	if len(found) != 1 || found[0].Path != tagged {
		t.Fatalf("got %+v, want only the visible tagged file", found)
	}
}

func TestSearchWithoutANameFindsNothing(t *testing.T) {
	if found := Search(t.TempDir(), "  "); len(found) != 0 {
		t.Fatalf("got %+v", found)
	}
}

func TestVocabularyRemembersNamesAcrossLaunches(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())
	vocabulary := OpenVocabulary()

	vocabulary.Remember([]string{"work", "work", " personal "})

	if got := OpenVocabulary().Names(); len(got) != 2 || got[0] != "personal" || got[1] != "work" {
		t.Fatalf("got %v", got)
	}
}

func TestForgettingANameLeavesTheFilesAlone(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())
	path := taggableFile(t)
	mustSucceed(t, Add(path, "work"))
	vocabulary := OpenVocabulary()
	vocabulary.Remember([]string{"work"})

	vocabulary.Forget("work")

	if len(vocabulary.Names()) != 0 {
		t.Fatalf("expected the name to be dropped from the sidebar")
	}
	if !Has(path, "work") {
		t.Fatalf("expected the file to keep its tag")
	}
}

func taggableFile(t *testing.T) string {
	t.Helper()
	path := writeFile(t, t.TempDir(), "notes.md")
	requireTaggable(t, path)
	return path
}

// Tags live in an extended attribute, which not every filesystem carries. On
// one that does not, there is nothing here to test.
func requireTaggable(t *testing.T, path string) {
	t.Helper()
	err := syscall.Setxattr(path, attributeName, []byte("probe"), 0)
	if errors.Is(err, syscall.EOPNOTSUPP) || errors.Is(err, syscall.ENOTSUP) {
		t.Skip("this filesystem does not carry extended attributes")
	}
	if err != nil {
		t.Fatal(err)
	}
	if err := syscall.Removexattr(path, attributeName); err != nil {
		t.Fatal(err)
	}
}

func writeFile(t *testing.T, root string, name string) string {
	t.Helper()
	path := filepath.Join(root, name)
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte("contents"), 0o600); err != nil {
		t.Fatal(err)
	}
	return path
}

func mustSucceed(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatal(err)
	}
}
