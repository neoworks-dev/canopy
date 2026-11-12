use std::cmp::Ordering;
use std::hash::{BuildHasher, RandomState};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::entry::Entry;

/// Sort keys. An unknown or empty key lists by name.
pub const SORT_BY_NAME: &str = "name";
pub const SORT_BY_SIZE: &str = "size";
pub const SORT_BY_MODIFIED: &str = "modified";
pub const SORT_BY_PERMISSIONS: &str = "permissions";
pub const SORT_BY_EXTENSION: &str = "extension";
/// Case-insensitive, with runs of digits compared as numbers: `file2` before `file10`.
pub const SORT_BY_NATURAL: &str = "natural";
pub const SORT_BY_CREATED: &str = "created";
/// A fresh shuffle every time a listing is sorted.
pub const SORT_BY_RANDOM: &str = "random";

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
    let shuffle = RandomState::new();
    entries.sort_by(|first, second| compare(first, second, order, &shuffle));
}

fn compare(first: &Entry, second: &Entry, order: &Order, shuffle: &RandomState) -> Ordering {
    if order.directories_first && first.is_dir != second.is_dir {
        return second.is_dir.cmp(&first.is_dir);
    }
    let difference = compare_by_key(first, second, &order.key, shuffle);
    if difference == Ordering::Equal {
        // Names break every tie, so equal sizes or times still arrive in a readable order.
        return compare_names(first, second);
    }
    if order.descending {
        return difference.reverse();
    }
    difference
}

