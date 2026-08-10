package filesystem

import (
	"os"
	"path/filepath"
	"testing"
)

func TestResolvePathEmptyReturnsHome(t *testing.T) {
	home, err := os.UserHomeDir()
	if err != nil {
		t.Skip("no home directory available")
	}

	resolved, err := ResolvePath("")
	if err != nil {
		t.Fatalf("ResolvePath returned error: %v", err)
	}
	if resolved != home {
		t.Errorf("ResolvePath(\"\") = %q, want %q", resolved, home)
	}
}

func TestResolvePathExpandsTilde(t *testing.T) {
	home, err := os.UserHomeDir()
	if err != nil {
		t.Skip("no home directory available")
	}

	resolved, err := ResolvePath("~/Documents")
	if err != nil {
		t.Fatalf("ResolvePath returned error: %v", err)
	}

	expected := filepath.Join(home, "Documents")
	if resolved != expected {
		t.Errorf("ResolvePath(\"~/Documents\") = %q, want %q", resolved, expected)
	}
}

func TestResolvePathMakesRelativePathsAbsolute(t *testing.T) {
	resolved, err := ResolvePath("./somewhere")
	if err != nil {
		t.Fatalf("ResolvePath returned error: %v", err)
	}
	if !filepath.IsAbs(resolved) {
		t.Errorf("ResolvePath(\"./somewhere\") = %q, want an absolute path", resolved)
	}
}

func TestParentPathStopsAtRoot(t *testing.T) {
	if ParentPath("/") != "" {
		t.Errorf("ParentPath(\"/\") = %q, want an empty string", ParentPath("/"))
	}
	if ParentPath("/home/moritz") != "/home" {
		t.Errorf("ParentPath(\"/home/moritz\") = %q, want \"/home\"", ParentPath("/home/moritz"))
	}
}

func TestIsHidden(t *testing.T) {
	if !IsHidden(".config") {
		t.Error("IsHidden(\".config\") = false, want true")
	}
	if IsHidden("config") {
		t.Error("IsHidden(\"config\") = true, want false")
	}
}
