package filesystem

import "syscall"

// capacityOf reports the size of a mounted filesystem and how much of it is
// free to an unprivileged user. A volume that cannot be measured reports zero,
// which the sidebar shows as no capacity rather than as an empty disk.
func capacityOf(mountPoint string) (uint64, uint64) {
	statistics := syscall.Statfs_t{}
	if err := syscall.Statfs(mountPoint, &statistics); err != nil {
		return 0, 0
	}
	blockSize := uint64(statistics.Bsize)
	return statistics.Blocks * blockSize, statistics.Bavail * blockSize
}
