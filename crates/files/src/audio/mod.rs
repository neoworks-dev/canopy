//! What a details pane can show about a sound file: what the container says
//! about it, and the shape of the sound itself.
//!
//! Everything here is best effort. ffprobe or ffmpeg being absent, or a file
//! they cannot decode, comes back as "nothing to show" rather than an error,
//! because the pane still has the file's own metadata to fall back on.

mod probe;
mod waveform;

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::paths;
use crate::process::run_helper;

/// Long enough to decode a long track, short enough that a file ffmpeg cannot
/// make sense of does not hold the pane open.
const READ_TIME_LIMIT: Duration = Duration::from_secs(20);

const AUDIO_EXTENSIONS: [&str; 13] =
    [".mp3", ".flac", ".wav", ".ogg", ".oga", ".opus", ".m4a", ".aac", ".wma", ".aiff", ".aif", ".ape", ".wv"];

/// What one sound file reports about itself. A zero field is one the container
/// did not carry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub duration: f64,
    pub bitrate: i64,
    pub channels: i64,
    pub sample_rate: i64,
    pub codec: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Peaks between 0 and 1, in playback order; empty when nothing decoded,
    /// and `None` (JSON `null`) for a file that was never read.
    pub waveform: Option<Vec<f64>>,
}

/// Whether a path is one this module can say anything about.
pub fn supports(path: &Path) -> bool {
    supports_extension(&paths::extension_of(path))
}

/// `supports` for a caller that already has the extension (with its leading
/// dot, lowercase), such as one picking a renderer by type.
pub fn supports_extension(extension: &str) -> bool {
    AUDIO_EXTENSIONS.contains(&extension)
}

/// Describes one sound file. `None` means there is nothing worth showing,
/// which is what a missing ffprobe looks like.
pub fn read(absolute_path: &Path) -> Option<Details> {
    let probed = run_probe(absolute_path)?;
    let mut details = probe::parse_probe(&probed).ok()?;
    details.waveform = Some(waveform::read_waveform(absolute_path, details.duration));
    Some(details)
}

fn run_probe(absolute_path: &Path) -> Option<Vec<u8>> {
    let arguments = [
        "-v".to_string(),
        "error".to_string(),
        "-print_format".to_string(),
        "json".to_string(),
        "-show_format".to_string(),
        "-show_streams".to_string(),
        absolute_path.to_string_lossy().into_owned(),
    ];
    run_helper("ffprobe", &arguments, READ_TIME_LIMIT).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_extension_is_read_whatever_its_case() {
        assert!(supports(Path::new("/music/A Song.FLAC")));
        assert!(!supports(Path::new("/music/notes.txt")));
        assert!(!supports(Path::new("/music/no-extension")));
    }
}
