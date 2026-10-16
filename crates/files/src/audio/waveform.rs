use std::path::Path;

use crate::audio::READ_TIME_LIMIT;
use crate::process::run_helper;

/// How many peaks the waveform is drawn from. Wider than any details pane, so
/// the drawing never has to invent detail it was not given.
const WAVEFORM_BUCKETS: usize = 480;

/// The decode is downsampled to about this many samples whatever the track's
/// length, so a three minute song and a three hour recording cost the same.
const TARGET_SAMPLE_COUNT: f64 = 200_000.0;

const LOWEST_SAMPLE_RATE: i64 = 200;
const HIGHEST_SAMPLE_RATE: i64 = 8000;

/// Decodes the track to mono at a rate chosen from its length, so the amount
/// decoded stays about the same however long it runs.
pub(super) fn read_waveform(absolute_path: &Path, duration: f64) -> Vec<f64> {
    let arguments = [
        "-v".to_string(),
        "error".to_string(),
        "-i".to_string(),
        absolute_path.to_string_lossy().into_owned(),
        "-ac".to_string(),
        "1".to_string(),
        "-ar".to_string(),
        sample_rate_for(duration).to_string(),
        "-f".to_string(),
        "s16le".to_string(),
        "-".to_string(),
    ];
    match run_helper("ffmpeg", &arguments, READ_TIME_LIMIT) {
        Ok(decoded) => peaks(&samples_of(&decoded), WAVEFORM_BUCKETS),
        Err(_) => Vec::new(),
    }
}

fn sample_rate_for(duration: f64) -> i64 {
    if duration <= 0.0 {
        return LOWEST_SAMPLE_RATE;
    }
    let rate = (TARGET_SAMPLE_COUNT / duration) as i64;
    rate.clamp(LOWEST_SAMPLE_RATE, HIGHEST_SAMPLE_RATE)
}

fn samples_of(decoded: &[u8]) -> Vec<i16> {
    let (pairs, _) = decoded.as_chunks::<2>();
    pairs.iter().map(|pair| i16::from_le_bytes(*pair)).collect()
}

/// Reduces the decoded samples to one loudest value per bucket, which is what
/// makes a waveform read as the shape of the sound rather than as noise.
fn peaks(samples: &[i16], buckets: usize) -> Vec<f64> {
    if samples.is_empty() || buckets == 0 {
        return Vec::new();
    }
    (0..buckets).map(|bucket| loudest(bucket_of(samples, bucket, buckets))).collect()
}

fn loudest(samples: &[i16]) -> f64 {
    let mut highest = 0.0_f64;
    for sample in samples {
        highest = highest.max(amplitude(*sample));
    }
    highest
}

/// The last bucket takes the remainder, so no sample is left out by rounding.
fn bucket_of(samples: &[i16], bucket: usize, buckets: usize) -> &[i16] {
    let start = bucket * samples.len() / buckets;
    let mut end = (bucket + 1) * samples.len() / buckets;
    if bucket == buckets - 1 {
        end = samples.len();
    }
    if start >= end {
        return &[];
    }
    &samples[start..end]
}

fn amplitude(sample: i16) -> f64 {
    (sample as f64).abs() / i16::MAX as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(found: f64, wanted: f64) {
        assert!((found - wanted).abs() < 0.001, "peak = {found}, want {wanted}");
    }

    #[test]
    fn each_bucket_takes_its_loudest_sample() {
        let found = peaks(&[0, i16::MAX, 0, 0, i16::MAX / 2, 0], 2);
        assert_close(found[0], 1.0);
        assert_close(found[1], 0.5);
    }

    #[test]
    fn a_negative_sample_is_as_loud_as_a_positive_one() {
        assert_close(peaks(&[-i16::MAX, 0], 1)[0], 1.0);
    }

    #[test]
    fn every_bucket_is_filled_when_the_samples_do_not_divide() {
        assert_eq!(peaks(&[1, 2, 3, 4, 5], 3).len(), 3);
    }

    #[test]
    fn there_is_one_peak_per_bucket_with_fewer_samples_than_buckets() {
        assert_eq!(peaks(&[i16::MAX], 4).len(), 4);
    }

    #[test]
    fn no_samples_make_no_peaks() {
        assert!(peaks(&[], 8).is_empty());
    }

    #[test]
    fn samples_are_little_endian_signed_pairs() {
        let samples = samples_of(&[0x00, 0x80, 0xff, 0x7f]);
        assert_eq!(samples, [i16::MIN, i16::MAX]);
    }

    #[test]
    fn the_rate_falls_with_the_length_of_the_track() {
        assert_eq!(sample_rate_for(10.0), HIGHEST_SAMPLE_RATE);
        assert_eq!(sample_rate_for(3600.0), LOWEST_SAMPLE_RATE);
    }

    #[test]
    fn an_unknown_length_decodes_at_the_lowest_rate() {
        assert_eq!(sample_rate_for(0.0), LOWEST_SAMPLE_RATE);
    }
}
