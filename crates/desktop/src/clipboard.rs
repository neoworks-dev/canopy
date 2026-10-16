//! Exchanges file selections with other applications through the
//! `text/uri-list` target, which is what file managers agree on.

use std::io::{self, Write};
use std::process::{Command, Stdio};

use crate::file_uri;

const URI_LIST_TYPE: &str = "text/uri-list";
const NO_CLIPBOARD_TOOL: &str = "neither wl-copy nor xclip is installed";

/// Tried in order, so a Wayland session picks wl-clipboard and an X11 one falls
/// through to xclip.
const WRITERS: &[ClipboardCommand] = &[
    ClipboardCommand { name: "wl-copy", arguments: &["--type", URI_LIST_TYPE] },
    ClipboardCommand { name: "xclip", arguments: &["-selection", "clipboard", "-t", URI_LIST_TYPE, "-i"] },
];

const READERS: &[ClipboardCommand] = &[
    ClipboardCommand { name: "wl-paste", arguments: &["--no-newline", "--type", URI_LIST_TYPE] },
    ClipboardCommand { name: "xclip", arguments: &["-selection", "clipboard", "-t", URI_LIST_TYPE, "-o"] },
];

struct ClipboardCommand {
    name: &'static str,
    arguments: &'static [&'static str],
}

/// Publishes a selection so other applications can paste it.
pub fn write_paths(paths: &[String]) -> io::Result<()> {
    let payload = to_uris(paths).join("\r\n") + "\r\n";
    for writer in WRITERS {
        if writer.feed(&payload).is_ok() {
            return Ok(());
        }
    }
    Err(no_clipboard_tool())
}

/// The file paths another application put on the clipboard.
pub fn read_paths() -> io::Result<Vec<String>> {
    for reader in READERS {
        if let Ok(output) = reader.capture() {
            return Ok(to_paths(&output));
        }
    }
    Err(no_clipboard_tool())
}

fn no_clipboard_tool() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, NO_CLIPBOARD_TOOL)
}

impl ClipboardCommand {
    fn feed(&self, payload: &str) -> io::Result<()> {
        let mut child = Command::new(self.name)
            .args(self.arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(payload.as_bytes())?;
        }
        let status = child.wait()?;
        if status.success() {
            return Ok(());
        }
        Err(io::Error::other(format!("{} failed", self.name)))
    }

    fn capture(&self) -> io::Result<String> {
        let output = Command::new(self.name).args(self.arguments).stdin(Stdio::null()).stderr(Stdio::null()).output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!("{} failed", self.name)));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

fn to_uris(paths: &[String]) -> Vec<String> {
    paths.iter().map(|path| file_uri::from_path(path)).collect()
}

/// Keeps only local files: a uri-list may carry comments and remote URIs this
/// explorer cannot act on.
fn to_paths(payload: &str) -> Vec<String> {
    payload.split('\n').filter_map(local_path_on_line).collect()
}

fn local_path_on_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    file_uri::to_path(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_uris_escapes_paths() {
        assert_eq!(to_uris(&["/home/user/a b.txt".to_string()]), ["file:///home/user/a%20b.txt"]);
    }

    #[test]
    fn to_paths_skips_comments_and_remote_uris() {
        let paths = to_paths("# comment\r\nfile:///home/user/a%20b.txt\r\nhttps://example.org/remote.txt\r\n\r\n");

        assert_eq!(paths, ["/home/user/a b.txt"]);
    }
}
