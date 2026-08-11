package filesystem

import "testing"

const sampleMounts = `proc /proc proc rw,nosuid 0 0
/dev/nvme0n1p2 / ext4 rw,relatime 0 0
/dev/nvme0n1p1 /boot vfat rw,relatime 0 0
tmpfs /run tmpfs rw,nosuid 0 0
/dev/sda1 /run/media/moritz/Field\040Notes exfat rw,relatime 0 0
/dev/nvme0n1p2 /home btrfs rw,relatime,subvol=home 0 0
`

func TestDevicesFromKeepsBrowsableVolumes(t *testing.T) {
	devices := devicesFrom(parseMounts(sampleMounts))

	if len(devices) != 2 {
		t.Fatalf("got %d devices, want the boot partition and the stick", len(devices))
	}
	if devices[0].Path != "/boot" {
		t.Errorf("first device = %q, want /boot", devices[0].Path)
	}
	if devices[1].Name != "Field Notes" {
		t.Errorf("name = %q, want the escaped mount point decoded", devices[1].Name)
	}
}

func TestDevicesFromLeavesOutTheRootAndPseudoFilesystems(t *testing.T) {
	for _, device := range devicesFrom(parseMounts(sampleMounts)) {
		if device.Path == "/" || device.Path == "/run" || device.Path == "/proc" {
			t.Errorf("%q should not be offered as a device", device.Path)
		}
	}
}

// One volume mounted twice is one device, or a subvolume layout would fill the
// sidebar with the same disk.
func TestDevicesFromListsAVolumeOnce(t *testing.T) {
	for _, device := range devicesFrom(parseMounts(sampleMounts)) {
		if device.Path == "/home" {
			t.Fatalf("the root volume's second mount point should not be a device")
		}
	}
}

func TestDevicesFromMarksRemovableMedia(t *testing.T) {
	devices := devicesFrom(parseMounts(sampleMounts))

	if devices[0].IsRemovable {
		t.Errorf("the boot partition is not removable")
	}
	if !devices[1].IsRemovable {
		t.Errorf("media mounted under /run/media is removable")
	}
}
