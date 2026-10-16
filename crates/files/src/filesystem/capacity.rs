use std::path::Path;

/// The size of a mounted filesystem and how much of it is free to an
/// unprivileged user. A volume that cannot be measured reports zero, which the
/// sidebar shows as no capacity rather than as an empty disk.
pub(super) fn capacity_of(mount_point: &Path) -> (u64, u64) {
    let Ok(statistics) = rustix::fs::statfs(mount_point) else {
        return (0, 0);
    };
    let block_size = statistics.f_bsize as u64;
    (statistics.f_blocks * block_size, statistics.f_bavail * block_size)
}
