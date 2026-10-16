//! Turns one file into a PNG: images are decoded in-process, everything else
//! goes to a helper program (ffmpeg, ffmpegthumbnailer, pdftoppm).

use std::io::Cursor;
use std::path::Path;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use image::{DynamicImage, ImageFormat};

use crate::audio;
use crate::paths;
use crate::process::run_helper;

/// The edge length the shared cache calls "large". Tiles are drawn well below
/// it, so this leaves room for a denser grid without a second rendering pass.
pub(super) const THUMBNAIL_SIZE: u32 = 256;

/// Rendering a folder of video files would otherwise start one helper process
/// per visible tile.
const CONCURRENT_RENDERS: usize = 4;

/// Long enough for a helper to open a large video, short enough that a file it
/// cannot make sense of does not hold a slot for the rest of the session.
const RENDER_TIME_LIMIT: Duration = Duration::from_secs(20);

const IMAGE_EXTENSIONS: [&str; 10] =
    [".png", ".jpg", ".jpeg", ".gif", ".webp", ".avif", ".bmp", ".ico", ".tif", ".tiff"];

const VIDEO_EXTENSIONS: [&str; 10] = [".mp4", ".mkv", ".webm", ".mov", ".avi", ".m4v", ".mpg", ".mpeg", ".wmv", ".flv"];

/// How a file type is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Renderer {
    /// Decoded in-process; ffmpeg handles the formats the decoder does not know.
    Image,
    /// Cover art is a video stream to ffmpeg, so a sound file draws as an image.
    AudioCover,
    VideoFrame,
    PdfFirstPage,
}

impl Renderer {
    pub(super) fn for_path(path: &Path) -> Option<Renderer> {
        Renderer::for_extension(&paths::extension_of(path))
    }

    fn for_extension(extension: &str) -> Option<Renderer> {
        if IMAGE_EXTENSIONS.contains(&extension) {
            return Some(Renderer::Image);
        }
        if audio::supports_extension(extension) {
            return Some(Renderer::AudioCover);
        }
        if VIDEO_EXTENSIONS.contains(&extension) {
            return Some(Renderer::VideoFrame);
        }
        if extension == ".pdf" {
            return Some(Renderer::PdfFirstPage);
        }
        None
    }
}

/// Draws a file no larger than `size` on its longest side, waiting for a free
/// render slot first. Nothing comes back for a file that cannot be drawn.
pub(super) fn render_within_limits(absolute_path: &Path, renderer: Renderer, size: u32) -> Option<Vec<u8>> {
    let _slot = RENDER_SLOTS.acquire();
    match renderer {
        Renderer::Image => render_image(absolute_path, size),
        Renderer::AudioCover => render_with_ffmpeg(absolute_path, size, &[]),
        Renderer::VideoFrame => render_video_frame(absolute_path, size),
        Renderer::PdfFirstPage => render_first_pdf_page(absolute_path, size),
    }
}

fn render_image(absolute_path: &Path, size: u32) -> Option<Vec<u8>> {
    if let Some(decoded) = decode_and_fit(absolute_path, size) {
        return Some(decoded);
    }
    render_with_ffmpeg(absolute_path, size, &[])
}

fn decode_and_fit(absolute_path: &Path, size: u32) -> Option<Vec<u8>> {
    let image = image::open(absolute_path).ok()?;
    encode_png(&fit_within(image, size))
}

/// Shrinks an image to fit a square; an image that already fits stays as it is.
fn fit_within(image: DynamicImage, size: u32) -> DynamicImage {
    if image.width() <= size && image.height() <= size {
        return image;
    }
    image.thumbnail(size, size)
}

fn encode_png(image: &DynamicImage) -> Option<Vec<u8>> {
    let mut encoded = Cursor::new(Vec::new());
    image.write_to(&mut encoded, ImageFormat::Png).ok()?;
    Some(encoded.into_inner())
}

/// A frame from inside the file rather than the first one, which on most
/// videos is a black fade-in.
fn render_video_frame(absolute_path: &Path, size: u32) -> Option<Vec<u8>> {
    let thumbnailer_arguments = [
        "-i".to_string(),
        absolute_path.to_string_lossy().into_owned(),
        "-o".to_string(),
        "-".to_string(),
        "-c".to_string(),
        "png".to_string(),
        "-s".to_string(),
        size.to_string(),
        "-t".to_string(),
        "20%".to_string(),
    ];
    if let Some(frame) = run_renderer("ffmpegthumbnailer", &thumbnailer_arguments) {
        return Some(frame);
    }
    render_with_ffmpeg(absolute_path, size, &["-ss".to_string(), "3".to_string()])
}

fn render_with_ffmpeg(absolute_path: &Path, size: u32, leading_arguments: &[String]) -> Option<Vec<u8>> {
    let mut arguments = leading_arguments.to_vec();
    arguments.extend(ffmpeg_arguments(absolute_path, size));
    run_renderer("ffmpeg", &arguments)
}

fn ffmpeg_arguments(absolute_path: &Path, size: u32) -> Vec<String> {
    let scale = format!("scale=w='min({size},iw)':h='min({size},ih)':force_original_aspect_ratio=decrease");
    [
        "-v",
        "error",
        "-i",
        &absolute_path.to_string_lossy(),
        "-vf",
        &scale,
        "-frames:v",
        "1",
        "-f",
        "image2",
        "-c:v",
        "png",
        "-",
    ]
    .iter()
    .map(|argument| argument.to_string())
    .collect()
}

/// pdftoppm writes to stdout only when no output root is given, and
/// `-singlefile` keeps it from numbering the page onto the end of one.
fn render_first_pdf_page(absolute_path: &Path, size: u32) -> Option<Vec<u8>> {
    let arguments = [
        "-png".to_string(),
        "-f".to_string(),
        "1".to_string(),
        "-singlefile".to_string(),
        "-scale-to".to_string(),
        size.to_string(),
        absolute_path.to_string_lossy().into_owned(),
    ];
    run_renderer("pdftoppm", &arguments)
}

/// A helper that is not installed is a missing feature rather than a failure,
/// so it comes back as the same "cannot render this" every other refusal does.
fn run_renderer(program: &str, arguments: &[String]) -> Option<Vec<u8>> {
    let output = run_helper(program, arguments, RENDER_TIME_LIMIT).ok()?;
    if output.is_empty() {
        return None;
    }
    Some(output)
}

/// A counting semaphore: at most `CONCURRENT_RENDERS` renders run at once.
struct RenderSlots {
    free: Mutex<usize>,
    released: Condvar,
}

static RENDER_SLOTS: RenderSlots = RenderSlots { free: Mutex::new(CONCURRENT_RENDERS), released: Condvar::new() };

struct RenderSlot<'a> {
    slots: &'a RenderSlots,
}

impl RenderSlots {
    fn acquire(&self) -> RenderSlot<'_> {
        let mut free = self.free.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while *free == 0 {
            free = self.released.wait(free).unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        *free -= 1;
        RenderSlot { slots: self }
    }
}

impl Drop for RenderSlot<'_> {
    fn drop(&mut self) {
        let mut free = self.slots.free.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *free += 1;
        self.slots.released.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_choose_the_renderer() {
        assert_eq!(Renderer::for_extension(".jpg"), Some(Renderer::Image));
        assert_eq!(Renderer::for_extension(".flac"), Some(Renderer::AudioCover));
        assert_eq!(Renderer::for_extension(".mkv"), Some(Renderer::VideoFrame));
        assert_eq!(Renderer::for_extension(".pdf"), Some(Renderer::PdfFirstPage));
        assert_eq!(Renderer::for_extension(".txt"), None);
    }
}
