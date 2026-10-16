//! Remembers what the explorer opened, newest first, so the sidebar can offer
//! it again. It records only what this application opened: nothing else on the
//! system writes to it.

use std::fs;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use canopy_model::config_path;
use chrono::{Local, SecondsFormat};
use serde::{Deserialize, Serialize};

use crate::path_text::base_name;

const FILE_NAME: &str = "recents.json";

/// Long enough to reach back past a morning's browsing, short enough that the
/// sidebar stays a list rather than a history.
const LIMIT: usize = 40;

/// One thing that was opened. The serialised form is both the file on disk and
/// what the UI reads.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub opened_at: String,
}

/// Holds the list and keeps the file behind it current. A store whose path
/// cannot be resolved still works for the session, it just never persists.
pub struct Store {
    path: Option<PathBuf>,
    entries: Mutex<Vec<Entry>>,
}

impl Store {
    /// Opens the store at the default config location.
    pub fn open() -> Store {
        match config_path(FILE_NAME) {
            Some(path) => Store::at(path),
            None => Store { path: None, entries: Mutex::new(Vec::new()) },
        }
    }

    /// Opens the store over an explicit file.
    pub fn at(path: PathBuf) -> Store {
        let entries = read(&path);
        Store { path: Some(path), entries: Mutex::new(entries) }
    }

    /// Moves a path to the front of the list, so opening something again
    /// promotes it rather than repeating it.
    pub fn record(&self, path: &str, is_dir: bool) {
        let mut entries = self.lock();
        if is_already_in_front(&entries, path) {
            return;
        }
        *entries = with_entry_in_front(&entries, entry_for(path, is_dir));
        self.write(&entries);
    }

    pub fn list(&self) -> Vec<Entry> {
        self.lock().clone()
    }

    pub fn clear(&self) {
        let mut entries = self.lock();
        entries.clear();
        self.write(&entries);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Entry>> {
        match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// A list that cannot be written is not worth reporting: the session still
    /// has it, and the next launch simply starts from what was last saved.
    fn write(&self, entries: &[Entry]) {
        let Some(path) = &self.path else {
            return;
        };
        let _ = write_entries(path, entries);
    }
}

/// Re-reading a folder the window is already sitting on is not a new visit,
/// and a watched directory is re-read whenever anything in it changes.
fn is_already_in_front(entries: &[Entry], path: &str) -> bool {
    match entries.first() {
        Some(first) => first.path == path,
        None => false,
    }
}

fn entry_for(path: &str, is_dir: bool) -> Entry {
    Entry {
        name: base_name(path),
        path: path.to_string(),
        is_dir,
        opened_at: Local::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    }
}

fn with_entry_in_front(entries: &[Entry], recorded: Entry) -> Vec<Entry> {
    let mut promoted = vec![recorded.clone()];
    promoted.extend(entries.iter().filter(|entry| entry.path != recorded.path).cloned());
    promoted.truncate(LIMIT);
    promoted
}

fn read(path: &Path) -> Vec<Entry> {
    let Ok(contents) = fs::read(path) else {
        return Vec::new();
    };
    serde_json::from_slice(&contents).unwrap_or_default()
}

fn write_entries(path: &Path, entries: &[Entry]) -> std::io::Result<()> {
    use std::io::Write;
    let contents = serde_json::to_vec_pretty(entries)?;
    if let Some(directory) = path.parent() {
        fs::DirBuilder::new().recursive(true).mode(0o700).create(directory)?;
    }
    let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    file.write_all(&contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_in_temporary_config() -> (tempfile::TempDir, Store) {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::at(directory.path().join("canopy").join(FILE_NAME));
        (directory, store)
    }

    fn reopen(directory: &tempfile::TempDir) -> Store {
        Store::at(directory.path().join("canopy").join(FILE_NAME))
    }

    #[test]
    fn record_puts_the_newest_first() {
        let (_directory, store) = store_in_temporary_config();

        store.record("/home/moritz/Projects", true);
        store.record("/home/moritz/Notes", true);

        let listed = store.list();
        assert_eq!(listed[0].path, "/home/moritz/Notes");
        assert_eq!(listed[1].path, "/home/moritz/Projects");
    }

    #[test]
    fn record_promotes_rather_than_repeats() {
        let (_directory, store) = store_in_temporary_config();

        store.record("/a", true);
        store.record("/b", true);
        store.record("/a", true);

        let listed = store.list();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].path, "/a");
    }

    #[test]
    fn record_ignores_what_is_already_in_front() {
        let (_directory, store) = store_in_temporary_config();

        store.record("/a", true);
        let mut first = store.list();
        first[0].opened_at = "kept".to_string();
        *store.lock() = first;
        store.record("/a", true);

        assert_eq!(store.list()[0].opened_at, "kept");
    }

    #[test]
    fn record_keeps_the_list_bounded() {
        let (_directory, store) = store_in_temporary_config();

        for index in 0..LIMIT + 10 {
            store.record(&format!("/folder/{index}"), true);
        }

        assert_eq!(store.list().len(), LIMIT);
    }

    #[test]
    fn recorded_entries_survive_a_reopen() {
        let (directory, store) = store_in_temporary_config();
        store.record("/home/moritz/Notes", true);
        store.record("/home/moritz/report.pdf", false);

        let listed = reopen(&directory).list();

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].path, "/home/moritz/report.pdf");
        assert!(!listed[0].is_dir);
        assert_eq!(listed[0].name, "report.pdf");
    }

    #[test]
    fn clear_empties_the_list_on_disk_as_well() {
        let (directory, store) = store_in_temporary_config();
        store.record("/a", true);

        store.clear();

        assert!(store.list().is_empty());
        assert!(reopen(&directory).list().is_empty());
    }

    #[test]
    fn entries_serialise_with_the_field_names_the_frontend_reads() {
        let entry = Entry { name: "a".into(), path: "/a".into(), is_dir: true, opened_at: "t".into() };

        let json = serde_json::to_string(&entry).unwrap();

        assert_eq!(json, r#"{"name":"a","path":"/a","isDir":true,"openedAt":"t"}"#);
    }

    #[test]
    fn an_empty_list_serialises_as_a_list() {
        let (_directory, store) = store_in_temporary_config();

        assert_eq!(serde_json::to_string(&store.list()).unwrap(), "[]");
    }
}
