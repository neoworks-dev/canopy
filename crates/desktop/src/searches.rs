//! Keeps the queries a user has saved, so a search worth running twice is in
//! the sidebar rather than retyped.
//!
//! A saved search is the query and the folder it was confined to, not its
//! results: the index moves on, and running it again is what makes it useful.

use std::fs;
use std::io;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use canopy_model::config_path;
use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "searches.json";

/// One saved query. The name is what the sidebar calls it and what identifies
/// it: two searches cannot share a name.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Search {
    pub name: String,
    pub query: String,
    /// The folder the search is confined to, empty for every indexed root.
    pub scope: String,
}

/// The saved searches on disk. Reads answer from memory, and every change is
/// written straight back out.
pub struct Store {
    path: Option<PathBuf>,
    saved: Mutex<Vec<Search>>,
}

impl Store {
    /// Opens the store at the default config location.
    pub fn open() -> Store {
        match config_path(FILE_NAME) {
            Some(path) => Store::at(path),
            None => Store { path: None, saved: Mutex::new(Vec::new()) },
        }
    }

    /// Opens the store over an explicit file.
    pub fn at(path: PathBuf) -> Store {
        let saved = read(&path);
        Store { path: Some(path), saved: Mutex::new(saved) }
    }

    pub fn list(&self) -> Vec<Search> {
        self.lock().clone()
    }

    /// Adds a search, replacing one of the same name so saving twice under a
    /// name the user recognises updates it rather than growing a duplicate. A
    /// search with no name or no query is silently not saved.
    pub fn save(&self, search: Search) -> io::Result<()> {
        let Some(prepared) = prepare(&search) else {
            return Ok(());
        };
        let mut saved = self.lock();
        *saved = with_search(&saved, prepared);
        self.write(&saved)
    }

    pub fn forget(&self, name: &str) -> io::Result<()> {
        let mut saved = self.lock();
        *saved = without_name(&saved, name.trim());
        self.write(&saved)
    }

    /// Keeps the query and moves it under a new name, which is what the
    /// sidebar's inline edit does.
    pub fn rename(&self, name: &str, new_name: &str) -> io::Result<()> {
        let Some(mut renamed) = self.search_named(name) else {
            return Ok(());
        };
        renamed.name = new_name.to_string();
        self.forget(name)?;
        self.save(renamed)
    }

    fn search_named(&self, name: &str) -> Option<Search> {
        let wanted = name.trim();
        self.lock().iter().find(|saved| saved.name == wanted).cloned()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Search>> {
        match self.saved.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn write(&self, saved: &[Search]) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Err(io::Error::other("the config directory cannot be determined"));
        };
        write_searches(path, saved)
    }
}

fn prepare(search: &Search) -> Option<Search> {
    let prepared = Search {
        name: search.name.trim().to_string(),
        query: search.query.trim().to_string(),
        scope: search.scope.trim().to_string(),
    };
    if prepared.name.is_empty() || prepared.query.is_empty() {
        return None;
    }
    Some(prepared)
}

fn with_search(saved: &[Search], search: Search) -> Vec<Search> {
    let mut updated = without_name(saved, &search.name);
    updated.push(search);
    updated
}

fn without_name(saved: &[Search], name: &str) -> Vec<Search> {
    saved.iter().filter(|candidate| candidate.name != name).cloned().collect()
}

/// A missing or unreadable file means nothing has been saved yet.
fn read(path: &Path) -> Vec<Search> {
    let Ok(contents) = fs::read(path) else {
        return Vec::new();
    };
    let Ok(saved) = serde_json::from_slice::<Vec<Search>>(&contents) else {
        return Vec::new();
    };
    saved.iter().filter_map(prepare).collect()
}

fn write_searches(path: &Path, saved: &[Search]) -> io::Result<()> {
    use std::io::Write;
    let contents = to_tab_indented_json(saved)?;
    if let Some(directory) = path.parent() {
        fs::DirBuilder::new().recursive(true).mode(0o700).create(directory)?;
    }
    let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    file.write_all(&contents)
}

