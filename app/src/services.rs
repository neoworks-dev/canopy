//! Everything the commands share for the life of the window: the stores behind
//! the sidebar and settings, the directory watcher and the search index.

use std::sync::{Arc, Mutex};

use canopy_desktop::settings::{self, Settings};
use canopy_desktop::{recents, searches, tags, watcher, zoxide};
use canopy_files::{diskusage, duplicates, fileops};
use canopy_index::searchindex;
use svelte_gpui::Emitter;

/// Emitted with the path of a watched directory whose contents changed.
pub const DIRECTORY_CHANGED_EVENT: &str = "watch:changed";

pub struct Services {
    /// Copies, moves, trashing, extraction and packing, with undo.
    pub engine: fileops::Engine,
    pub folder_sizes: diskusage::Scanner,
    pub duplicates: duplicates::Scanner,
    pub settings: settings::Store,
    /// The settings this launch started from, updated on every save, so the window
    /// size can be written alongside what the UI last saved.
    pub current_settings: Mutex<Settings>,
    pub zoxide: zoxide::Database,
    pub recents: recents::Store,
    pub tag_names: tags::Vocabulary,
    pub searches: searches::Store,
    pub watcher: watcher::Watcher,
    pub index: searchindex::Connection,
}

impl Services {
    pub fn start(emitter: Emitter) -> Arc<Services> {
        let settings = settings::Store::open();
        let current_settings = Mutex::new(settings.load());
        Arc::new(Services {
            engine: fileops::Engine::new(forward_operations(emitter.clone())),
            folder_sizes: diskusage::Scanner::new(forward_folder_sizes(emitter.clone())),
            duplicates: duplicates::Scanner::new(forward_duplicates(emitter.clone())),
            settings,
            current_settings,
            zoxide: zoxide::Database::new(),
            recents: recents::Store::open(),
            tag_names: tags::Vocabulary::open(),
            searches: searches::Store::open(),
            watcher: watcher::Watcher::new(directory_changed(emitter)),
            index: searchindex::Connection::new(),
        })
    }

    /// Writes what is pending, for the moment the window closes.
    pub fn shut_down(&self) {
        let _ = self.settings.flush();
        self.watcher.stop();
        self.index.close();
    }
}

fn directory_changed(emitter: Emitter) -> watcher::ChangeCallback {
    Arc::new(move |path: &str| emitter.emit(DIRECTORY_CHANGED_EVENT, serde_json::json!(path)))
}

// The engine and the scanners report from their worker threads; each event goes
// to the UI under its own name.

fn forward_operations(emitter: Emitter) -> fileops::OperationEmitter {
    Arc::new(move |event: fileops::OperationEvent| emitter.emit(event.name(), event.payload()))
}

fn forward_folder_sizes(emitter: Emitter) -> diskusage::UsageEmitter {
    Arc::new(move |event: diskusage::UsageEvent| emitter.emit(event.name(), event.payload()))
}

fn forward_duplicates(emitter: Emitter) -> duplicates::DuplicatesEmitter {
    Arc::new(move |event: duplicates::DuplicatesEvent| emitter.emit(event.name(), event.payload()))
}
