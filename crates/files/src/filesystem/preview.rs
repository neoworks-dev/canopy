use std::fs::File;
use std::io::Read;
use std::path::Path;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use crate::audio;
use crate::error::Result;
use crate::filesystem::path::resolve_path;
use crate::highlight::{self, Token};
use crate::paths;
use crate::thumbnail;

/// What the explorer can show about a single file without handing it to
/// another application.
///
/// `kind` tells the UI which field to render: `text`, `image`, `audio`,
/// `directory`, or `unsupported`. `tokens` is the same text split into
/// coloured runs, `None` (JSON `null`) when the file has no highlighting, in
/// which case the pane draws `text` as it is. `data_url` carries the drawing
/// for an image, and the cover art for a sound file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub path: String,
    pub kind: String,
    pub text: String,
    pub tokens: Option<Vec<Token>>,
    pub data_url: String,
    pub truncated: bool,
    pub size: i64,
    pub audio: audio::Details,
}

/// Enough text to scroll through without pushing a huge file across the UI
/// bridge. Base64 inflates images by a third, so the image cap is the tighter
/// one. Highlighting takes whatever the text limit leaves, since the canvas
/// draws the lines on screen rather than all of them.
const TEXT_PREVIEW_LIMIT: usize = 128 * 1024;
const IMAGE_PREVIEW_LIMIT: u64 = 4 * 1024 * 1024;

const IMAGE_MEDIA_TYPES: [(&str, &str); 9] = [
    (".png", "image/png"),
    (".jpg", "image/jpeg"),
    (".jpeg", "image/jpeg"),
    (".gif", "image/gif"),
    (".webp", "image/webp"),
    (".avif", "image/avif"),
    (".bmp", "image/bmp"),
    (".ico", "image/x-icon"),
    (".svg", "image/svg+xml"),
];

/// Inspects a single path. Anything too large or binary comes back as an
/// unsupported preview rather than an error, since the pane still shows the
/// file's metadata.
pub fn read_preview(requested_path: &str) -> Result<Preview> {
    let absolute_path = resolve_path(requested_path)?;
    let metadata = std::fs::metadata(&absolute_path)?;
    let path = absolute_path.to_string_lossy().into_owned();
    if metadata.is_dir() {
        return Ok(Preview { path, kind: "directory".to_string(), ..Preview::default() });
    }
    preview_of_file(&absolute_path, metadata.len())
}

fn preview_of_file(absolute_path: &Path, size: u64) -> Result<Preview> {
    if let Some(media_type) = image_media_type(absolute_path) {
        return image_preview(absolute_path, size, media_type);
    }
    if audio::supports(absolute_path) {
        return Ok(audio_preview(absolute_path, size));
    }
    if thumbnail::supports(absolute_path) {
        return Ok(rendered_preview(absolute_path, size));
    }
    text_preview(absolute_path, size)
}

fn image_media_type(absolute_path: &Path) -> Option<&'static str> {
    let extension = paths::extension_of(absolute_path);
    let found = IMAGE_MEDIA_TYPES.iter().find(|(known, _)| *known == extension);
    found.map(|(_, media_type)| *media_type)
}

fn preview_base(absolute_path: &Path, size: u64, kind: &str) -> Preview {
    Preview {
        path: absolute_path.to_string_lossy().into_owned(),
        kind: kind.to_string(),
        size: size as i64,
        ..Preview::default()
    }
}

/// A sound file has nothing to draw of its own, so the pane gets what the
/// container says about it and the shape of the sound. Cover art comes from
/// the same renderer images use, since embedded art is a video stream to ffmpeg.
fn audio_preview(absolute_path: &Path, size: u64) -> Preview {
    let Some(details) = audio::read(absolute_path) else {
        return preview_base(absolute_path, size, "unsupported");
    };
    Preview {
        audio: details,
        data_url: thumbnail::render_preview(absolute_path),
        ..preview_base(absolute_path, size, "audio")
    }
}

/// A PDF or a video has no bytes worth showing as they are, so the pane gets
/// the same drawing the tiles use, at the size it has room for. A file nothing
/// can draw falls back to the metadata every preview carries.
fn rendered_preview(absolute_path: &Path, size: u64) -> Preview {
    let data_url = thumbnail::render_preview(absolute_path);
    if data_url.is_empty() {
        return preview_base(absolute_path, size, "unsupported");
    }
    Preview { data_url, ..preview_base(absolute_path, size, "image") }
}

fn image_preview(absolute_path: &Path, size: u64, media_type: &str) -> Result<Preview> {
    if size > IMAGE_PREVIEW_LIMIT {
        return Ok(preview_base(absolute_path, size, "unsupported"));
    }
    let contents = std::fs::read(absolute_path)?;
    let data_url = format!("data:{media_type};base64,{}", STANDARD.encode(contents));
    Ok(Preview { data_url, ..preview_base(absolute_path, size, "image") })
}

fn text_preview(absolute_path: &Path, size: u64) -> Result<Preview> {
    let contents = read_at_most(absolute_path, TEXT_PREVIEW_LIMIT)?;
    let readable = drop_partial_rune(&contents);
    if !is_textual(readable) {
        return Ok(preview_base(absolute_path, size, "unsupported"));
    }
    let text = String::from_utf8_lossy(readable).into_owned();
    let file_name = paths::base_name(absolute_path);
    Ok(Preview {
        tokens: highlight::tokens(&file_name, &text),
        truncated: size > contents.len() as u64,
        text,
        ..preview_base(absolute_path, size, "text")
    })
}

fn read_at_most(absolute_path: &Path, limit: usize) -> Result<Vec<u8>> {
    let file = File::open(absolute_path)?;
    let mut contents = Vec::new();
    file.take(limit as u64).read_to_end(&mut contents)?;
    Ok(contents)
}

