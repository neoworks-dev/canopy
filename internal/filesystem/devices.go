package filesystem

import (
	"os"
	"path/filepath"
	"strings"
)

// Device is one mounted volume the sidebar can navigate to. JSON tags drive the
// TypeScript bindings Wails generates for the frontend.
type Device struct {
	Name        string `json:"name"`
	Path        string `json:"path"`
	IsRemovable bool   `json:"isRemovable"`
	TotalBytes  uint64 `json:"totalBytes"`
	FreeBytes   uint64 `json:"freeBytes"`
}

// mount is one line of the kernel's mount table.
type mount struct {
	source     string
	mountPoint string
	filesystem string
}

// Filesystems that hold files somebody would browse. Everything else on the
// mount table is a kernel interface, a container overlay or a duplicate view of
// a volume already listed.
var browsableFilesystems = map[string]bool{
	"ext2":    true,
	"ext3":    true,
	"ext4":    true,
	"btrfs":   true,
	"xfs":     true,
	"f2fs":    true,
	"zfs":     true,
	"vfat":    true,
	"exfat":   true,
	"ntfs":    true,
	"ntfs3":   true,
	"iso9660": true,
	"udf":     true,
	"hfsplus": true,
}

// Where the desktop mounts media somebody plugged in, as opposed to the volumes
// that are part of the machine.
var removableRoots = []string{"/run/media", "/media", "/mnt"}

// Devices lists the mounted volumes worth showing beside the places, which is
// every browsable filesystem except the root the places already reach.
func Devices() []Device {
	contents, err := os.ReadFile("/proc/self/mounts")
	if err != nil {
		return nil
	}
	return devicesFrom(parseMounts(string(contents)))
}

func devicesFrom(mounts []mount) []Device {
	devices := []Device{}
	seen := map[string]bool{}
	for _, entry := range mounts {
		devices = appendDevice(devices, seen, entry)
	}
	return devices
}

// The same volume can be mounted twice, and a subvolume mounted at several
// points; the first mount point is the one the sidebar offers. The root counts
// as one of those mounts, so a volume that also holds /home is not offered
// again under its second name.
func appendDevice(devices []Device, seen map[string]bool, entry mount) []Device {
	if !strings.HasPrefix(entry.source, "/dev/") || seen[entry.source] {
		return devices
	}
	seen[entry.source] = true
	if !isBrowsableMount(entry) {
		return devices
	}
	return append(devices, deviceFor(entry))
}

// The root is reached from the places rather than the devices, so it is only
// here to claim its volume.
func isBrowsableMount(entry mount) bool {
	if entry.mountPoint == "/" {
		return false
	}
	return browsableFilesystems[entry.filesystem]
}

func deviceFor(entry mount) Device {
	total, free := capacityOf(entry.mountPoint)
	return Device{
		Name:        deviceName(entry),
		Path:        entry.mountPoint,
		IsRemovable: isRemovableMount(entry.mountPoint),
		TotalBytes:  total,
		FreeBytes:   free,
	}
}

// The mount point's own name is what the desktop labels a volume with, since
// the automounter names the directory after the volume label.
func deviceName(entry mount) string {
	name := filepath.Base(entry.mountPoint)
	if name == "" || name == "." || name == string(filepath.Separator) {
		return filepath.Base(entry.source)
	}
	return name
}

func isRemovableMount(mountPoint string) bool {
	for _, root := range removableRoots {
		if strings.HasPrefix(mountPoint, root+"/") {
			return true
		}
	}
	return false
}

func parseMounts(contents string) []mount {
	mounts := []mount{}
	for _, line := range strings.Split(contents, "\n") {
		mounts = appendMount(mounts, line)
	}
	return mounts
}

func appendMount(mounts []mount, line string) []mount {
	fields := strings.Fields(line)
	if len(fields) < 3 {
		return mounts
	}
	return append(mounts, mount{
		source:     unescapeMountField(fields[0]),
		mountPoint: unescapeMountField(fields[1]),
		filesystem: fields[2],
	})
}

// unescapeMountField decodes the octal escapes the kernel writes for spaces,
// tabs, newlines and backslashes in mount table fields.
func unescapeMountField(field string) string {
	return mountEscapes.Replace(field)
}

var mountEscapes = strings.NewReplacer(`\040`, " ", `\011`, "\t", `\012`, "\n", `\134`, `\`)
