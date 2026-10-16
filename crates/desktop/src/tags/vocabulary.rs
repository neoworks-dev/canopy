use std::fs;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use canopy_model::config_path;

use super::{normalize, without};

const VOCABULARY_FILE_NAME: &str = "tags.json";

/// The list of tag names the sidebar offers.
///
/// The tags themselves live on the files; this is only the set of names that
/// have been used, which is not something the filesystem can be asked for
/// without walking all of it. Removing a name here hides it from the sidebar
/// and leaves every file that carries it untouched.
pub struct Vocabulary {
    path: Option<PathBuf>,
    names: Mutex<Vec<String>>,
}

impl Vocabulary {
    /// Opens the vocabulary at the default config location.
    pub fn open() -> Vocabulary {
        match config_path(VOCABULARY_FILE_NAME) {
            Some(path) => Vocabulary::at(path),
            None => Vocabulary { path: None, names: Mutex::new(Vec::new()) },
        }
    }

    /// Opens the vocabulary over an explicit file.
    pub fn at(path: PathBuf) -> Vocabulary {
        let names = read(&path);
        Vocabulary { path: Some(path), names: Mutex::new(names) }
    }

    pub fn names(&self) -> Vec<String> {
        self.lock().clone()
    }

    /// Adds names that are now in use, which is how tagging a file for the first
    /// time puts its tag in the sidebar.
    pub fn remember(&self, new_names: &[String]) {
        let mut names = self.lock();
        let mut combined = names.clone();
        combined.extend_from_slice(new_names);
        *names = normalize(&combined);
        self.write(&names);
    }

    pub fn forget(&self, name: &str) {
        let mut names = self.lock();
        *names = without(&names, name);
        self.write(&names);
    }

    fn lock(&self) -> MutexGuard<'_, Vec<String>> {
        match self.names.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// A list that cannot be written costs the sidebar a name after the next
    /// launch, which is not worth failing a tagging action over.
    fn write(&self, names: &[String]) {
        let Some(path) = &self.path else {
            return;
        };
        let _ = write_names(path, names);
    }
}

fn read(path: &Path) -> Vec<String> {
    let Ok(contents) = fs::read(path) else {
        return Vec::new();
    };
    match serde_json::from_slice::<Vec<String>>(&contents) {
        Ok(names) => normalize(&names),
        Err(_) => Vec::new(),
    }
}

fn write_names(path: &Path, names: &[String]) -> std::io::Result<()> {
    let contents = serde_json::to_vec_pretty(names)?;
    if let Some(directory) = path.parent() {
        fs::DirBuilder::new().recursive(true).mode(0o700).create(directory)?;
    }
    let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    file.write_all(&contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn vocabulary_file() -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("canopy").join(VOCABULARY_FILE_NAME);
        (directory, file)
    }

    #[test]
    fn remembers_names_across_launches() {
        let (_directory, file) = vocabulary_file();

        Vocabulary::at(file.clone()).remember(&names(&["work", "work", " personal "]));

        assert_eq!(Vocabulary::at(file).names(), ["personal", "work"]);
    }

    #[test]
    fn forgetting_a_name_leaves_the_files_alone() {
        let (_directory, file) = vocabulary_file();
        let vocabulary = Vocabulary::at(file);
        vocabulary.remember(&names(&["work"]));

        vocabulary.forget("work");

        assert!(vocabulary.names().is_empty());
    }

    #[test]
    fn an_empty_vocabulary_serialises_as_a_list() {
        let (_directory, file) = vocabulary_file();

        assert_eq!(serde_json::to_string(&Vocabulary::at(file).names()).unwrap(), "[]");
    }
}
