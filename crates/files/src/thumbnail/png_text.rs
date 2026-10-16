//! Reads and writes the PNG text chunks the thumbnail managing standard uses to
//! identify a cached image: the source it was made from and the modification
//! time it had then, so any file manager can validate the same cache.

use flate2::Crc;

use crate::error::{Error, Result};

const URI_KEYWORD: &str = "Thumb::URI";
const MTIME_KEYWORD: &str = "Thumb::MTime";

const SIGNATURE_LENGTH: usize = 8;
const LENGTH_BYTES: usize = 4;
const TYPE_BYTES: usize = 4;
const CHECKSUM_BYTES: usize = 4;
const HEADER_CHUNK_TYPE: &[u8] = b"IHDR";
const TEXT_CHUNK_TYPE: &[u8] = b"tEXt";

/// Inserts the standard text chunks after the header, so an image produced by
/// the standard encoder or by an external renderer can be annotated without
/// decoding it again.
pub(super) fn with_thumbnail_metadata(encoded: &[u8], uri: &str, modified_at: &str) -> Result<Vec<u8>> {
    let end = header_chunk_end(encoded)?;
    let mut annotated = encoded[..end].to_vec();
    annotated.extend(text_chunk(URI_KEYWORD, uri));
    annotated.extend(text_chunk(MTIME_KEYWORD, modified_at));
    annotated.extend(&encoded[end..]);
    Ok(annotated)
}

/// The source and modification time a cached image records, empty when it
/// carries neither.
pub(super) fn thumbnail_metadata(encoded: &[u8]) -> (String, String) {
    (text_value(encoded, URI_KEYWORD), text_value(encoded, MTIME_KEYWORD))
}

fn header_chunk_end(encoded: &[u8]) -> Result<usize> {
    if encoded.len() < SIGNATURE_LENGTH + LENGTH_BYTES + TYPE_BYTES {
        return Err(not_a_png());
    }
    if chunk_type_at(encoded, SIGNATURE_LENGTH) != HEADER_CHUNK_TYPE {
        return Err(not_a_png());
    }
    Ok(chunk_end(encoded, SIGNATURE_LENGTH))
}

fn not_a_png() -> Error {
    Error::message("not a PNG image")
}

fn text_value(encoded: &[u8], keyword: &str) -> String {
    let mut offset = SIGNATURE_LENGTH;
    while offset + LENGTH_BYTES + TYPE_BYTES <= encoded.len() {
        if let Some(value) = keyword_value_at(encoded, offset, keyword) {
            return value;
        }
        offset = chunk_end(encoded, offset);
    }
    String::new()
}

fn keyword_value_at(encoded: &[u8], offset: usize, keyword: &str) -> Option<String> {
    if chunk_type_at(encoded, offset) != TEXT_CHUNK_TYPE {
        return None;
    }
    let data = chunk_data(encoded, offset)?;
    let separator = data.iter().position(|byte| *byte == 0)?;
    if &data[..separator] != keyword.as_bytes() {
        return None;
    }
    Some(String::from_utf8_lossy(&data[separator + 1..]).into_owned())
}

fn text_chunk(keyword: &str, text: &str) -> Vec<u8> {
    let mut data = keyword.as_bytes().to_vec();
    data.push(0);
    data.extend(text.as_bytes());
    let mut chunk = (data.len() as u32).to_be_bytes().to_vec();
    chunk.extend(TEXT_CHUNK_TYPE);
    chunk.extend(&data);
    let mut checksum = Crc::new();
    checksum.update(&chunk[LENGTH_BYTES..]);
    chunk.extend(checksum.sum().to_be_bytes());
    chunk
}

fn chunk_type_at(encoded: &[u8], offset: usize) -> &[u8] {
    &encoded[offset + LENGTH_BYTES..offset + LENGTH_BYTES + TYPE_BYTES]
}

fn chunk_length_at(encoded: &[u8], offset: usize) -> usize {
    let bytes: [u8; 4] = encoded[offset..offset + LENGTH_BYTES].try_into().unwrap_or_default();
    u32::from_be_bytes(bytes) as usize
}

fn chunk_end(encoded: &[u8], offset: usize) -> usize {
    offset + LENGTH_BYTES + TYPE_BYTES + chunk_length_at(encoded, offset) + CHECKSUM_BYTES
}

/// The payload of the chunk at `offset`, nothing when the declared length runs
/// past the end of a truncated file.
fn chunk_data(encoded: &[u8], offset: usize) -> Option<&[u8]> {
    let start = offset + LENGTH_BYTES + TYPE_BYTES;
    let end = start + chunk_length_at(encoded, offset);
    encoded.get(start..end)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{ImageFormat, RgbaImage};

    use super::*;

    fn sample() -> Vec<u8> {
        let mut encoded = Cursor::new(Vec::new());
        RgbaImage::new(8, 8).write_to(&mut encoded, ImageFormat::Png).unwrap();
        encoded.into_inner()
    }

    #[test]
    fn metadata_survives_encoding() {
        let annotated = with_thumbnail_metadata(&sample(), "file:///photos/a.png", "1700000000").unwrap();

        let (uri, modified_at) = thumbnail_metadata(&annotated);

        assert_eq!((uri.as_str(), modified_at.as_str()), ("file:///photos/a.png", "1700000000"));
        assert!(image::load_from_memory(&annotated).is_ok(), "annotating must not break the image");
    }

    #[test]
    fn something_that_is_not_a_png_is_refused() {
        assert!(with_thumbnail_metadata(b"nowhere near a png", "file:///a", "1").is_err());
    }

    #[test]
    fn an_unannotated_image_has_no_metadata() {
        assert_eq!(thumbnail_metadata(&sample()), (String::new(), String::new()));
    }
}
