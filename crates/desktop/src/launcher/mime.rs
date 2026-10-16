use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

const DIRECTORY_MIME_TYPE: &str = "inode/directory";
const FALLBACK_MIME_TYPE: &str = "application/octet-stream";
const SNIFF_LIMIT: usize = 512;

/// Names the type of a path, preferring the extension because it is cheap and
/// matches what the XDG associations are keyed on, and sniffing the contents
/// only when the extension says nothing.
pub fn mime_type_of(path: &str) -> String {
    if is_directory(path) {
        return DIRECTORY_MIME_TYPE.to_string();
    }
    if let Some(by_extension) = type_by_extension(path) {
        return by_extension;
    }
    sniff_mime_type(path)
}

fn is_directory(path: &str) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

fn type_by_extension(path: &str) -> Option<String> {
    let extension = Path::new(path).extension()?.to_str()?;
    let guessed = mime_guess::from_ext(extension).first()?;
    Some(guessed.essence_str().to_string())
}

fn sniff_mime_type(path: &str) -> String {
    let Ok(mut file) = File::open(path) else {
        return FALLBACK_MIME_TYPE.to_string();
    };
    let mut header = vec![0u8; SNIFF_LIMIT];
    match file.read(&mut header) {
        Ok(read_count) if read_count > 0 => detect_content_type(&header[..read_count]).to_string(),
        _ => FALLBACK_MIME_TYPE.to_string(),
    }
}

/// A fixed table of leading-byte signatures, in the order they are tried. A
/// pattern byte of `?` matches anything.
const SIGNATURES: &[(&[u8], &str)] = &[
    (b"%PDF-", "application/pdf"),
    (b"%!PS-Adobe-", "application/postscript"),
    (b"GIF87a", "image/gif"),
    (b"GIF89a", "image/gif"),
    (b"\x89PNG\x0D\x0A\x1A\x0A", "image/png"),
    (b"\xFF\xD8\xFF", "image/jpeg"),
    (b"BM", "image/bmp"),
    (b"RIFF????WEBP", "image/webp"),
    (b"\x00\x00\x01\x00", "image/x-icon"),
    (b"\x00\x00\x02\x00", "image/x-icon"),
    (b"OggS\x00", "application/ogg"),
    (b"RIFF????WAVE", "audio/wave"),
    (b"FORM????AIFF", "audio/aiff"),
    (b"ID3", "audio/mpeg"),
    (b"MThd\x00\x00\x00\x06", "audio/midi"),
    (b"RIFF????AVI ", "video/avi"),
    (b"\x1A\x45\xDF\xA3", "video/webm"),
    (b"????ftyp", "video/mp4"),
    (b"wOFF\x00\x01\x00\x00", "font/woff"),
    (b"wOF2\x00\x01\x00\x00", "font/woff2"),
    (b"\x1F\x8B\x08", "application/x-gzip"),
    (b"PK\x03\x04", "application/zip"),
    (b"Rar!\x1A\x07\x00", "application/x-rar-compressed"),
    (b"\x00asm", "application/wasm"),
    (b"\xEF\xBB\xBF", "text/plain"),
    (b"\xFE\xFF", "text/plain"),
    (b"\xFF\xFE", "text/plain"),
];

const HTML_TAGS: &[&str] = &[
    "<!DOCTYPE HTML", "<HTML", "<HEAD", "<SCRIPT", "<IFRAME", "<H1", "<DIV", "<FONT", "<TABLE", "<A", "<STYLE",
    "<TITLE", "<B", "<BODY", "<BR", "<P", "<!--",
];

/// A reduced form of the WHATWG sniffing algorithm: the common binary
/// signatures, HTML and XML, then text versus binary by the bytes present.
fn detect_content_type(data: &[u8]) -> &'static str {
    let trimmed = skip_leading_whitespace(data);
    if starts_with_html_tag(trimmed) {
        return "text/html";
    }
    if trimmed.starts_with(b"<?xml") {
        return "text/xml";
    }
    for (pattern, mime_type) in SIGNATURES {
        if matches_signature(data, pattern) {
            return mime_type;
        }
    }
    if contains_binary_byte(data) {
        return FALLBACK_MIME_TYPE;
    }
    "text/plain"
}

fn skip_leading_whitespace(data: &[u8]) -> &[u8] {
    let start = data.iter().position(|byte| !matches!(byte, b'\t' | b'\n' | 0x0C | b'\r' | b' '));
    match start {
        Some(index) => &data[index..],
        None => &[],
    }
}

fn starts_with_html_tag(data: &[u8]) -> bool {
    HTML_TAGS.iter().any(|tag| matches_html_tag(data, tag.as_bytes()))
}

fn matches_html_tag(data: &[u8], tag: &[u8]) -> bool {
    if data.len() < tag.len() + 1 {
        return false;
    }
    if !data[..tag.len()].eq_ignore_ascii_case(tag) {
        return false;
    }
    matches!(data[tag.len()], b' ' | b'>')
}

fn matches_signature(data: &[u8], pattern: &[u8]) -> bool {
    if data.len() < pattern.len() {
        return false;
    }
    pattern.iter().zip(data).all(|(expected, actual)| *expected == b'?' || expected == actual)
}

fn contains_binary_byte(data: &[u8]) -> bool {
    data.iter().any(|byte| matches!(byte, 0x00..=0x08 | 0x0B | 0x0E..=0x1A | 0x1C..=0x1F))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_directories_and_known_extensions() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(mime_type_of(directory.path().to_str().unwrap()), "inode/directory");
        let page = directory.path().join("page.html");
        fs::write(&page, "<p>hi</p>").unwrap();

        assert_eq!(mime_type_of(page.to_str().unwrap()), "text/html");
    }

    #[test]
    fn sniffs_an_unknown_extension() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("payload.unknownext");
        fs::write(&path, "plain words").unwrap();

        assert_eq!(mime_type_of(path.to_str().unwrap()), "text/plain");
    }

    #[test]
    fn sniffs_binary_signatures() {
        assert_eq!(detect_content_type(b"\x89PNG\x0D\x0A\x1A\x0A\x00\x00"), "image/png");
        assert_eq!(detect_content_type(b"RIFF\x01\x02\x03\x04WEBPVP8 "), "image/webp");
        assert_eq!(detect_content_type(b"  <!doctype html><p>"), "text/html");
        assert_eq!(detect_content_type(b"\x00\x01\x02 binary"), "application/octet-stream");
    }

    #[test]
    fn an_unreadable_path_is_a_generic_binary() {
        assert_eq!(mime_type_of("/definitely/not/here.unknownext"), "application/octet-stream");
    }
}