fn compare_by_key(first: &Entry, second: &Entry, key: &str, shuffle: &RandomState) -> Ordering {
    match key {
        SORT_BY_SIZE => first.size.cmp(&second.size),
        SORT_BY_MODIFIED => first.modified_at.cmp(&second.modified_at),
        SORT_BY_PERMISSIONS => first.mode.cmp(&second.mode),
        SORT_BY_EXTENSION => lowercase_extension(&first.name).cmp(&lowercase_extension(&second.name)),
        SORT_BY_NATURAL => compare_natural(&first.name, &second.name),
        SORT_BY_CREATED => first.created_at.cmp(&second.created_at),
        SORT_BY_RANDOM => shuffle.hash_one(&first.name).cmp(&shuffle.hash_one(&second.name)),
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

/// Empty for a name without one, so those sort first. A leading dot does not
/// start an extension: `.bashrc` has none.
fn lowercase_extension(name: &str) -> String {
    match Path::new(name).extension() {
        Some(extension) => extension.to_string_lossy().to_lowercase(),
        None => String::new(),
    }
}

fn compare_natural(first: &str, second: &str) -> Ordering {
    let first_folded = first.to_lowercase();
    let second_folded = second.to_lowercase();
    let first_chunks = natural_chunks(&first_folded);
    let second_chunks = natural_chunks(&second_folded);
    for (first_chunk, second_chunk) in first_chunks.iter().zip(&second_chunks) {
        let difference = compare_chunks(first_chunk, second_chunk);
        if difference != Ordering::Equal {
            return difference;
        }
    }
    first_chunks.len().cmp(&second_chunks.len())
}

/// Splits text into alternating runs of ASCII digits and everything else.
fn natural_chunks(text: &str) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut chunk_start = 0;
    let mut previous_was_digit = None;
    for (index, character) in text.char_indices() {
        let is_digit = character.is_ascii_digit();
        if previous_was_digit.is_some_and(|previous| previous != is_digit) {
            chunks.push(&text[chunk_start..index]);
            chunk_start = index;
        }
        previous_was_digit = Some(is_digit);
    }
    if chunk_start < text.len() {
        chunks.push(&text[chunk_start..]);
    }
    chunks
}

fn compare_chunks(first: &str, second: &str) -> Ordering {
    let both_numbers = starts_with_digit(first) && starts_with_digit(second);
    if !both_numbers {
        return first.cmp(second);
    }
    compare_digit_runs(first, second)
}

fn starts_with_digit(text: &str) -> bool {
    text.starts_with(|character: char| character.is_ascii_digit())
}

/// Compares digit runs by value without parsing, so runs longer than any
/// integer type still order correctly. Equal values with more leading zeros
/// sort after fewer.
fn compare_digit_runs(first: &str, second: &str) -> Ordering {
    let first_significant = first.trim_start_matches('0');
    let second_significant = second.trim_start_matches('0');
    let by_value = first_significant
        .len()
        .cmp(&second_significant.len())
        .then_with(|| first_significant.cmp(second_significant));
    by_value.then_with(|| first.len().cmp(&second.len()))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::*;

    fn entry_at(name: &str, is_dir: bool, size: i64, day: u64) -> Entry {
        let modified_at = SystemTime::UNIX_EPOCH + Duration::from_secs(day * 86_400);
        Entry { name: name.into(), is_dir, size, modified_at: Some(modified_at), mode: "-rw-r--r--".into(), ..Entry::default() }
    }

    fn created_on(name: &str, day: u64) -> Entry {
        let created_at = SystemTime::UNIX_EPOCH + Duration::from_secs(day * 86_400);
        Entry { name: name.into(), created_at: Some(created_at), ..Entry::default() }
    }

    fn named(names: &[&str]) -> Vec<Entry> {
        names.iter().map(|name| Entry { name: name.to_string(), ..Entry::default() }).collect()
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
        sort_names(sample(), order)
    }

    fn sort_names(mut entries: Vec<Entry>, order: Order) -> Vec<String> {
        sort_entries(&mut entries, &order);
        entries.into_iter().map(|entry| entry.name).collect()
    }

    fn by_key(key: &str) -> Order {
        Order { key: key.into(), descending: false, directories_first: false }
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
        assert_eq!(sorted_names(by_key(SORT_BY_SIZE)), ["Alpha", "zebra", "Apple.txt", "cherry.txt", "banana.txt"]);
    }

    #[test]
    fn sorting_by_modified() {
        assert_eq!(sorted_names(by_key(SORT_BY_MODIFIED)), ["cherry.txt", "Apple.txt", "banana.txt", "zebra", "Alpha"]);
    }

    #[test]
    fn sorting_by_extension_ignores_case_and_breaks_ties_by_name() {
        let entries = named(&["b.TXT", "a.zip", "c.png", "a.txt", "README", ".bashrc"]);
        assert_eq!(sort_names(entries, by_key(SORT_BY_EXTENSION)), [".bashrc", "README", "c.png", "a.txt", "b.TXT", "a.zip"]);
    }

    #[test]
    fn sorting_by_extension_keeps_directories_first_and_descends() {
        let mut entries = named(&["a.txt", "b.png"]);
        entries.push(Entry { name: "folder".into(), is_dir: true, ..Entry::default() });
        let order = Order { key: SORT_BY_EXTENSION.into(), descending: true, directories_first: true };
        assert_eq!(sort_names(entries, order), ["folder", "a.txt", "b.png"]);
    }

    #[test]
    fn natural_sorting_compares_digit_runs_as_numbers() {
        let entries = named(&["file10", "File2", "file1", "file02b", "file2a", "file"]);
        assert_eq!(sort_names(entries, by_key(SORT_BY_NATURAL)), ["file", "file1", "File2", "file2a", "file02b", "file10"]);
    }

    #[test]
    fn natural_sorting_handles_runs_longer_than_an_integer() {
        let entries = named(&["v123456789012345678901234567890", "v99"]);
        assert_eq!(sort_names(entries, by_key(SORT_BY_NATURAL)), ["v99", "v123456789012345678901234567890"]);
    }

    #[test]
    fn sorting_by_created() {
        let unknown = Entry { name: "unknown".into(), ..Entry::default() };
        let entries = vec![created_on("late", 9), unknown, created_on("early", 1)];
        assert_eq!(sort_names(entries, by_key(SORT_BY_CREATED)), ["unknown", "early", "late"]);
    }

    #[test]
    fn random_order_keeps_every_entry_and_directories_first() {
        let mut entries = named(&["a", "b", "c", "d", "e", "f"]);
        entries.push(Entry { name: "folder".into(), is_dir: true, ..Entry::default() });
        let order = Order { key: SORT_BY_RANDOM.into(), descending: false, directories_first: true };

        let mut shuffled = sort_names(entries, order);

        assert_eq!(shuffled[0], "folder");
        shuffled.sort();
        assert_eq!(shuffled, ["a", "b", "c", "d", "e", "f", "folder"]);
    }

    #[test]
    fn random_order_changes_between_sorts() {
        let names: Vec<String> = (0..30).map(|number| format!("entry{number}")).collect();
        let entries: Vec<Entry> = names.iter().map(|name| Entry { name: name.clone(), ..Entry::default() }).collect();
        let orders: std::collections::HashSet<Vec<String>> =
            (0..5).map(|_| sort_names(entries.clone(), by_key(SORT_BY_RANDOM))).collect();
        assert!(orders.len() > 1);
    }
}