fn to_tab_indented_json(saved: &[Search]) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, formatter);
    saved.serialize(&mut serializer)?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(name: &str, query: &str, scope: &str) -> Search {
        Search { name: name.into(), query: query.into(), scope: scope.into() }
    }

    fn store_file() -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("canopy").join(FILE_NAME);
        (directory, file)
    }

    fn write_search_file(file: &Path, contents: &str) {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, contents).unwrap();
    }

    #[test]
    fn saved_searches_survive_reopening() {
        let (_directory, file) = store_file();
        Store::at(file.clone()).save(search("Invoices", "invoice", "/home/user/docs")).unwrap();

        let reopened = Store::at(file).list();

        assert_eq!(reopened, [search("Invoices", "invoice", "/home/user/docs")]);
    }

    #[test]
    fn saving_under_a_name_in_use_replaces_it() {
        let (_directory, file) = store_file();
        let store = Store::at(file);

        store.save(search("Invoices", "invoice", "")).unwrap();
        store.save(search("Invoices", "receipt", "")).unwrap();

        let saved = store.list();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].query, "receipt");
    }

    #[test]
    fn an_empty_query_is_not_a_search() {
        let (_directory, file) = store_file();
        let store = Store::at(file);

        store.save(search("Nothing", "  ", "")).unwrap();

        assert!(store.list().is_empty());
    }

    #[test]
    fn an_unnamed_search_is_not_saved() {
        let (_directory, file) = store_file();
        let store = Store::at(file);

        store.save(search("", "invoice", "")).unwrap();

        assert!(store.list().is_empty());
    }

    #[test]
    fn names_and_queries_are_trimmed() {
        let (_directory, file) = store_file();
        let store = Store::at(file);

        store.save(search("  Invoices  ", "  invoice  ", "")).unwrap();

        let saved = store.list();
        assert_eq!(saved[0].name, "Invoices");
        assert_eq!(saved[0].query, "invoice");
    }

    #[test]
    fn forgetting_a_search_leaves_the_rest() {
        let (_directory, file) = store_file();
        let store = Store::at(file.clone());
        store.save(search("Invoices", "invoice", "")).unwrap();
        store.save(search("Photos", "jpg", "")).unwrap();

        store.forget("Invoices").unwrap();

        let saved = Store::at(file).list();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].name, "Photos");
    }

    #[test]
    fn forgetting_something_unsaved_is_not_a_failure() {
        let (_directory, file) = store_file();

        assert!(Store::at(file).forget("Never saved").is_ok());
    }

    #[test]
    fn renaming_keeps_the_query_and_the_scope() {
        let (_directory, file) = store_file();
        let store = Store::at(file.clone());
        store.save(search("Invoices", "invoice", "/home/user/docs")).unwrap();

        store.rename("Invoices", "Bills").unwrap();

        assert_eq!(Store::at(file).list(), [search("Bills", "invoice", "/home/user/docs")]);
    }

    #[test]
    fn renaming_something_unsaved_changes_nothing() {
        let (_directory, file) = store_file();
        let store = Store::at(file);
        store.save(search("Photos", "jpg", "")).unwrap();

        store.rename("Invoices", "Bills").unwrap();

        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn an_unreadable_file_reads_as_nothing_saved() {
        let (_directory, file) = store_file();
        write_search_file(&file, "not json");

        assert!(Store::at(file).list().is_empty());
    }

    #[test]
    fn an_entry_without_a_query_is_dropped_on_reading() {
        let (_directory, file) = store_file();
        write_search_file(&file, r#"[{"name":"Broken"},{"name":"Photos","query":"jpg"}]"#);

        let saved = Store::at(file).list();

        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].name, "Photos");
    }

    #[test]
    fn list_hands_out_a_copy() {
        let (_directory, file) = store_file();
        let store = Store::at(file);
        store.save(search("Photos", "jpg", "")).unwrap();

        store.list()[0].name = "Changed".to_string();

        assert_eq!(store.list()[0].name, "Photos");
    }

    #[test]
    fn the_file_is_tab_indented() {
        let (_directory, file) = store_file();
        Store::at(file.clone()).save(search("A", "b", "")).unwrap();

        let contents = fs::read_to_string(file).unwrap();

        assert_eq!(contents, "[\n\t{\n\t\t\"name\": \"A\",\n\t\t\"query\": \"b\",\n\t\t\"scope\": \"\"\n\t}\n]");
    }
}
