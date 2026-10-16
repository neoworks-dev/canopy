use std::fs;
use std::io;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use canopy_model::config_path;
use serde_json::{Map, Value};

use super::{CURRENT_VERSION, KNOWN_FIELDS, Keybinding, Settings};

const FILE_NAME: &str = "settings.json";

/// Long enough that dragging a window or clicking through columns writes once,
/// short enough that a crash loses nothing anyone would notice.
const WRITE_DELAY: Duration = Duration::from_millis(500);

/// Reads and writes the settings file. Writes are debounced, because the
/// frontend saves on every change, and are staged through a temporary file so
/// an interrupted write cannot leave a half-written one behind.
#[derive(Clone)]
pub struct Store {
    shared: Arc<Shared>,
}

struct Shared {
    path: Option<PathBuf>,
    delay: Duration,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    /// Fields a newer version of the application wrote, kept so rewriting the
    /// file here does not drop them.
    unknown_fields: Map<String, Value>,
    /// The keymap as it was last read. Nothing in the application authors one,
    /// so restoring it before every write keeps a hand-edited file intact.
    keybindings: Vec<Keybinding>,
    pending: Option<Settings>,
    /// Incremented by every save; a debounce timer writes only if no later save
    /// has replaced it.
    save_generation: u64,
}

impl Store {
    /// Resolves the settings path. A store whose path cannot be resolved still
    /// works, it just never persists, which beats refusing to start.
    pub fn open() -> Store {
        Store::new(config_path(FILE_NAME), WRITE_DELAY)
    }

    /// A store over an explicit file with its own debounce delay.
    pub fn at(path: PathBuf, delay: Duration) -> Store {
        Store::new(Some(path), delay)
    }

    fn new(path: Option<PathBuf>, delay: Duration) -> Store {
        Store { shared: Arc::new(Shared { path, delay, state: Mutex::new(State::default()) }) }
    }

    /// Reads the file, falling back to defaults when it is missing or corrupt.
    pub fn load(&self) -> Settings {
        let Some(path) = &self.shared.path else {
            return Settings::default();
        };
        let Ok(contents) = fs::read(path) else {
            return Settings::default();
        };
        self.parse(&contents)
    }

    /// Schedules a write. Repeated calls inside the delay collapse into one.
    pub fn save(&self, next: Settings) {
        let generation = {
            let mut state = self.lock();
            state.pending = Some(next);
            state.save_generation += 1;
            state.save_generation
        };
        let shared = Arc::clone(&self.shared);
        thread::spawn(move || {
            thread::sleep(shared.delay);
            shared.write_pending_if_current(generation);
        });
    }

    /// Writes anything still pending straight away, for shutdown.
    pub fn flush(&self) -> io::Result<()> {
        let pending = {
            let mut state = self.lock();
            state.save_generation += 1;
            state.pending.take()
        };
        match pending {
            Some(settings) => self.shared.write(settings),
            None => Ok(()),
        }
    }

    fn parse(&self, contents: &[u8]) -> Settings {
        let Ok(loaded) = serde_json::from_slice::<Settings>(contents) else {
            return Settings::default();
        };
        let mut state = self.lock();
        state.unknown_fields = unknown_fields_of(contents);
        state.keybindings = loaded.keybindings.clone();
        loaded
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.shared.lock()
    }

    #[cfg(test)]
    fn write_now(&self, next: Settings) -> io::Result<()> {
        self.shared.write(next)
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn write_pending_if_current(&self, generation: u64) {
        let pending = {
            let mut state = self.lock();
            if state.save_generation != generation {
                return;
            }
            state.pending.take()
        };
        if let Some(settings) = pending {
            // Nothing is watching a debounced write, so a failure leaves the
            // settings as they were rather than interrupting anyone.
            let _ = self.write(settings);
        }
    }

    fn write(&self, mut next: Settings) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        next.version = CURRENT_VERSION;
        next.keybindings = self.lock().keybindings.clone();
        let contents = self.encode(&next)?;
        write_atomically(path, &contents)
    }

    fn encode(&self, next: &Settings) -> io::Result<Vec<u8>> {
        let Value::Object(mut fields) = serde_json::to_value(next)? else {
            return Err(io::Error::other("settings did not serialise to an object"));
        };
        for (name, value) in &self.lock().unknown_fields {
            fields.insert(name.clone(), value.clone());
        }
        Ok(serde_json::to_vec_pretty(&fields)?)
    }
}

fn unknown_fields_of(contents: &[u8]) -> Map<String, Value> {
    let Ok(Value::Object(mut fields)) = serde_json::from_slice::<Value>(contents) else {
        return Map::new();
    };
    for name in KNOWN_FIELDS {
        fields.remove(name);
    }
    fields
}

