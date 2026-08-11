package thumbnail

import (
	"bytes"
	"context"
	"errors"
	"image"
	"image/png"
	"os"
	"os/exec"
	"strconv"

	_ "image/gif"
	_ "image/jpeg"
)

// The edge length the shared cache calls "large". Tiles are drawn well below
// it, so this leaves room for a denser grid without a second rendering pass.
const thumbnailSize = 256

var errNoRenderer = errors.New("no renderer for this file type")

// renderer turns one file into a PNG no larger than size on its longest side.
type renderer func(ctx context.Context, absolutePath string, size int) ([]byte, error)

// Extensions the standard library decodes on its own. Everything else needs a
// helper program, which may not be installed.
var nativeImageExtensions = map[string]bool{
	".png":  true,
	".jpg":  true,
	".jpeg": true,
	".gif":  true,
}

var decodableImageExtensions = map[string]bool{
	".webp": true,
	".avif": true,
	".bmp":  true,
	".ico":  true,
	".tif":  true,
	".tiff": true,
}

var videoExtensions = map[string]bool{
	".mp4":  true,
	".mkv":  true,
	".webm": true,
	".mov":  true,
	".avi":  true,
	".m4v":  true,
	".mpg":  true,
	".mpeg": true,
	".wmv":  true,
	".flv":  true,
}

func rendererFor(extension string) renderer {
	if nativeImageExtensions[extension] {
		return renderNativeImage
	}
	if decodableImageExtensions[extension] {
		return renderImageWithFFmpeg
	}
	if videoExtensions[extension] {
		return renderVideoFrame
	}
	if extension == ".pdf" {
		return renderFirstPDFPage
	}
	return nil
}

// CanRender reports whether a file of this type has a thumbnail worth asking
// for, so the frontend can draw an icon without a round trip.
func CanRender(extension string) bool {
	return rendererFor(extension) != nil
}

func renderNativeImage(_ context.Context, absolutePath string, size int) ([]byte, error) {
	file, err := os.Open(absolutePath)
	if err != nil {
		return nil, err
	}
	defer file.Close()
	decoded, _, err := image.Decode(file)
	if err != nil {
		return nil, err
	}
	return encodePNG(scaleToFit(decoded, size))
}

func encodePNG(scaled image.Image) ([]byte, error) {
	buffer := bytes.Buffer{}
	if err := png.Encode(&buffer, scaled); err != nil {
		return nil, err
	}
	return buffer.Bytes(), nil
}

// ffmpeg reads the formats the standard library does not, so an image it can
// decode is scaled and re-encoded in one pass.
func renderImageWithFFmpeg(ctx context.Context, absolutePath string, size int) ([]byte, error) {
	return runRenderer(ctx, "ffmpeg", ffmpegArguments(absolutePath, size)...)
}

// A frame from inside the file rather than the first one, which on most videos
// is a black fade-in.
func renderVideoFrame(ctx context.Context, absolutePath string, size int) ([]byte, error) {
	frame, err := runRenderer(ctx, "ffmpegthumbnailer",
		"-i", absolutePath, "-o", "-", "-c", "png", "-s", strconv.Itoa(size), "-t", "20%")
	if err == nil {
		return frame, nil
	}
	return runRenderer(ctx, "ffmpeg", append([]string{"-ss", "3"}, ffmpegArguments(absolutePath, size)...)...)
}

func ffmpegArguments(absolutePath string, size int) []string {
	edge := strconv.Itoa(size)
	return []string{
		"-v", "error",
		"-i", absolutePath,
		"-vf", "scale=w='min(" + edge + ",iw)':h='min(" + edge + ",ih)':force_original_aspect_ratio=decrease",
		"-frames:v", "1",
		"-f", "image2",
		"-c:v", "png",
		"-",
	}
}

// pdftoppm writes to stdout only when no output root is given, and -singlefile
// keeps it from numbering the page onto the end of one.
func renderFirstPDFPage(ctx context.Context, absolutePath string, size int) ([]byte, error) {
	return runRenderer(ctx, "pdftoppm",
		"-png", "-f", "1", "-singlefile", "-scale-to", strconv.Itoa(size), absolutePath)
}

// A helper that is not installed is a missing feature rather than a failure, so
// it comes back as the same "cannot render this" every other refusal does.
func runRenderer(ctx context.Context, program string, arguments ...string) ([]byte, error) {
	if _, err := exec.LookPath(program); err != nil {
		return nil, errNoRenderer
	}
	output := bytes.Buffer{}
	command := exec.CommandContext(ctx, program, arguments...)
	command.Stdout = &output
	if err := command.Run(); err != nil {
		return nil, err
	}
	if output.Len() == 0 {
		return nil, errNoRenderer
	}
	return output.Bytes(), nil
}
