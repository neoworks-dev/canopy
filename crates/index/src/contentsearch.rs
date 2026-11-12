//! Searching file contents under one directory with ripgrep. The answer has
//! the shape of an index search, so the UI shows both the same way.

use std::io::{self, BufRead, BufReader};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::proto::{Hit, SearchResult};

/// More than anyone reads through; ripgrep is stopped once it is reached.
pub const CONTENT_SEARCH_LIMIT: usize = 1000;

const RIPGREP: &str = "rg";

/// Files under `directory` containing `query`, case-insensitive unless the
/// query has an uppercase letter. Hidden and ignored files are skipped as
/// ripgrep skips them by default. An empty query matches nothing.
pub fn search_contents(directory: &str, query: &str) -> io::Result<SearchResult> {
    if query.is_empty() {
        return Ok(SearchResult::default());
    }
    require_directory(directory)?;
    let mut ripgrep = spawn_ripgrep(directory, query)?;
    let Some(output) = ripgrep.stdout.take() else {
        return Err(io::Error::other("ripgrep produced no output"));
    };
    let matches = read_matches(BufReader::new(output), CONTENT_SEARCH_LIMIT);
    let _ = ripgrep.kill();
    let _ = ripgrep.wait();
    Ok(to_result(Path::new(directory), matches))
}

fn require_directory(directory: &str) -> io::Result<()> {
    if !std::fs::metadata(directory)?.is_dir() {
        return Err(io::Error::other(format!("{directory} is not a directory")));
    }
    Ok(())
}

/// `--null` ends each path with a NUL so names holding newlines survive.
fn spawn_ripgrep(directory: &str, query: &str) -> io::Result<Child> {
    let spawned = Command::new(RIPGREP)
        .args(["--files-with-matches", "--smart-case", "--fixed-strings", "--null", "--", query])
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    match spawned {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(io::Error::other("ripgrep (rg) is not installed")),
        other => other,
    }
}

/// Paths relative to the searched directory, as ripgrep prints them.
#[derive(Debug, PartialEq)]
struct Matches {
    relative_paths: Vec<String>,
    truncated: bool,
}

/// Reads NUL-terminated paths up to `limit`; one more means there were more.
fn read_matches(reader: impl BufRead, limit: usize) -> Matches {
    let mut relative_paths = Vec::new();
    for record in reader.split(b'\0') {
        let Ok(record) = record else {
            break;
        };
        if relative_paths.len() == limit {
            return Matches { relative_paths, truncated: true };
        }
        relative_paths.push(String::from_utf8_lossy(&record).into_owned());
    }
    Matches { relative_paths, truncated: false }
}

fn to_result(directory: &Path, matches: Matches) -> SearchResult {
    let hits = matches.relative_paths.iter().map(|relative_path| hit_for(&directory.join(relative_path))).collect();
    SearchResult { hits, truncated: matches.truncated }
}

/// A file that vanished since ripgrep read it keeps its path and name.
fn hit_for(path: &Path) -> Hit {
    let mut hit = Hit {
        path: path.to_string_lossy().into_owned(),
        name: path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default(),
        ..Hit::default()
    };
    let Ok(metadata) = std::fs::metadata(path) else {
        return hit;
    };
    hit.is_dir = metadata.is_dir();
    hit.size = metadata.len() as i64;
    hit.mod_time_unix = metadata.mtime();
    hit
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn ripgrep_installed() -> bool {
        Command::new(RIPGREP).arg("--version").stdout(Stdio::null()).status().is_ok()
    }

    #[test]
    fn matches_are_split_on_nul() {
        let matches = read_matches(&b"a.txt\0sub/line\nbreak.md\0"[..], 10);

        assert_eq!(matches, Matches { relative_paths: vec!["a.txt".into(), "sub/line\nbreak.md".into()], truncated: false });
    }

    #[test]
    fn matches_beyond_the_limit_are_dropped_and_reported() {
        let matches = read_matches(&b"a\0b\0c\0"[..], 2);
        assert_eq!(matches, Matches { relative_paths: vec!["a".into(), "b".into()], truncated: true });

        let exactly = read_matches(&b"a\0b\0"[..], 2);
        assert!(!exactly.truncated);
    }

    #[test]
    fn hits_are_described_from_the_filesystem() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("sub")).unwrap();
        fs::write(directory.path().join("sub").join("notes.txt"), "hello").unwrap();
        let matches = Matches { relative_paths: vec!["sub/notes.txt".into(), "gone.txt".into()], truncated: false };

        let result = to_result(directory.path(), matches);

        assert_eq!(result.hits[0].name, "notes.txt");
        assert_eq!(result.hits[0].path, directory.path().join("sub/notes.txt").to_str().unwrap());
        assert_eq!(result.hits[0].size, 5);
        assert!(result.hits[0].mod_time_unix > 0);
        assert_eq!(result.hits[1].name, "gone.txt");
        assert_eq!(result.hits[1].size, 0);
    }

    #[test]
    fn an_empty_query_finds_nothing() {
        assert_eq!(search_contents("/nonexistent", "").unwrap(), SearchResult::default());
    }

    #[test]
    fn ripgrep_finds_files_by_their_contents() {
        if !ripgrep_installed() {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("match.txt"), "the Needle is here").unwrap();
        fs::write(directory.path().join("other.txt"), "nothing").unwrap();
        fs::write(directory.path().join("regex.txt"), "a.b").unwrap();

        let found = search_contents(directory.path().to_str().unwrap(), "needle").unwrap();
        let fixed = search_contents(directory.path().to_str().unwrap(), "a.b").unwrap();

        assert_eq!(found.hits.len(), 1);
        assert_eq!(found.hits[0].path, directory.path().join("match.txt").to_str().unwrap());
        assert_eq!(fixed.hits.len(), 1);
        assert_eq!(fixed.hits[0].name, "regex.txt");
    }
}
