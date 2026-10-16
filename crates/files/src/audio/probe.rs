use std::collections::HashMap;

use serde::Deserialize;

use crate::audio::Details;
use crate::error::Result;

/// The part of ffprobe's JSON this module reads.
#[derive(Deserialize, Default)]
struct ProbeReport {
    #[serde(default)]
    streams: Vec<ProbeStream>,
    #[serde(default)]
    format: ProbeFormat,
}

#[derive(Deserialize, Default)]
struct ProbeStream {
    #[serde(default)]
    codec_type: String,
    #[serde(default)]
    codec_name: String,
    #[serde(default)]
    channels: i64,
    #[serde(default)]
    sample_rate: String,
    #[serde(default)]
    bit_rate: String,
}

#[derive(Deserialize, Default)]
struct ProbeFormat {
    #[serde(default)]
    duration: String,
    #[serde(default)]
    bit_rate: String,
    #[serde(default)]
    tags: HashMap<String, String>,
}

/// Reads ffprobe's JSON output into the details it carries.
pub(super) fn parse_probe(probed: &[u8]) -> Result<Details> {
    let report: ProbeReport =
        serde_json::from_slice(probed).map_err(|error| crate::Error::message(error.to_string()))?;
    let details = details_from_format(&report.format);
    Ok(with_stream(details, first_audio_stream(&report.streams)))
}

fn details_from_format(format: &ProbeFormat) -> Details {
    Details {
        duration: parse_number(&format.duration),
        bitrate: parse_number(&format.bit_rate) as i64,
        title: tag_value(&format.tags, &["title"]),
        artist: tag_value(&format.tags, &["artist", "album_artist", "performer"]),
        album: tag_value(&format.tags, &["album"]),
        ..Details::default()
    }
}

fn with_stream(mut details: Details, stream: Option<&ProbeStream>) -> Details {
    let Some(stream) = stream else {
        return details;
    };
    details.codec = stream.codec_name.clone();
    details.channels = stream.channels;
    details.sample_rate = parse_number(&stream.sample_rate) as i64;
    if details.bitrate == 0 {
        details.bitrate = parse_number(&stream.bit_rate) as i64;
    }
    details
}

/// A file with cover art carries a video stream as well, so the audio stream
/// is picked by type rather than by position.
fn first_audio_stream(streams: &[ProbeStream]) -> Option<&ProbeStream> {
    streams.iter().find(|stream| stream.codec_type == "audio")
}

/// Tag names differ by container, and ffprobe lowercases none of them.
fn tag_value(tags: &HashMap<String, String>, names: &[&str]) -> String {
    for name in names {
        if let Some((_, value)) = tags.iter().find(|(key, value)| key.eq_ignore_ascii_case(name) && !value.is_empty()) {
            return value.clone();
        }
    }
    String::new()
}

fn parse_number(value: &str) -> f64 {
    value.parse().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROBE_OUTPUT: &str = r#"{
      "streams": [
        {"codec_type": "video", "codec_name": "mjpeg"},
        {"codec_type": "audio", "codec_name": "flac", "channels": 2, "sample_rate": "44100", "bit_rate": "900000"}
      ],
      "format": {
        "duration": "213.746000",
        "bit_rate": "1024000",
        "tags": {"TITLE": "A Song", "ARTIST": "Somebody", "album": "A Record"}
      }
    }"#;

    #[test]
    fn the_format_and_the_audio_stream_are_read() {
        let details = parse_probe(PROBE_OUTPUT.as_bytes()).unwrap();
        assert_eq!(details.duration, 213.746);
        assert_eq!(details.bitrate, 1_024_000, "the format's bitrate");
        assert_eq!((details.codec.as_str(), details.channels, details.sample_rate), ("flac", 2, 44100));
    }

    #[test]
    fn the_cover_art_stream_is_skipped() {
        let details = parse_probe(PROBE_OUTPUT.as_bytes()).unwrap();
        assert_ne!(details.codec, "mjpeg");
    }

    #[test]
    fn tags_are_read_whatever_their_case() {
        let details = parse_probe(PROBE_OUTPUT.as_bytes()).unwrap();
        assert_eq!(
            (details.title.as_str(), details.artist.as_str(), details.album.as_str()),
            ("A Song", "Somebody", "A Record")
        );
    }

    #[test]
    fn the_stream_bitrate_is_the_fallback() {
        let report = r#"{"streams":[{"codec_type":"audio","bit_rate":"320000"}],"format":{"duration":"10"}}"#;
        assert_eq!(parse_probe(report.as_bytes()).unwrap().bitrate, 320_000);
    }

    #[test]
    fn output_that_is_not_a_report_does_not_parse() {
        assert!(parse_probe(b"not json").is_err());
    }
}
