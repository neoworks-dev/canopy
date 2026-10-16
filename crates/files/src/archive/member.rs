use std::time::{Duration, SystemTime};

use chrono::{Local, TimeZone};
use serde::{Deserialize, Serialize};

use canopy_model::format_time;

/// One entry inside an archive. The name is the path it holds inside, always
/// slash separated and never leading with one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub name: String,
    pub size: i64,
    pub is_dir: bool,
    pub mod_time: String,
    pub mode: String,
}

/// Permissions in the `ls -l` style: a type letter, then `rwx` for owner, group
/// and others.
pub(super) fn mode_text(is_dir: bool, permission_bits: u32) -> String {
    let mut text = String::with_capacity(10);
    if is_dir {
        text.push('d');
    } else {
        text.push('-');
    }
    for (bit, letter) in [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ] {
        if permission_bits & bit != 0 {
            text.push(letter);
        } else {
            text.push('-');
        }
    }
    text
}

/// What a time reads as when an archive records none.
const UNKNOWN_TIME: &str = "0001-01-01T00:00:00Z";

/// A tar header's modification time, in seconds since the epoch.
pub(super) fn tar_time_text(seconds_since_epoch: Option<u64>) -> String {
    match seconds_since_epoch {
        Some(seconds) => format_time(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)),
        None => UNKNOWN_TIME.to_string(),
    }
}

/// A zip entry's modification time. The format stores a local calendar time
/// with no zone, which is read the way every unzip tool reads it.
pub(super) fn zip_time_text(modified: Option<zip::DateTime>) -> String {
    let Some(modified) = modified else {
        return UNKNOWN_TIME.to_string();
    };
    let local = Local.with_ymd_and_hms(
        i32::from(modified.year()),
        u32::from(modified.month()),
        u32::from(modified.day()),
        u32::from(modified.hour()),
        u32::from(modified.minute()),
        u32::from(modified.second()),
    );
    match local.earliest() {
        Some(time) => format_time(time.into()),
        None => UNKNOWN_TIME.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_read_like_ls() {
        assert_eq!(mode_text(false, 0o644), "-rw-r--r--");
        assert_eq!(mode_text(true, 0o755), "drwxr-xr-x");
    }
}
