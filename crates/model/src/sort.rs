use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::entry::Entry;

/// Sort keys. An unknown or empty key lists by name.
pub const SORT_BY_NAME: &str = "name";
pub const SORT_BY_SIZE: &str = "size";
pub const SORT_BY_MODIFIED: &str = "modified";
pub const SORT_BY_PERMISSIONS: &str = "permissions";

/// How a listing is sorted. Directories-first is independent of the key, so
/// sorting by size still keeps folders together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Order {
    pub key: String,
    pub descending: bool,
    pub directories_first: bool,
}

impl Default for Order {
    fn default() -> Self {
        Order { key: SORT_BY_NAME.to_string(), descending: false, directories_first: true }
    }
}

pub fn sort_entries(entries: &mut [Entry], order: &Order) {
    entries.sort_by(|first, second| compare(first, second, order));
}

fn compare(first: &Entry, second: &Entry, order: &Order) -> Ordering {
    if order.directories_first && first.is_dir != second.is_dir {
        return second.is_dir.cmp(&first.is_dir);
    }
    let difference = compare_by_key(first, second, &order.key);
    if difference == Ordering::Equal {
        // Names break every tie, so equal sizes or times still arrive in a readable order.
        return compare_names(first, second);
    }
    if order.descending {
        return difference.reverse();
    }
    difference
}

fn compare_by_key(first: &Entry, second: &Entry, key: &str) -> Ordering {
    match key {
        SORT_BY_SIZE => first.size.cmp(&second.size),
        SORT_BY_MODIFIED => first.modified_at.cmp(&second.modified_at),
        SORT_BY_PERMISSIONS => first.mode.cmp(&second.mode),
        _ => compare_names(first, second),
    }
}

/// Case-insensitive, falling back to the raw name so names differing only in
/// case still have a defined order.
fn compare_names(first: &Entry, second: &Entry) -> Ordering {
    let folded = first.name.to_lowercase().cmp(&second.name.to_lowercase());
    if folded != Ordering::Equal {
        return folded;
    }
    first.name.cmp(&second.name)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::*;

    fn entry_at(name: &str, is_dir: bool, size: i64, day: u64) -> Entry {
        let modified_at = SystemTime::UNIX_EPOCH + Duration::from_secs(day * 86_400);
        Entry { name: name.into(), is_dir, size, modified_at: Some(modified_at), mode: "-rw-r--r--".into(), ..Entry::default() }
    }

    fn sample() -> Vec<Entry> {
        vec![
            entry_at("banana.txt", false, 300, 3),
            entry_at("Apple.txt", false, 100, 2),
            entry_at("cherry.txt", false, 200, 1),
            entry_at("zebra", true, 0, 4),
            entry_at("Alpha", true, 0, 5),
        ]
    }

    fn sorted_names(order: Order) -> Vec<String> {
        let mut entries = sample();
        sort_entries(&mut entries, &order);
        entries.into_iter().map(|entry| entry.name).collect()
    }

    #[test]
    fn default_order_lists_directories_first_then_by_name() {
        assert_eq!(sorted_names(Order::default()), ["Alpha", "zebra", "Apple.txt", "banana.txt", "cherry.txt"]);
    }

    #[test]
    fn descending_reverses_the_key_but_not_the_directory_grouping() {
        let order = Order { descending: true, ..Order::default() };
        assert_eq!(sorted_names(order), ["zebra", "Alpha", "cherry.txt", "banana.txt", "Apple.txt"]);
    }

    #[test]
    fn sorting_by_size_without_grouping() {
        let order = Order { key: SORT_BY_SIZE.into(), descending: false, directories_first: false };
        assert_eq!(sorted_names(order), ["Alpha", "zebra", "Apple.txt", "cherry.txt", "banana.txt"]);
    }

    #[test]
    fn sorting_by_modified() {
        let order = Order { key: SORT_BY_MODIFIED.into(), descending: false, directories_first: false };
        assert_eq!(sorted_names(order), ["cherry.txt", "Apple.txt", "banana.txt", "zebra", "Alpha"]);
    }
}
