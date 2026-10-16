use std::path::Path;

use walkdir::WalkDir;

use super::operation::Operation;
use crate::error::Result;

/// Adds up the bytes of a file, or of every file below a directory. Entries it
/// cannot read contribute nothing rather than failing the sizing: a total that
/// is slightly low is better than no progress bar at all.
pub(super) fn tree_size(path: &Path, operation: &Operation) -> Result<i64> {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return Ok(0);
    };
    if !metadata.is_dir() {
        return Ok(metadata.len() as i64);
    }
    directory_tree_size(path, operation)
}

fn directory_tree_size(path: &Path, operation: &Operation) -> Result<i64> {
    let mut total = 0;
    let walker = WalkDir::new(path).follow_links(false).follow_root_links(false);
    for entry in walker {
        let Ok(entry) = entry else {
            continue;
        };
        total += entry_size(&entry);
        operation.wait_while_paused()?;
    }
    Ok(total)
}

fn entry_size(entry: &walkdir::DirEntry) -> i64 {
    if entry.file_type().is_dir() {
        return 0;
    }
    match entry.metadata() {
        Ok(metadata) => metadata.len() as i64,
        Err(_) => 0,
    }
}

/// The byte total of the first `count` sources, which is where the counter
/// snaps to once a source is finished.
pub(super) fn prefix_sum(sizes: &[i64], count: usize) -> i64 {
    sizes.iter().take(count).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prefix_sum_stops_at_the_count_or_the_end() {
        assert_eq!(prefix_sum(&[3, 4, 5], 2), 7);
        assert_eq!(prefix_sum(&[3, 4, 5], 10), 12);
        assert_eq!(prefix_sum(&[], 1), 0);
    }
}
