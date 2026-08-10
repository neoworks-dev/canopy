//go:build unix

package proto

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
)

// maxSocketPathLength is the portable floor for the kernel's sun_path field
// (macOS allows 104 bytes, Linux 108). Exceeding it fails with a bare
// "invalid argument", so the limit is checked explicitly instead.
const maxSocketPathLength = 103

const applicationDirectoryName = "neoworks"

// RuntimeDirectoryEnvironmentVariable overrides every runtime path at once,
// which is what test harnesses and sandboxed runs use to stay isolated.
const RuntimeDirectoryEnvironmentVariable = "NEOWORKS_RUNTIME_DIR"

// DataDirectoryEnvironmentVariable overrides where the index database lives.
const DataDirectoryEnvironmentVariable = "NEOWORKS_DATA_DIR"

// RuntimeDirectory holds the socket and lock files. It is deliberately on a
// tmpfs so a reboot cannot leave a stale socket behind.
func RuntimeDirectory() string {
	override := os.Getenv(RuntimeDirectoryEnvironmentVariable)
	if override != "" {
		return override
	}
	runtimeHome := os.Getenv("XDG_RUNTIME_DIR")
	if runtimeHome == "" {
		return filepath.Join(os.TempDir(), applicationDirectoryName+"-"+strconv.Itoa(os.Getuid()))
	}
	return filepath.Join(runtimeHome, applicationDirectoryName)
}

func SocketPath() string {
	return filepath.Join(RuntimeDirectory(), "daemon.sock")
}

// ValidateSocketPath reports a path the kernel cannot bind. Both sides check it,
// because a client that cannot dial would otherwise spawn a daemon that cannot
// listen.
func ValidateSocketPath(socketPath string) error {
	if len(socketPath) <= maxSocketPathLength {
		return nil
	}
	return fmt.Errorf(
		"socket path %q is %d bytes, over the %d byte kernel limit; point %s at a shorter directory",
		socketPath, len(socketPath), maxSocketPathLength, RuntimeDirectoryEnvironmentVariable)
}

// DaemonLockPath is held by the daemon for its entire lifetime, which is what
// makes a second daemon impossible.
func DaemonLockPath() string {
	return filepath.Join(RuntimeDirectory(), "daemon.lock")
}

// SpawnLockPath is held by a client only while it starts a daemon, so
// simultaneous launches of several apps spawn exactly one.
func SpawnLockPath() string {
	return filepath.Join(RuntimeDirectory(), "spawn.lock")
}

// DataDirectory holds durable state: the index database and the daemon log.
func DataDirectory() string {
	override := os.Getenv(DataDirectoryEnvironmentVariable)
	if override != "" {
		return override
	}
	dataHome := os.Getenv("XDG_DATA_HOME")
	if dataHome == "" {
		dataHome = defaultDataHome()
	}
	return filepath.Join(dataHome, applicationDirectoryName)
}

func IndexDatabasePath() string {
	return filepath.Join(DataDirectory(), "index.db")
}

func LogPath() string {
	return filepath.Join(DataDirectory(), "daemon.log")
}

// EnsurePrivateDirectory creates a directory only the owning user can enter.
// Existing directories are tightened too, because a permissive mode on the
// runtime directory would expose the socket.
func EnsurePrivateDirectory(path string) error {
	if err := os.MkdirAll(path, 0o700); err != nil {
		return err
	}
	return os.Chmod(path, 0o700)
}

func defaultDataHome() string {
	homeDirectory, err := os.UserHomeDir()
	if err != nil {
		return filepath.Join(os.TempDir(), "neoworks-data")
	}
	return filepath.Join(homeDirectory, ".local", "share")
}
