use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// A repository that takes longer than this to report is one whose status is
/// not worth holding a listing for.
const STATUS_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// What an entry can be, most notable first. A folder shows whichever of these
/// comes first among everything underneath it.
const STATUS_PRIORITY: [&str; 7] = ["conflicted", "modified", "added", "deleted", "renamed", "untracked", "ignored"];

/// The two porcelain columns are the staged and unstaged states; a file that is
/// both staged and changed again reads as modified either way.
fn status_for_code(code: u8) -> Option<&'static str> {
    match code {
        b'M' | b'T' => Some("modified"),
        b'A' | b'C' => Some("added"),
        b'D' => Some("deleted"),
        b'R' => Some("renamed"),
        b'U' => Some("conflicted"),
        _ => None,
    }
}

/// The state of the entries directly inside a directory, keyed by entry name.
pub(super) fn statuses_in(root: &Path, directory: &Path) -> BTreeMap<String, String> {
    let prefix = relative_prefix(root, directory);
    if prefix.is_empty() && directory != root {
        return BTreeMap::new();
    }
    fold_into_entries(&run_status(directory), &prefix)
}

/// The pathspec confines the walk to the directory being listed, so a listing
/// inside a large repository does not pay for the whole of it.
fn run_status(directory: &Path) -> Vec<String> {
    let spawned = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(["status", "--porcelain=v1", "-z", "--ignored=matching", "--untracked-files=normal", "--", "."])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(child) = spawned else {
        return Vec::new();
    };
    match collect_output(child) {
        Some(output) => output.split('\0').map(str::to_string).collect(),
        None => Vec::new(),
    }
}

fn collect_output(mut child: std::process::Child) -> Option<String> {
    let mut stdout = child.stdout.take()?;
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let succeeded = wait_within_timeout(&mut child);
    let bytes = reader.join().ok()?;
    if !succeeded {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn wait_within_timeout(child: &mut std::process::Child) -> bool {
    let deadline = Instant::now() + STATUS_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {}
            Err(_) => return false,
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return false;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Where the listed directory sits inside the repository, with a trailing
/// separator, empty at the root.
fn relative_prefix(root: &Path, directory: &Path) -> String {
    let Ok(relative) = directory.strip_prefix(root) else {
        return String::new();
    };
    if relative.as_os_str().is_empty() {
        return String::new();
    }
    format!("{}/", relative.to_string_lossy())
}

/// Turns paths anywhere under the directory into the entry that holds them, so
/// a folder carries the state of its contents.
fn fold_into_entries(records: &[String], prefix: &str) -> BTreeMap<String, String> {
    let mut statuses: BTreeMap<String, String> = BTreeMap::new();
    for record in records {
        let Some((name, status)) = parse_record(record, prefix) else {
            continue;
        };
        if outranks_current(statuses.get(&name), status) {
            statuses.insert(name, status.to_string());
        }
    }
    statuses
}

/// A record is two status columns, a space and a path. The rename form carries
/// a second path in the next record, which has no columns of its own and is
/// skipped by the same length check.
fn parse_record(record: &str, prefix: &str) -> Option<(String, &'static str)> {
    let bytes = record.as_bytes();
    if bytes.len() < 4 || bytes[2] != b' ' {
        return None;
    }
    let status = status_for(bytes[0], bytes[1])?;
    let name = entry_name_of(record.get(3..)?, prefix)?;
    Some((name, status))
}

fn status_for(staged: u8, unstaged: u8) -> Option<&'static str> {
    if staged == b'?' {
        return Some("untracked");
    }
    if staged == b'!' {
        return Some("ignored");
    }
    match status_for_code(unstaged) {
        Some(status) => Some(status),
        None => status_for_code(staged),
    }
}

/// The first path segment below the listed directory, which is the entry
/// standing for everything deeper.
fn entry_name_of(path: &str, prefix: &str) -> Option<String> {
    let trimmed = path.strip_suffix('/').unwrap_or(path);
    let inside = trimmed.strip_prefix(prefix)?;
    if inside.is_empty() {
        return None;
    }
    let first_segment = inside.split('/').next()?;
    Some(first_segment.to_string())
}

fn outranks_current(current: Option<&String>, candidate: &str) -> bool {
    match current {
        Some(current) => rank_of(candidate) < rank_of(current),
        None => true,
    }
}

fn rank_of(status: &str) -> usize {
    match STATUS_PRIORITY.iter().position(|known| *known == status) {
        Some(rank) => rank,
        None => STATUS_PRIORITY.len(),
    }
}
