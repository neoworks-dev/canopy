// Package audio reads what a details pane can show about a sound file: what the
// container says about it, and the shape of the sound itself.
//
// Everything here is best effort. ffprobe or ffmpeg being absent, or a file they
// cannot decode, comes back as "nothing to show" rather than an error, because
// the pane still has the file's own metadata to fall back on.
package audio

import (
	"bytes"
	"context"
	"encoding/binary"
	"encoding/json"
	"math"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"time"
)

// Long enough to decode a long track, short enough that a file ffmpeg cannot
// make sense of does not hold the pane open.
const readTimeout = 20 * time.Second

// How many peaks the waveform is drawn from. Wider than any details pane, so the
// drawing never has to invent detail it was not given.
const waveformBuckets = 480

// The decode is downsampled to about this many samples whatever the track's
// length, so a three minute song and a three hour recording cost the same.
const targetSampleCount = 200000

const (
	lowestSampleRate  = 200
	highestSampleRate = 8000
)

var audioExtensions = map[string]bool{
	".mp3":  true,
	".flac": true,
	".wav":  true,
	".ogg":  true,
	".oga":  true,
	".opus": true,
	".m4a":  true,
	".aac":  true,
	".wma":  true,
	".aiff": true,
	".aif":  true,
	".ape":  true,
	".wv":   true,
}

// Details is what one sound file reports about itself. A zero field is one the
// container did not carry.
type Details struct {
	Duration   float64 `json:"duration"`
	Bitrate    int     `json:"bitrate"`
	Channels   int     `json:"channels"`
	SampleRate int     `json:"sampleRate"`
	Codec      string  `json:"codec"`
	Title      string  `json:"title"`
	Artist     string  `json:"artist"`
	Album      string  `json:"album"`
	// Peaks between 0 and 1, in playback order; empty when nothing decoded.
	Waveform []float64 `json:"waveform"`
}

// Supports reports whether a path is one this package can say anything about.
func Supports(path string) bool {
	return SupportsExtension(strings.ToLower(filepath.Ext(path)))
}

// SupportsExtension is Supports for a caller that already has the extension,
// such as one picking a renderer by type.
func SupportsExtension(extension string) bool {
	return audioExtensions[extension]
}

// Read describes one sound file. The second result is false when there is
// nothing worth showing, which is what a missing ffprobe looks like.
func Read(absolutePath string) (Details, bool) {
	probed, err := runProbe(absolutePath)
	if err != nil {
		return Details{}, false
	}
	details, err := parseProbe(probed)
	if err != nil {
		return Details{}, false
	}
	details.Waveform = readWaveform(absolutePath, details.Duration)
	return details, true
}

func runProbe(absolutePath string) ([]byte, error) {
	return run("ffprobe",
		"-v", "error",
		"-print_format", "json",
		"-show_format",
		"-show_streams",
		absolutePath)
}

// probeReport is the part of ffprobe's JSON this package reads.
type probeReport struct {
	Streams []probeStream `json:"streams"`
	Format  probeFormat   `json:"format"`
}

type probeStream struct {
	CodecType  string `json:"codec_type"`
	CodecName  string `json:"codec_name"`
	Channels   int    `json:"channels"`
	SampleRate string `json:"sample_rate"`
	BitRate    string `json:"bit_rate"`
}

type probeFormat struct {
	Duration string            `json:"duration"`
	BitRate  string            `json:"bit_rate"`
	Tags     map[string]string `json:"tags"`
}

func parseProbe(probed []byte) (Details, error) {
	report := probeReport{}
	err := json.Unmarshal(probed, &report)
	if err != nil {
		return Details{}, err
	}
	details := detailsFromFormat(report.Format)
	return withStream(details, firstAudioStream(report.Streams)), nil
}

func detailsFromFormat(format probeFormat) Details {
	return Details{
		Duration: parseNumber(format.Duration),
		Bitrate:  int(parseNumber(format.BitRate)),
		Title:    tagValue(format.Tags, "title"),
		Artist:   tagValue(format.Tags, "artist", "album_artist", "performer"),
		Album:    tagValue(format.Tags, "album"),
	}
}