fn write_atomically(path: &Path, contents: &[u8]) -> io::Result<()> {
    use std::io::Write;
    if let Some(directory) = path.parent() {
        fs::DirBuilder::new().recursive(true).mode(0o700).create(directory)?;
    }
    let mut temporary_name = path.as_os_str().to_owned();
    temporary_name.push(".tmp");
    let temporary_path = PathBuf::from(temporary_name);
    let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&temporary_path)?;
    file.write_all(contents)?;
    fs::rename(temporary_path, path)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::super::Window;
    use super::*;

    fn store_in(directory: &tempfile::TempDir) -> Store {
        Store::at(directory.path().join("settings.json"), Duration::from_millis(1))
    }

    fn settings_file(directory: &tempfile::TempDir) -> PathBuf {
        directory.path().join("settings.json")
    }

    fn read_fields(directory: &tempfile::TempDir) -> Map<String, Value> {
        let contents = fs::read(settings_file(directory)).unwrap();
        match serde_json::from_slice(&contents).unwrap() {
            Value::Object(fields) => fields,
            other => panic!("settings file holds {other}"),
        }
    }

    #[test]
    fn loading_without_a_file_returns_defaults() {
        let directory = tempfile::tempdir().unwrap();

        let loaded = store_in(&directory).load();

        assert_eq!(loaded.version, CURRENT_VERSION);
        assert_eq!(loaded.sort.key, "name");
        assert!(loaded.sort.directories_first);
    }

    #[test]
    fn settings_survive_a_write_and_read() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        let saved = Settings {
            view_mode: "grid".into(),
            columns: Some(vec!["kind".into(), "size".into()]),
            show_hidden: true,
            last_directory: "/tmp".into(),
            window: Window { width: 900, height: 600, x: 40, y: 20 },
            ..Settings::default()
        };
        store.save(saved);
        store.flush().unwrap();

        let reloaded = store_in(&directory).load();

        assert_eq!(reloaded.view_mode, "grid");
        assert!(reloaded.show_hidden);
        assert_eq!(reloaded.window, Window { width: 900, height: 600, x: 40, y: 20 });
        assert_eq!(reloaded.columns.unwrap().len(), 2);
    }

    #[test]
    fn a_corrupt_file_falls_back_to_defaults() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(settings_file(&directory), "{not json at all").unwrap();

        let loaded = store_in(&directory).load();

        assert_eq!(loaded.version, CURRENT_VERSION);
        assert_eq!(loaded.window.width, 1100);
    }

    #[test]
    fn a_field_from_a_newer_version_is_not_dropped() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        fs::write(settings_file(&directory), r#"{"version": 2, "viewMode": "grid", "somethingNewer": {"kept": true}}"#)
            .unwrap();

        store.load();
        store.save(Settings::default());
        store.flush().unwrap();

        assert!(read_fields(&directory).contains_key("somethingNewer"));
    }

    #[test]
    fn rewriting_stamps_the_current_version() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        fs::write(settings_file(&directory), r#"{"version": 99}"#).unwrap();

        store.load();
        store.save(Settings::default());
        store.flush().unwrap();

        assert_eq!(read_fields(&directory)["version"], Value::from(1));
    }

    #[test]
    fn repeated_saves_collapse_into_the_last_one() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        for mode in ["rows", "grid", "rows"] {
            store.save(Settings { view_mode: mode.into(), ..Settings::default() });
        }
        store.flush().unwrap();

        assert_eq!(store.load().view_mode, "rows");
    }

    #[test]
    fn a_debounced_save_lands_on_its_own() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        store.save(Settings { last_directory: "/var".into(), ..Settings::default() });

        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            if store.load().last_directory == "/var" {
                return;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("the debounced write never landed");
    }

    #[test]
    fn flushing_nothing_is_not_an_error() {
        let directory = tempfile::tempdir().unwrap();

        assert!(store_in(&directory).flush().is_ok());
    }

    #[test]
    fn a_store_without_a_path_never_writes() {
        let store = Store::new(None, Duration::from_millis(1));
        store.save(Settings::default());

        assert!(store.flush().is_ok());
        assert_eq!(store.load().version, CURRENT_VERSION);
    }

    #[test]
    fn a_hand_edited_keymap_is_read_back() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(
            settings_file(&directory),
            r#"{
                "version": 1,
                "keybindings": [
                    {"key": "f", "command": "search.here"},
                    {"key": "g g", "command": "list.move.first", "when": "listActive"}
                ]
            }"#,
        )
        .unwrap();

        let loaded = store_in(&directory).load();

        assert_eq!(loaded.keybindings.len(), 2);
        assert_eq!(loaded.keybindings[1].key, "g g");
        assert_eq!(loaded.keybindings[1].when, "listActive");
    }

    #[test]
    fn saving_does_not_drop_a_hand_edited_keymap() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        fs::write(settings_file(&directory), r#"{"version": 1, "keybindings": [{"key": "f", "command": "search.here"}]}"#)
            .unwrap();
        store.load();

        store.write_now(Settings { view_mode: "grid".into(), ..Settings::default() }).unwrap();

        let reloaded = store.load();
        assert_eq!(reloaded.keybindings.len(), 1);
        assert_eq!(reloaded.keybindings[0].command, "search.here");
        assert_eq!(reloaded.view_mode, "grid");
    }

    #[test]
    fn a_file_without_a_keymap_stays_without_one() {
        let directory = tempfile::tempdir().unwrap();
        store_in(&directory).write_now(Settings::default()).unwrap();

        assert!(!read_fields(&directory).contains_key("keybindings"));
    }

    #[test]
    fn collapsed_sections_saved_by_the_application_replace_the_ones_on_disk() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        fs::write(settings_file(&directory), r#"{"collapsedSections": ["places"]}"#).unwrap();
        store.load();

        store.write_now(Settings { collapsed_sections: vec!["tags".into()], ..Settings::default() }).unwrap();

        assert_eq!(store.load().collapsed_sections, ["tags"]);
    }
}
