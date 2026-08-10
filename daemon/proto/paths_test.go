//go:build unix

package proto

import (
	"path/filepath"
	"strings"
	"testing"
)

func TestValidateSocketPathAcceptsAPathTheKernelCanBind(t *testing.T) {
	if err := ValidateSocketPath("/run/user/1000/neoworks/daemon.sock"); err != nil {
		t.Fatalf("expected a normal runtime path to be accepted: %v", err)
	}
}

// A path over the sun_path limit fails at bind time with a bare "invalid
// argument", so it has to be rejected with an explanation instead.
func TestValidateSocketPathRejectsAnOverlongPath(t *testing.T) {
	overlongPath := filepath.Join("/tmp", strings.Repeat("d", maxSocketPathLength), "daemon.sock")

	err := ValidateSocketPath(overlongPath)
	if err == nil {
		t.Fatal("expected an overlong socket path to be rejected")
	}
	if !strings.Contains(err.Error(), RuntimeDirectoryEnvironmentVariable) {
		t.Fatalf("error should name the override that fixes it, got: %v", err)
	}
}

func TestRuntimeDirectoryHonoursTheOverride(t *testing.T) {
	t.Setenv(RuntimeDirectoryEnvironmentVariable, "/tmp/neoworks-test-runtime")

	if RuntimeDirectory() != "/tmp/neoworks-test-runtime" {
		t.Fatalf("override ignored, got %s", RuntimeDirectory())
	}
	if filepath.Dir(SocketPath()) != "/tmp/neoworks-test-runtime" {
		t.Fatalf("socket did not follow the override, got %s", SocketPath())
	}
}

func TestDataDirectoryHonoursTheOverride(t *testing.T) {
	t.Setenv(DataDirectoryEnvironmentVariable, "/tmp/neoworks-test-data")

	if filepath.Dir(IndexDatabasePath()) != "/tmp/neoworks-test-data" {
		t.Fatalf("index database did not follow the override, got %s", IndexDatabasePath())
	}
}
