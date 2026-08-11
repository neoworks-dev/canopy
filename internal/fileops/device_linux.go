package fileops

import (
	"fmt"
	"os"
	"syscall"
)

// deviceOf identifies the filesystem a path lives on. The trash spec needs it to
// decide between the home trash and a per-volume one.
func deviceOf(path string) (uint64, error) {
	info, err := os.Stat(path)
	if err != nil {
		return 0, err
	}
	systemInfo, ok := info.Sys().(*syscall.Stat_t)
	if !ok {
		return 0, fmt.Errorf("no device information for %s", path)
	}
	return uint64(systemInfo.Dev), nil
}

func onSameVolume(firstPath, secondPath string) (bool, error) {
	firstDevice, err := deviceOf(firstPath)
	if err != nil {
		return false, err
	}
	secondDevice, err := deviceOf(secondPath)
	if err != nil {
		return false, err
	}
	return firstDevice == secondDevice, nil
}
