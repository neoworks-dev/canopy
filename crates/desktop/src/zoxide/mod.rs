//! Reads the frecency-ranked directories zoxide keeps, so the explorer can jump
//! to a folder by a fragment of its name.
//!
//! The database is read through `zoxide query --list --score` rather than by
//! parsing `db.zo`: the file is a bincode dump of zoxide's own structs whose
//! layout has changed between releases, while the command is a published
//! interface. The cost of a process per keystroke is avoided by dumping the
//! whole list once and matching in process.

mod matching;

use std::fs;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use matching::{matches_keywords, split_keywords};

const BINARY_NAME: &str = "zoxide";

/// Long enough that typing a query never spawns more than one process, short
/// enough that a directory visited in a terminal shows up while the window is
/// still open.
const CACHE_LIFETIME: Duration = Duration::from_secs(5);

/// One directory zoxide knows about, with the frecency score it ranks by.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub path: String,
    pub score: f64,
}

/// Holds the last dump of zoxide's list. A missing binary is recorded once and
/// every later query answers empty, so an explorer on a machine without zoxide
/// pays nothing and reports nothing.
pub struct Database {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    entries: Vec<Entry>,
    read_at: Option<Instant>,
    installed: bool,
    checked: bool,
}

impl Default for Database {
    fn default() -> Self {
        Database::new()
    }
}

impl Database {
    pub fn new() -> Database {
        Database { state: Mutex::new(State::default()) }
    }

    /// Ranks the directories matching `text`, most frecent first. An empty text
    /// returns the top of the list. The directory in `exclude` is left out,
    /// since jumping to where the window already is means nothing.
    pub fn query(&self, text: &str, exclude: &str, limit: usize) -> Vec<Entry> {
        let keywords = split_keywords(text);
        let mut matched = Vec::new();
        for entry in self.list() {
            if matched.len() >= limit {
                break;
            }
            if is_usable(&entry, &keywords, exclude) {
                matched.push(entry);
            }
        }
        matched
    }

    /// Whether zoxide is installed, for hiding a prompt that could never return
    /// anything.
    pub fn available(&self) -> bool {
        self.list();
        self.lock().installed
    }

    fn list(&self) -> Vec<Entry> {
        let mut state = self.lock();
        if should_read(&state) {
            read_into(&mut state);
        }
        state.entries.clone()
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// A database over a fixed list, so a test never depends on the machine's
    /// own zoxide data.
    #[cfg(test)]
    fn seeded(entries: Vec<Entry>) -> Database {
        Database {
            state: Mutex::new(State { entries, read_at: Some(Instant::now()), installed: true, checked: true }),
        }
    }
}

fn is_usable(entry: &Entry, keywords: &[String], exclude: &str) -> bool {
    if entry.path == exclude || !matches_keywords(&entry.path, keywords) {
        return false;
    }
    is_directory(&entry.path)
}

/// A directory zoxide remembers may since have been deleted or unmounted, and
/// is only checked once it is a candidate for the answer.
fn is_directory(path: &str) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

fn should_read(state: &State) -> bool {
    if state.checked && !state.installed {
        return false;
    }
    match state.read_at {
        Some(read_at) => read_at.elapsed() > CACHE_LIFETIME,
        None => true,
    }
}

fn read_into(state: &mut State) {
    let output = Command::new(BINARY_NAME)
        .args(["query", "--list", "--score"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    state.checked = true;
    state.read_at = Some(Instant::now());
    match output {
        Ok(output) if output.status.success() => {
            state.installed = true;
            state.entries = parse_list(&String::from_utf8_lossy(&output.stdout));
        }
        _ => {
            state.installed = false;
            state.entries = Vec::new();
        }
    }
}

/// Reads the `  112.0 /path/to/directory` lines zoxide prints. A line that does
/// not look like one is skipped rather than failing the whole dump.
///
/// The list is ranked here rather than trusted to arrive ranked, because a
/// query stops as soon as it has filled its limit and would otherwise return
/// whichever matches came first rather than the most frecent ones.
fn parse_list(output: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = output.split('\n').filter_map(parse_line).collect();
    entries.sort_by(|first, second| second.score.total_cmp(&first.score));
    entries
}

fn parse_line(line: &str) -> Option<Entry> {
    let (score_text, rest) = line.trim().split_once(' ')?;
    let score = score_text.parse::<f64>().ok()?;
    let path = rest.trim_start_matches(' ');
    if path.is_empty() {
        return None;
    }
    Some(Entry { path: path.to_string(), score })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn entry(path: &str, score: f64) -> Entry {
        Entry { path: path.to_string(), score }
    }

    fn make_directory(root: &Path, name: &str) -> String {
        let path = root.join(name);
        fs::create_dir_all(&path).unwrap();
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn parse_list_reads_scores_and_paths() {
        let entries = parse_list(" 112.0 /home/moritz/Documents\n  54.0 /home/moritz/git-worktree-editor\n");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], entry("/home/moritz/Documents", 112.0));
    }

    #[test]
    fn parse_list_ranks_by_score() {
        let entries = parse_list("  10.0 /home/moritz/notes\n  90.0 /home/moritz/projects\n  50.0 /home/moritz/archive\n");

        let scores: Vec<f64> = entries.iter().map(|entry| entry.score).collect();
        assert_eq!(scores, [90.0, 50.0, 10.0]);
    }

    #[test]
    fn parse_list_keeps_paths_containing_spaces() {
        let entries = parse_list("  10.5 /home/moritz/My Documents/old stuff\n");

        assert_eq!(entries, [entry("/home/moritz/My Documents/old stuff", 10.5)]);
    }

    #[test]
    fn parse_list_skips_unreadable_lines() {
        let entries = parse_list("not a score line\n\n 12.0 /home/moritz\nalso bad\n");

        assert_eq!(entries, [entry("/home/moritz", 12.0)]);
    }

    #[test]
    fn query_ranks_matches_and_honours_the_limit() {
        let root = tempfile::tempdir().unwrap();
        let database = Database::seeded(vec![
            entry(&make_directory(root.path(), "notes"), 10.0),
            entry(&make_directory(root.path(), "projects"), 90.0),
            entry(&make_directory(root.path(), "project-archive"), 50.0),
        ]);

        let found = database.query("project", "", 10);

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].score, 90.0);
        assert_eq!(database.query("project", "", 1).len(), 1);
    }

    #[test]
    fn query_leaves_out_the_excluded_directory() {
        let root = tempfile::tempdir().unwrap();
        let here = make_directory(root.path(), "projects");
        let database = Database::seeded(vec![entry(&here, 90.0)]);

        assert!(database.query("project", &here, 10).is_empty());
    }

    #[test]
    fn query_skips_directories_that_are_gone() {
        let database = Database::seeded(vec![entry("/definitely/not/here/projects", 90.0)]);

        assert!(database.query("project", "", 10).is_empty());
    }

    #[test]
    fn an_empty_query_returns_the_top_of_the_list() {
        let root = tempfile::tempdir().unwrap();
        let database = Database::seeded(vec![
            entry(&make_directory(root.path(), "notes"), 10.0),
            entry(&make_directory(root.path(), "projects"), 90.0),
        ]);

        assert_eq!(database.query("", "", 10).len(), 2);
    }

    #[test]
    fn entries_serialise_with_the_field_names_the_frontend_reads() {
        assert_eq!(serde_json::to_string(&entry("/a", 1.5)).unwrap(), r#"{"path":"/a","score":1.5}"#);
    }
}
