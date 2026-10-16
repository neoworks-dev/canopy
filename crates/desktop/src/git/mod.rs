//! What a working tree says about the files in one directory: which branch is
//! checked out and which entries are modified, untracked or ignored.
//!
//! Everything here is best effort. A directory outside a repository, a git that
//! is not installed and a repository git refuses to read all come back as "no
//! repository", because none of them is a reason to fail a listing.

mod status;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The working tree a directory belongs to.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Repository {
    pub root: String,
    pub branch: String,
}

/// What one directory looks like to git: the repository it is in and the state
/// of the entries directly inside it, keyed by entry name.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub repository: Repository,
    #[serde(rename = "isRepo")]
    pub is_repo: bool,
    pub statuses: BTreeMap<String, String>,
}

const DETACHED_BRANCH_NAME: &str = "detached";

/// Reports the state of one directory. The statuses cover the entries directly
/// inside it, with a folder carrying the most notable state of anything
/// underneath it.
pub fn describe(directory: &str) -> Report {
    let Some(root) = repository_root(Path::new(directory)) else {
        return Report::default();
    };
    let branch = branch_at(&root);
    let statuses = status::statuses_in(&root, Path::new(directory));
    Report {
        repository: Repository { root: root.to_string_lossy().into_owned(), branch },
        is_repo: true,
        statuses,
    }
}

/// Walks up from a directory looking for the working tree it belongs to.
fn repository_root(directory: &Path) -> Option<PathBuf> {
    let mut current = directory;
    while !current.as_os_str().is_empty() {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
    None
}

/// Reads the checked-out branch straight out of HEAD, which is cheaper than
/// asking git and is the same answer.
fn branch_at(root: &Path) -> String {
    let Ok(head) = fs::read_to_string(head_path(root)) else {
        return String::new();
    };
    let contents = head.trim();
    match contents.strip_prefix("ref: ") {
        Some(reference) => last_segment(reference),
        None => short_commit(contents),
    }
}

fn last_segment(reference: &str) -> String {
    match reference.rfind('/') {
        Some(separator) => reference[separator + 1..].to_string(),
        None => reference.to_string(),
    }
}

/// A worktree and a submodule keep a file where the directory would be, holding
/// the path of the real git directory.
fn head_path(root: &Path) -> PathBuf {
    let marker = root.join(".git");
    if marker.is_dir() {
        return marker.join("HEAD");
    }
    git_directory_from(&marker, root).join("HEAD")
}

fn git_directory_from(marker: &Path, root: &Path) -> PathBuf {
    let Ok(contents) = fs::read_to_string(marker) else {
        return marker.to_path_buf();
    };
    let trimmed = contents.trim();
    let path = trimmed.strip_prefix("gitdir:").unwrap_or(trimmed).trim();
    root.join(path)
}

/// A detached head has no branch name, so the commit it sits on stands in for
/// one.
fn short_commit(commit: &str) -> String {
    match commit.get(..7) {
        Some(short) => short.to_string(),
        None => DETACHED_BRANCH_NAME.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    #[test]
    fn describe_reports_no_repository_outside_a_working_tree() {
        let directory = tempfile::tempdir().unwrap();

        let report = describe(directory.path().to_str().unwrap());

        assert!(!report.is_repo);
        assert_eq!(serde_json::to_string(&report.statuses).unwrap(), "{}");
    }

    #[test]
    fn describe_reports_the_checked_out_branch() {
        let root = new_repository();
        let root_path = root_text(&root);

        let report = describe(&root_path);

        assert!(report.is_repo);
        assert_eq!(report.repository.root, root_path);
        assert_eq!(report.repository.branch, "trunk");
    }

    #[test]
    fn describe_reports_a_branch_from_a_subdirectory() {
        let root = new_repository();
        write_file(&root, "src/main.go", "package main");

        let report = describe(root.path().join("src").to_str().unwrap());

        assert_eq!(report.repository.root, root_text(&root));
        assert_eq!(report.repository.branch, "trunk");
    }

    #[test]
    fn describe_marks_untracked_and_modified_entries() {
        let root = new_repository();
        write_file(&root, "tracked.txt", "first");
        commit_everything(&root);
        write_file(&root, "tracked.txt", "second");
        write_file(&root, "fresh.txt", "new");

        let statuses = describe(&root_text(&root)).statuses;

        assert_eq!(statuses["tracked.txt"], "modified");
        assert_eq!(statuses["fresh.txt"], "untracked");
    }

    #[test]
    fn describe_folds_deep_changes_into_the_folder_holding_them() {
        let root = new_repository();
        write_file(&root, "src/deep/notes.txt", "first");
        commit_everything(&root);
        write_file(&root, "src/deep/notes.txt", "second");

        assert_eq!(describe(&root_text(&root)).statuses["src"], "modified");
    }

    #[test]
    fn describe_reports_ignored_entries() {
        let root = new_repository();
        write_file(&root, ".gitignore", "build/\n");
        write_file(&root, "build/output.bin", "binary");
        commit_everything(&root);

        assert_eq!(describe(&root_text(&root)).statuses["build"], "ignored");
    }

    #[test]
    fn describe_only_covers_the_directory_being_listed() {
        let root = new_repository();
        write_file(&root, "src/main.go", "package main");
        write_file(&root, "docs/readme.md", "hello");
        commit_everything(&root);
        write_file(&root, "docs/readme.md", "changed");

        let statuses = describe(root.path().join("src").to_str().unwrap()).statuses;

        assert!(statuses.is_empty(), "got {statuses:?}");
    }

    #[test]
    fn describe_marks_a_staged_addition() {
        let root = new_repository();
        write_file(&root, "seed.txt", "seed");
        commit_everything(&root);
        write_file(&root, "added.txt", "added");
        run(&root, &["add", "added.txt"]);

        assert_eq!(describe(&root_text(&root)).statuses["added.txt"], "added");
    }

    #[test]
    fn a_report_serialises_with_the_field_names_the_frontend_reads() {
        let mut report = Report::default();
        report.statuses.insert("a".into(), "modified".into());

        let json = serde_json::to_string(&report).unwrap();

        assert_eq!(json, r#"{"repository":{"root":"","branch":""},"isRepo":false,"statuses":{"a":"modified"}}"#);
    }

    #[test]
    fn a_short_head_reads_as_detached() {
        assert_eq!(short_commit("abc"), "detached");
        assert_eq!(short_commit("0123456789"), "0123456");
    }

    fn new_repository() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        run(&root, &["init", "--initial-branch", "trunk"]);
        run(&root, &["config", "user.email", "tests@example.invalid"]);
        run(&root, &["config", "user.name", "Tests"]);
        run(&root, &["config", "commit.gpgsign", "false"]);
        root
    }

    fn root_text(root: &tempfile::TempDir) -> String {
        root.path().to_str().unwrap().to_string()
    }

    fn commit_everything(root: &tempfile::TempDir) {
        run(root, &["add", "-A"]);
        run(root, &["commit", "-m", "state"]);
    }

    fn run(root: &tempfile::TempDir, arguments: &[&str]) {
        let output = Command::new("git").args(arguments).current_dir(root.path()).output().unwrap();
        assert!(output.status.success(), "git {arguments:?}: {}", String::from_utf8_lossy(&output.stderr));
    }

    fn write_file(root: &tempfile::TempDir, name: &str, contents: &str) {
        let path = root.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
}