func withStream(details Details, stream probeStream) Details {
	details.Codec = stream.CodecName
	details.Channels = stream.Channels
	details.SampleRate = int(parseNumber(stream.SampleRate))
	if details.Bitrate == 0 {
		details.Bitrate = int(parseNumber(stream.BitRate))
	}
	return details
}

// A file with cover art carries a video stream as well, so the audio stream is
// picked by type rather than by position.
func firstAudioStream(streams []probeStream) probeStream {
	for _, stream := range streams {
		if stream.CodecType == "audio" {
			return stream
		}
	}
	return probeStream{}
}

// Tag names differ by container, and ffprobe lowercases none of them.
func tagValue(tags map[string]string, names ...string) string {
	for _, name := range names {
		found := matchingTag(tags, name)
		if found != "" {
			return found
		}
	}
	return ""
}

func matchingTag(tags map[string]string, name string) string {
	for key, value := range tags {
		if strings.EqualFold(key, name) {
			return value
		}
	}
	return ""
}

func parseNumber(value string) float64 {
	number, err := strconv.ParseFloat(value, 64)
	if err != nil {
		return 0
	}
	return number
}

// readWaveform decodes the track to mono at a rate chosen from its length, so
// the amount decoded stays about the same however long it runs.
func readWaveform(absolutePath string, duration float64) []float64 {
	decoded, err := run("ffmpeg",
		"-v", "error",
		"-i", absolutePath,
		"-ac", "1",
		"-ar", strconv.Itoa(sampleRateFor(duration)),
		"-f", "s16le",
		"-")
	if err != nil {
		return []float64{}
	}
	return peaks(samplesOf(decoded), waveformBuckets)
}

func sampleRateFor(duration float64) int {
	if duration <= 0 {
		return lowestSampleRate
	}
	rate := int(targetSampleCount / duration)
	return clampRate(rate)
}

func clampRate(rate int) int {
	if rate < lowestSampleRate {
		return lowestSampleRate
	}
	if rate > highestSampleRate {
		return highestSampleRate
	}
	return rate
}

func samplesOf(decoded []byte) []int16 {
	samples := make([]int16, len(decoded)/2)
	for index := range samples {
		samples[index] = int16(binary.LittleEndian.Uint16(decoded[index*2:]))
	}
	return samples
}

// peaks reduces the decoded samples to one loudest value per bucket, which is
// what makes a waveform read as the shape of the sound rather than as noise.
func peaks(samples []int16, buckets int) []float64 {
	if len(samples) == 0 || buckets <= 0 {
		return []float64{}
	}
	found := make([]float64, buckets)
	for bucket := range found {
		found[bucket] = loudest(samples, bucket, buckets)
	}
	return found
}

func loudest(samples []int16, bucket int, buckets int) float64 {
	highest := 0.0
	for _, sample := range bucketOf(samples, bucket, buckets) {
		highest = math.Max(highest, amplitude(sample))
	}
	return highest
}

// The last bucket takes the remainder, so no sample is left out by rounding.
func bucketOf(samples []int16, bucket int, buckets int) []int16 {
	start := bucket * len(samples) / buckets
	end := (bucket + 1) * len(samples) / buckets
	if bucket == buckets-1 {
		end = len(samples)
	}
	if start >= end {
		return nil
	}
	return samples[start:end]
}

func amplitude(sample int16) float64 {
	return math.Abs(float64(sample)) / math.MaxInt16
}

// A helper that is not installed is a missing feature rather than a failure.
func run(program string, arguments ...string) ([]byte, error) {
	_, err := exec.LookPath(program)
	if err != nil {
		return nil, err
	}
	ctx, cancel := context.WithTimeout(context.Background(), readTimeout)
	defer cancel()
	output := bytes.Buffer{}
	command := exec.CommandContext(ctx, program, arguments...)
	command.Stdout = &output
	err = command.Run()
	if err != nil {
		return nil, err
	}
	return output.Bytes(), nil
}
