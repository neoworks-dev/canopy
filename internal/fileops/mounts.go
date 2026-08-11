package fileops

import (
	"os"
	"strings"
)

// volumeTrashLocations finds per-volume trash directories that already exist, so
// items trashed on removable media stay listable and restorable.
func volumeTrashLocations() []trashLocation {
	locations := []trashLocation{}
	for _, mountPoint := range mountPoints() {
		locations = appendExistingVolumeTrash(locations, mountPoint)
	}
	return locations
}

func appendExistingVolumeTrash(locations []trashLocation, mountPoint string) []trashLocation {
	directory := volumeTrashDirectory(mountPoint)
	if !isDirectory(directory) {
		return locations
	}
	return append(locations, trashLocation{directory: directory, topDirectory: mountPoint})
}

func mountPoints() []string {
	contents, err := os.ReadFile("/proc/self/mounts")
	if err != nil {
		return nil
	}
	return parseMountPoints(string(contents))
}

func parseMountPoints(contents string) []string {
	points := []string{}
	for _, line := range strings.Split(contents, "\n") {
		points = appendMountPoint(points, line)
	}
	return points
}

func appendMountPoint(points []string, line string) []string {
	fields := strings.Fields(line)
	if len(fields) < 2 {
		return points
	}
	return append(points, unescapeMountPoint(fields[1]))
}

// unescapeMountPoint decodes the octal escapes the kernel writes for spaces,
// tabs, newlines and backslashes in mount points.
func unescapeMountPoint(field string) string {
	return mountEscapes.Replace(field)
}

var mountEscapes = strings.NewReplacer(`\040`, " ", `\011`, "\t", `\012`, "\n", `\134`, `\`)
