use std::path::PathBuf;

use super::names::is_directory;
use super::trash::{TrashLocation, volume_trash_directory};
use crate::filesystem::unescape_mount_field;

/// Finds per-volume trash directories that already exist, so items trashed on
/// removable media stay listable and restorable.
pub(super) fn volume_trash_locations() -> Vec<TrashLocation> {
    let mut locations = Vec::new();
    for mount_point in mount_points() {
        let directory = volume_trash_directory(&mount_point);
        if is_directory(&directory) {
            locations.push(TrashLocation { directory, top_directory: Some(mount_point) });
        }
    }
    locations
}

fn mount_points() -> Vec<PathBuf> {
    match std::fs::read_to_string("/proc/self/mounts") {
        Ok(contents) => parse_mount_points(&contents),
        Err(_) => Vec::new(),
    }
}

fn parse_mount_points(contents: &str) -> Vec<PathBuf> {
    let mut points = Vec::new();
    for line in contents.split('\n') {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 2 {
            continue;
        }
        points.push(PathBuf::from(unescape_mount_field(fields[1])));
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_points_decode_escapes() {
        let points = parse_mount_points("/dev/sda1 /run/media/my\\040drive ext4 rw 0 0\nproc /proc proc rw 0 0\n");
        assert_eq!(points, [PathBuf::from("/run/media/my drive"), PathBuf::from("/proc")]);
    }
}