/// The heuristic most editors use: valid UTF-8 with no NUL bytes is text,
/// everything else is binary.
fn is_textual(contents: &[u8]) -> bool {
    if contents.contains(&0) {
        return false;
    }
    std::str::from_utf8(contents).is_ok()
}

/// Removes a multi-byte rune the read limit cut in half, so a truncated text
/// file is not mistaken for binary.
fn drop_partial_rune(contents: &[u8]) -> &[u8] {
    let Some(start) = last_rune_start(contents) else {
        return contents;
    };
    if std::str::from_utf8(&contents[start..]).is_ok() {
        return contents;
    }
    &contents[..start]
}

/// The start of the final rune, looking back at most as far as a rune is long.
fn last_rune_start(contents: &[u8]) -> Option<usize> {
    const LONGEST_RUNE: usize = 4;
    let lowest_index = contents.len().saturating_sub(LONGEST_RUNE);
    (lowest_index..contents.len()).rev().find(|index| contents[*index] & 0xC0 != 0x80)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temporary_file(name: &str, contents: &[u8]) -> (tempfile::TempDir, String) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(name);
        std::fs::write(&path, contents).unwrap();
        let path = path.to_string_lossy().into_owned();
        (directory, path)
    }

    #[test]
    fn text_is_read() {
        let (_directory, path) = write_temporary_file("notes.md", b"# Title\n\nBody text.\n");
        let preview = read_preview(&path).unwrap();
        assert_eq!(preview.kind, "text");
        assert_eq!(preview.text, "# Title\n\nBody text.\n");
        assert!(!preview.truncated);
    }

    #[test]
    fn long_text_is_truncated() {
        let contents = "a".repeat(TEXT_PREVIEW_LIMIT + 2048);
        let (_directory, path) = write_temporary_file("big.log", contents.as_bytes());
        let preview = read_preview(&path).unwrap();
        assert_eq!(preview.kind, "text");
        assert_eq!(preview.text.len(), TEXT_PREVIEW_LIMIT);
        assert!(preview.truncated);
    }

    #[test]
    fn truncated_multibyte_text_stays_readable() {
        let contents = "ä".repeat(TEXT_PREVIEW_LIMIT);
        let (_directory, path) = write_temporary_file("umlauts.txt", contents.as_bytes());
        let preview = read_preview(&path).unwrap();
        assert_eq!(preview.kind, "text");
        assert!(!preview.text.ends_with('\u{FFFD}'));
    }

    #[test]
    fn source_comes_back_highlighted() {
        let (_directory, path) = write_temporary_file("main.go", b"package main\n");
        let preview = read_preview(&path).unwrap();
        assert!(!preview.tokens.expect("tokens").is_empty());
        assert_eq!(preview.text, "package main\n");
    }

    #[test]
    fn a_large_file_is_highlighted_as_far_as_it_was_read() {
        let source = format!("package main\n{}", "// a comment\n".repeat(TEXT_PREVIEW_LIMIT / 13));
        let (_directory, path) = write_temporary_file("big.go", source.as_bytes());
        let preview = read_preview(&path).unwrap();
        assert!(preview.truncated);
        let spelled: String = preview.tokens.unwrap().iter().map(|token| token.text.as_str()).collect();
        assert_eq!(spelled, preview.text, "the tokens spell exactly the text that was read");
    }

    #[test]
    fn plain_text_is_left_unhighlighted() {
        let (_directory, path) = write_temporary_file("notes.unknownext", b"just some words\n");
        assert!(read_preview(&path).unwrap().tokens.is_none());
    }

    #[test]
    fn binary_content_is_rejected() {
        let (_directory, path) = write_temporary_file("program.bin", &[0x7f, b'E', b'L', b'F', 0x00, 0x01]);
        let preview = read_preview(&path).unwrap();
        assert_eq!(preview.kind, "unsupported");
        assert!(preview.text.is_empty());
    }

    #[test]
    fn images_are_encoded_as_data_urls() {
        let (_directory, path) = write_temporary_file("pixel.png", &[0x89, b'P', b'N', b'G']);
        let preview = read_preview(&path).unwrap();
        assert_eq!(preview.kind, "image");
        assert!(preview.data_url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn directories_are_reported() {
        let directory = tempfile::tempdir().unwrap();
        let preview = read_preview(directory.path().to_str().unwrap()).unwrap();
        assert_eq!(preview.kind, "directory");
    }

    #[test]
    fn a_missing_path_fails() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("absent.txt");
        assert!(read_preview(missing.to_str().unwrap()).is_err());
    }

    /// A PDF holds nothing worth showing as bytes, so it is drawn rather than
    /// read. One no renderer can make sense of still reports its metadata.
    #[test]
    fn a_pdf_is_drawn_rather_than_read() {
        let (_directory, path) = write_temporary_file("broken.pdf", b"%PDF-1.7 and then nothing usable");
        let preview = read_preview(&path).unwrap();
        assert_eq!(preview.kind, "unsupported");
        assert!(preview.text.is_empty());
    }

    #[test]
    fn the_json_matches_the_go_field_names() {
        let (_directory, path) = write_temporary_file("notes.unknownext", b"words\n");
        let value = serde_json::to_value(read_preview(&path).unwrap()).unwrap();
        let keys: Vec<&str> = value.as_object().unwrap().keys().map(|key| key.as_str()).collect();
        for key in ["path", "kind", "text", "tokens", "dataUrl", "truncated", "size", "audio"] {
            assert!(keys.contains(&key), "{key}");
        }
        assert!(value["tokens"].is_null());
        assert!(value["audio"]["waveform"].is_null());
    }
}
