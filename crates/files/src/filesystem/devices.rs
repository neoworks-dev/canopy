use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::filesystem::capacity::capacity_of;
use crate::paths;

/// One mounted volume the sidebar can navigate to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub name: String,
    pub path: String,
    pub is_removable: bool,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

/// One line of the kernel's mount table.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Mount {
    source: String,
    mount_point: String,
    filesystem: String,
}

/// Filesystems that hold files somebody would browse. Everything else on the
/// mount table is a kernel interface, a container overlay or a duplicate view
/// of a volume already listed.
const BROWSABLE_FILESYSTEMS: [&str; 14] = [
    "ext2", "ext3", "ext4", "btrfs", "xfs", "f2fs", "zfs", "vfat", "exfat", "ntfs", "ntfs3", "iso9660", "udf",
    "hfsplus",
];

/// Where the desktop mounts media somebody plugged in, as opposed to the
/// volumes that are part of the machine.
const REMOVABLE_ROOTS: [&str; 3] = ["/run/media", "/media", "/mnt"];

/// Lists the mounted volumes worth showing beside the places, which is every
/// browsable filesystem except the root the places already reach.
pub fn devices() -> Vec<Device> {
    let Ok(contents) = std::fs::read_to_string("/proc/self/mounts") else {
        return Vec::new();
    };
    devices_from(&parse_mounts(&contents))
}

fn devices_from(mounts: &[Mount]) -> Vec<Device> {
    let mut devices = Vec::new();
    let mut seen_sources = HashSet::new();
    for mount in mounts {
        if let Some(device) = device_for_new_source(mount, &mut seen_sources) {
            devices.push(device);
        }
    }
    devices
}

/// The same volume can be mounted twice, and a subvolume mounted at several
/// points; the first mount point is the one the sidebar offers. The root counts
/// as one of those mounts, so a volume that also holds /home is not offered
/// again under its second name.
fn device_for_new_source(mount: &Mount, seen_sources: &mut HashSet<String>) -> Option<Device> {
    if !mount.source.starts_with("/dev/") || !seen_sources.insert(mount.source.clone()) {
        return None;
    }
    if !is_browsable_mount(mount) {
        return None;
    }
    Some(device_for(mount))
}

/// The root is reached from the places rather than the devices, so it is only
/// listed here to claim its volume.
fn is_browsable_mount(mount: &Mount) -> bool {
    if mount.mount_point == "/" {
        return false;
    }
    BROWSABLE_FILESYSTEMS.contains(&mount.filesystem.as_str())
}

fn device_for(mount: &Mount) -> Device {
    let (total_bytes, free_bytes) = capacity_of(Path::new(&mount.mount_point));
    Device {
        name: device_name(mount),
        path: mount.mount_point.clone(),
        is_removable: is_removable_mount(&mount.mount_point),
        total_bytes,
        free_bytes,
    }
}

/// The mount point's own name is what the desktop labels a volume with, since
/// the automounter names the directory after the volume label.
fn device_name(mount: &Mount) -> String {
    let name = paths::base_name(Path::new(&mount.mount_point));
    if name.is_empty() || name == "." || name == "/" {
        return paths::base_name(Path::new(&mount.source));
    }
    name
}

fn is_removable_mount(mount_point: &str) -> bool {
    REMOVABLE_ROOTS.iter().any(|root| mount_point.starts_with(&format!("{root}/")))
}

fn parse_mounts(contents: &str) -> Vec<Mount> {
    let mut mounts = Vec::new();
    for line in contents.split('\n') {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            continue;
        }
        mounts.push(Mount {
            source: unescape_mount_field(fields[0]),
            mount_point: unescape_mount_field(fields[1]),
            filesystem: fields[2].to_string(),
        });
    }
    mounts
}

/// Decodes the octal escapes the kernel writes for spaces, tabs, newlines and
/// backslashes in mount table fields.
pub(crate) fn unescape_mount_field(field: &str) -> String {
    field.replace("\\040", " ").replace("\\011", "\t").replace("\\012", "\n").replace("\\134", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MOUNTS: &str = "proc /proc proc rw,nosuid 0 0
/dev/nvme0n1p2 / ext4 rw,relatime 0 0
/dev/nvme0n1p1 /boot vfat rw,relatime 0 0
tmpfs /run tmpfs rw,nosuid 0 0
/dev/sda1 /run/media/moritz/Field\\040Notes exfat rw,relatime 0 0
/dev/nvme0n1p2 /home btrfs rw,relatime,subvol=home 0 0
";

    fn sample_devices() -> Vec<Device> {
        devices_from(&parse_mounts(SAMPLE_MOUNTS))
    }

    #[test]
    fn browsable_volumes_are_kept_and_names_decoded() {
        let devices = sample_devices();
        assert_eq!(devices.len(), 2, "the boot partition and the stick");
        assert_eq!(devices[0].path, "/boot");
        assert_eq!(devices[1].name, "Field Notes");
    }

    #[test]
    fn the_root_and_pseudo_filesystems_are_left_out() {
        for device in sample_devices() {
            assert!(!["/", "/run", "/proc"].contains(&device.path.as_str()), "{}", device.path);
        }
    }

    #[test]
    fn a_volume_mounted_twice_is_listed_once() {
        assert!(sample_devices().iter().all(|device| device.path != "/home"));
    }

    #[test]
    fn media_under_run_media_is_removable() {
        let devices = sample_devices();
        assert!(!devices[0].is_removable);
        assert!(devices[1].is_removable);
    }

    #[test]
    fn no_mounts_is_an_empty_list() {
        assert!(devices_from(&[]).is_empty());
    }
}
