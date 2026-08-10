//go:build unix

// Package lockfile wraps advisory file locks. Two locks shape the daemon's
// lifecycle: the daemon holds one for as long as it runs, which makes a second
// daemon impossible, and a launching client holds the other only while starting
// a daemon, which makes simultaneous app launches spawn exactly one.
package lockfile

import (
	"errors"
	"os"
	"time"

	"golang.org/x/sys/unix"
)

// ErrLockBusy means another process holds the lock.
var ErrLockBusy = errors.New("lockfile: already held")

const acquirePollInterval = 20 * time.Millisecond

// Lock is held until Release. Closing the file releases the flock, so a crashed
// process never leaves a lock behind.
type Lock struct {
	file *os.File
}

// TryAcquire takes the lock without blocking, returning ErrLockBusy when another
// process holds it.
func TryAcquire(path string) (*Lock, error) {
	file, err := os.OpenFile(path, os.O_CREATE|os.O_RDWR, 0o600)
	if err != nil {
		return nil, err
	}
	if err := unix.Flock(int(file.Fd()), unix.LOCK_EX|unix.LOCK_NB); err != nil {
		file.Close()
		return nil, translateFlockError(err)
	}
	return &Lock{file: file}, nil
}

// AcquireWithin polls until the lock is free or the timeout expires. Polling
// keeps the caller cancellable without leaving a goroutine parked in flock.
func AcquireWithin(path string, timeout time.Duration) (*Lock, error) {
	deadline := time.Now().Add(timeout)
	for {
		lock, err := TryAcquire(path)
		if !errors.Is(err, ErrLockBusy) {
			return lock, err
		}
		if time.Now().After(deadline) {
			return nil, ErrLockBusy
		}
		time.Sleep(acquirePollInterval)
	}
}

func (lock *Lock) Release() error {
	return lock.file.Close()
}

func translateFlockError(err error) error {
	if errors.Is(err, unix.EWOULDBLOCK) {
		return ErrLockBusy
	}
	return err
}
