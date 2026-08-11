package audio

import (
	"math"
	"testing"
)

const probeOutput = `{
  "streams": [
    {"codec_type": "video", "codec_name": "mjpeg"},
    {"codec_type": "audio", "codec_name": "flac", "channels": 2, "sample_rate": "44100", "bit_rate": "900000"}
  ],
  "format": {
    "duration": "213.746000",
    "bit_rate": "1024000",
    "tags": {"TITLE": "A Song", "ARTIST": "Somebody", "album": "A Record"}
  }
}`

func TestParseProbeReadsTheFormatAndTheAudioStream(t *testing.T) {
	details, err := parseProbe([]byte(probeOutput))
	if err != nil {
		t.Fatalf("parseProbe: %v", err)
	}
	if details.Duration != 213.746 {
		t.Fatalf("duration = %v, want 213.746", details.Duration)
	}
	if details.Bitrate != 1024000 {
		t.Fatalf("bitrate = %d, want the format's", details.Bitrate)
	}
	if details.Codec != "flac" || details.Channels != 2 || details.SampleRate != 44100 {
		t.Fatalf("stream read as %q/%d/%d, want the audio stream's", details.Codec, details.Channels, details.SampleRate)
	}
}

// The cover art comes first in the stream list, and describes a picture rather
// than the sound.
func TestParseProbeSkipsTheCoverArtStream(t *testing.T) {
	details, _ := parseProbe([]byte(probeOutput))
	if details.Codec == "mjpeg" {
		t.Fatal("the cover art stream was read as the audio stream")
	}
}

func TestParseProbeReadsTagsWhateverTheirCase(t *testing.T) {
	details, _ := parseProbe([]byte(probeOutput))
	if details.Title != "A Song" || details.Artist != "Somebody" || details.Album != "A Record" {
		t.Fatalf("tags read as %q/%q/%q", details.Title, details.Artist, details.Album)
	}
}

func TestParseProbeFallsBackToTheStreamBitrate(t *testing.T) {
	report := `{"streams":[{"codec_type":"audio","bit_rate":"320000"}],"format":{"duration":"10"}}`
	details, _ := parseProbe([]byte(report))
	if details.Bitrate != 320000 {
		t.Fatalf("bitrate = %d, want the stream's when the format carries none", details.Bitrate)
	}
}

func TestParseProbeRejectsWhatIsNotAReport(t *testing.T) {
	_, err := parseProbe([]byte("not json"))
	if err == nil {
		t.Fatal("output that is not a report must not parse")
	}
}

func TestPeaksTakeTheLoudestSampleOfEachBucket(t *testing.T) {
	samples := []int16{0, math.MaxInt16, 0, 0, math.MaxInt16 / 2, 0}
	found := peaks(samples, 2)
	assertClose(t, found[0], 1)
	assertClose(t, found[1], 0.5)
}

func TestPeaksReadANegativeSampleAsLoudAsAPositiveOne(t *testing.T) {
	found := peaks([]int16{-math.MaxInt16, 0}, 1)
	assertClose(t, found[0], 1)
}

func TestPeaksFillEveryBucketEvenWhenTheSamplesDoNotDivide(t *testing.T) {
	found := peaks([]int16{1, 2, 3, 4, 5}, 3)
	if len(found) != 3 {
		t.Fatalf("peaks = %v, want one per bucket", found)
	}
}

// A bucket that rounds to no samples at all must still be a peak, or the
// waveform would come back shorter than it was asked for.
func TestPeaksAreOnePerBucketWithFewerSamplesThanBuckets(t *testing.T) {
	found := peaks([]int16{math.MaxInt16}, 4)
	if len(found) != 4 {
		t.Fatalf("peaks = %v, want four", found)
	}
}

func TestPeaksOfNothingAreNothing(t *testing.T) {
	if len(peaks(nil, 8)) != 0 {
		t.Fatal("no samples must produce no peaks")
	}
}

func TestSamplesReadAsLittleEndianSignedPairs(t *testing.T) {
	samples := samplesOf([]byte{0x00, 0x80, 0xff, 0x7f})
	if samples[0] != math.MinInt16 || samples[1] != math.MaxInt16 {
		t.Fatalf("samples = %v, want the extremes", samples)
	}
}

func TestSampleRateFallsWithTheLengthOfTheTrack(t *testing.T) {
	short := sampleRateFor(10)
	long := sampleRateFor(3600)
	if short != highestSampleRate {
		t.Fatalf("rate for a short track = %d, want the highest", short)
	}
	if long != lowestSampleRate {
		t.Fatalf("rate for a long track = %d, want the lowest", long)
	}
}

func TestSampleRateOfAnUnknownLengthIsTheLowest(t *testing.T) {
	if sampleRateFor(0) != lowestSampleRate {
		t.Fatal("a track of no known length must decode at the cheapest rate")
	}
}

func TestSupportsReadsTheExtensionWhateverItsCase(t *testing.T) {
	if !Supports("/music/A Song.FLAC") {
		t.Fatal("an uppercase extension names the same type")
	}
	if Supports("/music/notes.txt") {
		t.Fatal("a text file is not a sound file")
	}
}

func assertClose(t *testing.T, found float64, want float64) {
	t.Helper()
	if math.Abs(found-want) > 0.001 {
		t.Fatalf("peak = %v, want %v", found, want)
	}
}
