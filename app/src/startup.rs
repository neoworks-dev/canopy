//! What the window opens on: the folder named on the command line, which is how
//! a desktop starts a file manager, and the size the last session closed at.

use std::path::{Path, PathBuf};

use canopy_desktop::settings::Settings;
use svelte_gpui::Options;

/// The host's own flags that take a value; anything else not starting with `--`
/// is the folder to open.
const HOST_FLAGS_WITH_VALUES: [&str; 8] =
    ["--bundle", "--width", "--height", "--assets", "--allow-files", "--do", "--click-text", "--exit-after-ms"];

/// Opens the window at the size it was closed at, unless `--width` and
/// `--height` asked for another.
pub fn apply_window_size(options: &mut Options, settings: &Settings) {
    if options.width.is_none() {
        options.width = Some(settings.window.width as f32);
    }
    if options.height.is_none() {
        options.height = Some(settings.window.height as f32);
    }
}

/// Points the first listing at the folder named on the command line. A file
/// names the folder holding it, which makes "show in folder" work from other
/// applications; a path that is neither is ignored.
pub fn open_folder_argument(settings: &mut Settings) {
    let Some(argument) = folder_argument() else {
        return;
    };
    let Some(folder) = folder_for(Path::new(&argument)) else {
        return;
    };
    settings.last_directory = folder.to_string_lossy().into_owned();
}

fn folder_argument() -> Option<String> {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if HOST_FLAGS_WITH_VALUES.contains(&argument.as_str()) {
            arguments.next();
            continue;
        }
        if !argument.starts_with("--") {
            return Some(argument);
        }
    }
    None
}

fn folder_for(path: &Path) -> Option<PathBuf> {
    let absolute = std::path::absolute(path).ok()?;
    let metadata = std::fs::metadata(&absolute).ok()?;
    if metadata.is_dir() {
        return Some(absolute);
    }
    absolute.parent().map(Path::to_path_buf)
}
