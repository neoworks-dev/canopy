package thumbnail

import (
	"bytes"
	"image"
	"image/color"
	"image/png"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestRenderProducesADataURLAndCachesIt(t *testing.T) {
	cache := t.TempDir()
	t.Setenv("XDG_CACHE_HOME", cache)
	source := writePNG(t, 400, 200)

	rendered := Render(source)

	if !strings.HasPrefix(rendered.DataURL, "data:image/png;base64,") {
		t.Fatalf("expected a PNG data URL, got %q", truncate(rendered.DataURL))
	}
	if countCachedThumbnails(t, cache) != 1 {
		t.Fatalf("expected the thumbnail to be filed in the shared cache")
	}
}

func TestRenderReusesACachedThumbnail(t *testing.T) {
	t.Setenv("XDG_CACHE_HOME", t.TempDir())
	source := writePNG(t, 400, 200)
	setModified(t, source, 1700000000)
	first := Render(source)

	writeOver(t, source, 40, 20)
	setModified(t, source, 1700000000)
	second := Render(source)

	if second.DataURL != first.DataURL {
		t.Fatalf("expected the cached thumbnail to be used again")
	}
}

func TestRenderIgnoresACachedThumbnailOfAnOlderVersion(t *testing.T) {
	t.Setenv("XDG_CACHE_HOME", t.TempDir())
	source := writePNG(t, 400, 200)
	setModified(t, source, 1700000000)
	first := Render(source)

	writeOver(t, source, 40, 20)
	setModified(t, source, 1700000600)
	second := Render(source)

	if second.DataURL == first.DataURL {
		t.Fatalf("expected a rewritten file to be rendered again")
	}
}

func TestRenderRefusesFilesWithNoPreview(t *testing.T) {
	t.Setenv("XDG_CACHE_HOME", t.TempDir())
	source := filepath.Join(t.TempDir(), "notes.txt")
	if err := os.WriteFile(source, []byte("hello"), 0o600); err != nil {
		t.Fatal(err)
	}

	if Render(source).DataURL != "" {
		t.Fatalf("expected no thumbnail for a text file")
	}
	if Supports(source) {
		t.Fatalf("expected Supports to rule the file out before a render is asked for")
	}
}

func TestSupportsCoversRenderableTypes(t *testing.T) {
	for _, path := range []string{"/photos/a.JPG", "/clips/b.mkv", "/papers/c.pdf"} {
		if !Supports(path) {
			t.Errorf("expected %q to be renderable", path)
		}
	}
}

func TestThumbnailMetadataSurvivesEncoding(t *testing.T) {
	encoded := encodeSample(t, 8, 8)

	annotated, err := withThumbnailMetadata(encoded, "file:///photos/a.png", "1700000000")
	if err != nil {
		t.Fatal(err)
	}

	uri, modifiedAt := thumbnailMetadata(annotated)
	if uri != "file:///photos/a.png" || modifiedAt != "1700000000" {
		t.Fatalf("got uri %q and mtime %q", uri, modifiedAt)
	}
	if _, err := png.Decode(bytes.NewReader(annotated)); err != nil {
		t.Fatalf("annotating broke the image: %v", err)
	}
}

func TestThumbnailMetadataRejectsSomethingThatIsNotAPNG(t *testing.T) {
	if _, err := withThumbnailMetadata([]byte("nowhere near a png"), "file:///a", "1"); err == nil {
		t.Fatalf("expected an error for a file that is not a PNG")
	}
}

func TestScaleToFitCapsTheLongestSide(t *testing.T) {
	scaled := scaleToFit(image.NewRGBA(image.Rect(0, 0, 1000, 500)), 256)

	if scaled.Bounds().Dx() != 256 || scaled.Bounds().Dy() != 128 {
		t.Fatalf("got %v", scaled.Bounds())
	}
}

func TestScaleToFitLeavesSmallImagesAlone(t *testing.T) {
	source := image.NewRGBA(image.Rect(0, 0, 32, 16))

	if scaleToFit(source, 256) != image.Image(source) {
		t.Fatalf("expected the same image back")
	}
}

func TestScaleToFitAveragesTheSourcePixels(t *testing.T) {
	source := image.NewRGBA(image.Rect(0, 0, 2, 2))
	source.Set(0, 0, color.RGBA{R: 255, A: 255})
	source.Set(1, 0, color.RGBA{R: 255, A: 255})
	source.Set(0, 1, color.RGBA{A: 255})
	source.Set(1, 1, color.RGBA{A: 255})

	scaled := downscale(source, 1, 1)

	red, _, _, _ := scaled.At(0, 0).RGBA()
	if eightBit(uint64(red)) != 127 {
		t.Fatalf("expected the halves to average, got %d", eightBit(uint64(red)))
	}
}

func writePNG(t *testing.T, width int, height int) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "image.png")
	writeOver(t, path, width, height)
	return path
}

func writeOver(t *testing.T, path string, width int, height int) {
	t.Helper()
	if err := os.WriteFile(path, encodeSample(t, width, height), 0o600); err != nil {
		t.Fatal(err)
	}
}

func setModified(t *testing.T, path string, seconds int64) {
	t.Helper()
	stamp := time.Unix(seconds, 0)
	if err := os.Chtimes(path, stamp, stamp); err != nil {
		t.Fatal(err)
	}
}

func encodeSample(t *testing.T, width int, height int) []byte {
	t.Helper()
	sample := image.NewRGBA(image.Rect(0, 0, width, height))
	for x := range width {
		sample.Set(x, 0, color.RGBA{R: uint8(x), G: 128, B: 64, A: 255})
	}
	encoded, err := encodePNG(sample)
	if err != nil {
		t.Fatal(err)
	}
	return encoded
}

func countCachedThumbnails(t *testing.T, cache string) int {
	t.Helper()
	entries, err := os.ReadDir(filepath.Join(cache, cacheDirectoryName, sizeDirectoryName))
	if err != nil {
		return 0
	}
	return len(entries)
}

func truncate(value string) string {
	if len(value) <= 48 {
		return value
	}
	return value[:48] + "…"
}
