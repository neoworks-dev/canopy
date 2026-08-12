//go:build unix

package client

import (
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"syscall"
	"time"

	"canopy/daemon/internal/lockfile"
	"canopy/daemon/proto"
)

// BinaryPathEnvironmentVariable points at the daemon binary. Development builds
// and test harnesses use it; a shipped application passes Options.BinaryPath.
const BinaryPathEnvironmentVariable = "CANOPYD_BIN"

const daemonBinaryName = "canopyd"

const dialPollInterval = 25 * time.Millisecond

// dialOrSpawn connects to a running daemon, starting one if there is none.
func dialOrSpawn(options Options) (net.Conn, error) {
	if err := proto.ValidateSocketPath(options.SocketPath); err != nil {
		return nil, err
	}
	connection, err := dial(options.SocketPath)
	if err == nil {
		return connection, nil
	}
	return spawnAndDial(options)
}

// spawnAndDial serializes startup on the spawn lock, so several applications
// launching at once produce one daemon rather than a race.
func spawnAndDial(options Options) (net.Conn, error) {
	if err := proto.EnsurePrivateDirectory(filepath.Dir(options.SocketPath)); err != nil {
		return nil, err
	}
	lock, err := lockfile.AcquireWithin(proto.SpawnLockPath(), options.SpawnTimeout)
	if err != nil {
		return nil, fmt.Errorf("waiting for daemon startup: %w", err)
	}
	defer lock.Release()

	// Whoever held the lock before us may already have started the daemon.
	connection, err := dial(options.SocketPath)
	if err == nil {
		return connection, nil
	}
	if err := startDaemon(options); err != nil {
		return nil, err
	}
	return dialUntilReady(options)
}

func dial(socketPath string) (net.Conn, error) {
	return net.DialTimeout("unix", socketPath, time.Second)
}

func dialUntilReady(options Options) (net.Conn, error) {
	deadline := time.Now().Add(options.SpawnTimeout)
	for {
		connection, err := dial(options.SocketPath)
		if err == nil {
			return connection, nil
		}
		if time.Now().After(deadline) {
			return nil, fmt.Errorf("daemon did not accept connections within %s: %w", options.SpawnTimeout, err)
		}
		time.Sleep(dialPollInterval)
	}
}

// startDaemon launches the daemon in its own session so it survives the exit of
// the application that started it and never receives the terminal's signals.
func startDaemon(options Options) error {
	binaryPath, err := resolveDaemonBinary(options.BinaryPath)
	if err != nil {
		return err
	}
	logFile, err := openDaemonLog()
	if err != nil {
		return err
	}
	defer logFile.Close()

	command := exec.Command(binaryPath, "-socket", options.SocketPath)
	command.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
	command.Stdout = logFile
	command.Stderr = logFile
	if err := command.Start(); err != nil {
		return fmt.Errorf("start %s: %w", binaryPath, err)
	}
	// Reap the child if it exits while this application is still running. Once we
	// exit, init adopts the daemon.
	go command.Wait()
	return nil
}

// resolveDaemonBinary prefers an explicit path, then the environment override,
// then a binary shipped next to the running application, and finally PATH.
func resolveDaemonBinary(configuredPath string) (string, error) {
	candidates := daemonBinaryCandidates(configuredPath)
	for _, candidate := range candidates {
		if isExecutableFile(candidate) {
			return candidate, nil
		}
	}
	return exec.LookPath(daemonBinaryName)
}

func daemonBinaryCandidates(configuredPath string) []string {
	candidates := make([]string, 0, 3)
	candidates = appendIfNotEmpty(candidates, configuredPath)
	candidates = appendIfNotEmpty(candidates, os.Getenv(BinaryPathEnvironmentVariable))
	return appendIfNotEmpty(candidates, siblingOfExecutable())
}

func siblingOfExecutable() string {
	executablePath, err := os.Executable()
	if err != nil {
		return ""
	}
	return filepath.Join(filepath.Dir(executablePath), daemonBinaryName)
}

func isExecutableFile(path string) bool {
	info, err := os.Stat(path)
	if err != nil || info.IsDir() {
		return false
	}
	return info.Mode().Perm()&0o111 != 0
}

func appendIfNotEmpty(paths []string, candidate string) []string {
	if candidate == "" {
		return paths
	}
	return append(paths, candidate)
}

func openDaemonLog() (*os.File, error) {
	if err := proto.EnsurePrivateDirectory(proto.DataDirectory()); err != nil {
		return nil, err
	}
	return os.OpenFile(proto.LogPath(), os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o600)
}
