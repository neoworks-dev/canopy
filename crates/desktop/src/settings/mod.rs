//! Persists the preferences that would otherwise reset on every launch: view
//! mode, columns, hidden files, sort order, window geometry and the directory
//! the window was last showing.

mod store;

use canopy_model::Order;
use serde::{Deserialize, Deserializer, Serialize};

pub use store::Store;

pub(crate) const CURRENT_VERSION: i64 = 1;

/// Window geometry, in the coordinates the window manager reports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Window {
    pub width: i64,
    pub height: i64,
    pub x: i64,
    pub y: i64,
}

/// A first launch opens a window of this size.
impl Default for Window {
    fn default() -> Self {
        Window { width: 1100, height: 720, x: 0, y: 0 }
    }
}

/// The width in pixels of the panes flanking the listing. Zero means the
/// frontend's own default, so a file written before these existed still opens
/// on a sensible layout.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Panes {
    pub sidebar: i64,
    pub details: i64,
}

/// One entry in the user's keymap, appended after the built-in bindings so it
/// overrides them. What the fields mean is the frontend's business; the file
/// only has to carry them across a launch.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keybinding {
    /// A chord, or a sequence of them separated by spaces: "ctrl+c", "g g".
    pub key: String,
    /// Empty unbinds whatever the built-in keymap put on this key.
    pub command: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub when: String,
}

/// The whole persisted file. Fields the frontend owns are left empty here, so
/// their defaults live in one place rather than two: an absent view mode or
/// column list means "whatever the frontend starts with".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: i64,
    pub view_mode: String,
    /// `None` is a file that said nothing about columns, which is different
    /// from `Some` of an empty list: every column deliberately turned off.
    #[serde(deserialize_with = "null_as_default")]
    pub columns: Option<Vec<String>>,
    pub show_hidden: bool,
    pub sort: Order,
    pub last_directory: String,
    pub window: Window,
    pub panes: Panes,
    /// The sidebar groups the user has folded shut, by the frontend's own names.
    #[serde(skip_serializing_if = "Vec::is_empty", deserialize_with = "null_as_default")]
    pub collapsed_sections: Vec<String>,
    /// Whether the details pane is on screen at all, which does not depend on
    /// there being anything selected to describe.
    pub show_details: bool,
    /// Written by hand only. The store restores whatever was on disk before
    /// every write, so the application cannot overwrite them.
    #[serde(skip_serializing_if = "Vec::is_empty", deserialize_with = "null_as_default")]
    pub keybindings: Vec<Keybinding>,
}

impl Default for Settings {
    /// What a first launch, or an unreadable file, starts from.
    fn default() -> Self {
        Settings {
            version: CURRENT_VERSION,
            view_mode: String::new(),
            columns: None,
            show_hidden: false,
            sort: Order::default(),
            last_directory: String::new(),
            window: Window::default(),
            panes: Panes::default(),
            collapsed_sections: Vec::new(),
            show_details: false,
            keybindings: Vec::new(),
        }
    }
}

/// Every field name the file may carry, so the store can tell a field a newer
/// version wrote from one it knows.
pub(crate) const KNOWN_FIELDS: [&str; 11] = [
    "version",
    "viewMode",
    "columns",
    "showHidden",
    "sort",
    "lastDirectory",
    "window",
    "panes",
    "collapsedSections",
    "showDetails",
    "keybindings",
];

fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    let value = Option::<T>::deserialize(deserializer)?;
    match value {
        Some(value) => Ok(value),
        None => Ok(T::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_serialise_like_the_go_defaults() {
        let json = serde_json::to_string(&Settings::default()).unwrap();

        assert_eq!(
            json,
            r#"{"version":1,"viewMode":"","columns":null,"showHidden":false,"sort":{"key":"name","descending":false,"directoriesFirst":true},"lastDirectory":"","window":{"width":1100,"height":720,"x":0,"y":0},"panes":{"sidebar":0,"details":0},"showDetails":false}"#
        );
    }

    #[test]
    fn a_partial_file_keeps_the_defaults_for_what_it_omits() {
        let loaded: Settings = serde_json::from_str(r#"{"window":{"x":5},"sort":{"key":"size"}}"#).unwrap();

        assert_eq!(loaded.window, Window { width: 1100, height: 720, x: 5, y: 0 });
        assert_eq!(loaded.sort.key, "size");
        assert!(loaded.sort.directories_first);
    }

    #[test]
    fn null_lists_read_as_empty() {
        let loaded: Settings =
            serde_json::from_str(r#"{"columns":null,"collapsedSections":null,"keybindings":null}"#).unwrap();

        assert_eq!(loaded.columns, None);
        assert!(loaded.collapsed_sections.is_empty());
        assert!(loaded.keybindings.is_empty());
    }

    #[test]
    fn an_empty_column_list_stays_distinct_from_no_list() {
        let loaded: Settings = serde_json::from_str(r#"{"columns":[]}"#).unwrap();

        assert_eq!(loaded.columns, Some(Vec::new()));
        assert!(serde_json::to_string(&loaded).unwrap().contains(r#""columns":[]"#));
    }
}
