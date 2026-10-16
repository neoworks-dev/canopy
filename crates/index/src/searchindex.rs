//! The explorer's side of the shared Neoworks daemon. It hides the daemon's
//! lifecycle from the rest of the application: the first search connects,
//! starting the daemon if no other Neoworks application has already.

use std::fs;
use std::sync::{Arc, Mutex, PoisonError};

use canopy_model::Entry;

use crate::client::{Client, ClientResult, Options};
use crate::proto::{Hit, SearchParams, SearchResult, StatusResult};

const CLIENT_NAME: &str = "canopy";

/// What fits in the results view without paging.
const DEFAULT_SEARCH_LIMIT: i64 = 50;

#[derive(Default)]
struct ConnectionState {
    daemon_client: Option<Arc<Client>>,
}

/// Lazily holds one daemon client. Connecting on first use rather than at
/// startup keeps a daemon that fails to start from blocking the window.
#[derive(Default)]
pub struct Connection {
    state: Mutex<ConnectionState>,
}

impl Connection {
    pub fn new() -> Self {
        Connection::default()
    }

    /// Matches file and directory names. A scope limits the results to one
    /// directory and everything under it; an empty scope searches the indexed
    /// roots.
    pub fn search(&self, text: &str, scope: &str) -> ClientResult<SearchResult> {
        let daemon_client = self.ensure_client()?;
        daemon_client.search(&SearchParams {
            text: text.to_string(),
            limit: DEFAULT_SEARCH_LIMIT,
            scope: scope.to_string(),
            ..SearchParams::default()
        })
    }

    /// Runs a query and describes each hit the way a listing describes its own,
    /// so a saved search can be shown as a folder rather than as a list of
    /// suggestions. A hit the index still holds but the filesystem has lost is
    /// left out: the crawl runs behind the tree it follows.
    pub fn search_entries(&self, text: &str, scope: &str) -> ClientResult<Vec<Entry>> {
        let result = self.search(text, scope)?;
        Ok(describe_hits(&result.hits))
    }

    pub fn status(&self) -> ClientResult<StatusResult> {
        self.ensure_client()?.status()
    }

    /// Drops the connection. The daemon keeps running for other applications and
    /// exits on its own idle timeout.
    pub fn close(&self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(daemon_client) = state.daemon_client.take() {
            daemon_client.close();
        }
    }

    fn ensure_client(&self) -> ClientResult<Arc<Client>> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(daemon_client) = &state.daemon_client {
            return Ok(Arc::clone(daemon_client));
        }
        let options = Options { client_name: CLIENT_NAME.to_string(), ..Options::default() };
        let daemon_client = Arc::new(Client::connect(options)?);
        state.daemon_client = Some(Arc::clone(&daemon_client));
        register_home_directory(&daemon_client);
        Ok(daemon_client)
    }
}

/// Makes the explorer useful on first run without asking the user to configure
/// anything. A failure is not fatal: search still works over whatever roots
/// another Neoworks application already registered.
fn register_home_directory(daemon_client: &Client) {
    let Some(home_directory) = std::env::home_dir() else {
        return;
    };
    let _ = daemon_client.add_root(&home_directory.to_string_lossy());
}

fn describe_hits(hits: &[Hit]) -> Vec<Entry> {
    hits.iter()
        .filter(|hit| fs::symlink_metadata(&hit.path).is_ok())
        .map(|hit| canopy_model::describe(&hit.path))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_hits_leaves_out_paths_the_filesystem_lost() {
        let directory = tempfile::tempdir().unwrap();
        let existing = directory.path().join("kept.txt");
        fs::write(&existing, "x").unwrap();
        let hit_for = |path: &std::path::Path| Hit { path: path.to_string_lossy().into_owned(), ..Hit::default() };

        let entries = describe_hits(&[hit_for(&existing), hit_for(&directory.path().join("gone.txt"))]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "kept.txt");
    }
}
