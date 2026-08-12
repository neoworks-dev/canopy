// Command canopyd is the shared Neoworks background daemon. Every Neoworks
// desktop application ships this binary and starts it on demand; the first one to
// launch wins the daemon lock and the rest connect to it as clients.
package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"log/slog"
	"os"
	"os/signal"
	"path/filepath"
	"syscall"
	"time"

	"canopy/daemon/internal/index"
	"canopy/daemon/internal/lockfile"
	"canopy/daemon/internal/server"
	"canopy/daemon/proto"
)

// daemonVersion is compared by clients during the handshake and decides which
// bundled copy wins when two applications ship different builds.
const daemonVersion = "0.1.0"

type configuration struct {
	socketPath     string
	databasePath   string
	idleTimeout    time.Duration
	rescanInterval time.Duration
	maximumWatches int
	logLevel       string
	printVersion   bool
}

func main() {
	if err := run(parseConfiguration()); err != nil {
		fmt.Fprintln(os.Stderr, "canopyd:", err)
		os.Exit(1)
	}
}

func run(config configuration) error {
	if config.printVersion {
		fmt.Println(daemonVersion)
		return nil
	}
	logger := buildLogger(config.logLevel)
	lock, alreadyRunning, err := claimDaemonLock()
	if err != nil {
		return err
	}
	if alreadyRunning {
		logger.Info("another daemon already holds the lock, exiting")
		return nil
	}
	defer lock.Release()

	return serve(config, logger)
}

func serve(config configuration, logger *slog.Logger) error {
	runContext, stopOnSignal := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stopOnSignal()

	indexer, err := startIndexer(runContext, config, logger)
	if err != nil {
		return err
	}
	defer indexer.Close()

	logger.Info("daemon ready",
		"version", daemonVersion,
		"socket", config.socketPath,
		"database", config.databasePath)

	return server.New(indexer, server.Options{
		SocketPath:    config.socketPath,
		IdleTimeout:   config.idleTimeout,
		DaemonVersion: daemonVersion,
		IndexPath:     config.databasePath,
		Logger:        logger,
	}).Serve(runContext)
}

func startIndexer(runContext context.Context, config configuration, logger *slog.Logger) (*index.Indexer, error) {
	if err := proto.EnsurePrivateDirectory(filepath.Dir(config.databasePath)); err != nil {
		return nil, err
	}
	indexer, err := index.New(runContext, index.Options{
		DatabasePath:   config.databasePath,
		MaximumWatches: config.maximumWatches,
		RescanInterval: config.rescanInterval,
		Logger:         logger,
	})
	if err != nil {
		return nil, err
	}
	indexer.Start()
	return indexer, nil
}

// claimDaemonLock reports alreadyRunning rather than failing, because losing the
// race is the expected outcome when two applications start at once.
func claimDaemonLock() (*lockfile.Lock, bool, error) {
	if err := proto.EnsurePrivateDirectory(proto.RuntimeDirectory()); err != nil {
		return nil, false, err
	}
	lock, err := lockfile.TryAcquire(proto.DaemonLockPath())
	if errors.Is(err, lockfile.ErrLockBusy) {
		return nil, true, nil
	}
	if err != nil {
		return nil, false, err
	}
	return lock, false, nil
}

func parseConfiguration() configuration {
	var config configuration
	flag.StringVar(&config.socketPath, "socket", proto.SocketPath(), "unix socket to listen on")
	flag.StringVar(&config.databasePath, "database", proto.IndexDatabasePath(), "index database file")
	flag.DurationVar(&config.idleTimeout, "idle-timeout", 10*time.Minute,
		"exit after this long with no connected clients; zero disables")
	flag.DurationVar(&config.rescanInterval, "rescan-interval", 30*time.Minute,
		"how often every root is fully rescanned")
	flag.IntVar(&config.maximumWatches, "max-watches", 0,
		"ceiling on inotify watches; zero uses the built-in default")
	flag.StringVar(&config.logLevel, "log-level", "info", "debug, info, warn, or error")
	flag.BoolVar(&config.printVersion, "version", false, "print the daemon version and exit")
	flag.Parse()
	return config
}

// buildLogger writes to stderr; a client that spawns the daemon detached is
// responsible for pointing that at a file.
func buildLogger(logLevel string) *slog.Logger {
	handler := slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: parseLogLevel(logLevel)})
	return slog.New(handler)
}

func parseLogLevel(logLevel string) slog.Level {
	var level slog.Level
	if err := level.UnmarshalText([]byte(logLevel)); err != nil {
		return slog.LevelInfo
	}
	return level
}
