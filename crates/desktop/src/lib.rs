//! The desktop-facing services of Canopy: bookmarks, clipboard, git status,
//! application launching, recents, saved searches, settings, tags, directory
//! watching and zoxide.

pub mod bookmarks;
pub mod clipboard;
pub mod git;
pub mod launcher;
pub mod recents;
pub mod searches;
pub mod settings;
pub mod tags;
pub mod watcher;
pub mod zoxide;

mod atomic_write;
mod file_uri;
mod path_text;
