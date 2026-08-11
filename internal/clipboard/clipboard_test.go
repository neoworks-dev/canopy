package clipboard

import "testing"

func TestToURIsEscapesPaths(t *testing.T) {
	uris := toURIs([]string{"/home/user/a b.txt"})
	if len(uris) != 1 || uris[0] != "file:///home/user/a%20b.txt" {
		t.Fatalf("toURIs = %q, want an escaped file URI", uris)
	}
}

func TestToPathsSkipsCommentsAndRemoteURIs(t *testing.T) {
	paths := toPaths("# comment\r\nfile:///home/user/a%20b.txt\r\nhttps://example.org/remote.txt\r\n\r\n")
	if len(paths) != 1 {
		t.Fatalf("toPaths = %q, want only the local file", paths)
	}
	if paths[0] != "/home/user/a b.txt" {
		t.Fatalf("path = %q, want the unescaped local path", paths[0])
	}
}
