//! Renders preview images for the files a listing shows and files them in the
//! shared thumbnail cache every desktop file manager reads.
//!
//! Rendering is best effort throughout: a file type nothing here can draw, a
//! helper program that is not installed and a file that fails to decode all
//! come back as an empty thumbnail, because none of them is worth putting an
//! error in front of somebody browsing a folder.

mod cache;
mod png_text;
mod render;

use std::fs::Metadata;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use crate::filesystem::resolve_path;
use render::{Renderer, THUMBNAIL_SIZE};

/// What a details pane can show without the image being the largest thing that
/// crosses the bridge in a session.
const PREVIEW_SIZE: u32 = 1024;

/// One preview image, ready to be used as an image source. An empty `data_url`
/// means there is no preview for this file, which is not a failure.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnail {
    pub path: String,
    pub data_url: String,
}

/// Renders thumbnails into one cache directory.
#[derive(Debug, Clone, Default)]
pub struct Thumbnailer {
    /// The cache directory to use; `None` means the user's own, which is
    /// `$XDG_CACHE_HOME/thumbnails/large`.
    cache_directory: Option<PathBuf>,
}

impl Thumbnailer {
    /// A thumbnailer that files images in the user's shared thumbnail cache.
    pub fn new() -> Thumbnailer {
        Thumbnailer { cache_directory: None }
    }

    /// A thumbnailer that files images in `directory` instead.
    pub fn with_cache_directory(directory: impl Into<PathBuf>) -> Thumbnailer {
        Thumbnailer { cache_directory: Some(directory.into()) }
    }

    /// The preview image for one file, from the shared cache when it holds a
    /// current one and by rendering it otherwise.
    pub fn render(&self, absolute_path: &Path) -> Thumbnail {
        let path = absolute_path.to_string_lossy().into_owned();
        let empty = Thumbnail { path: path.clone(), data_url: String::new() };
        let Some(renderer) = Renderer::for_path(absolute_path) else {
            return empty;
        };
        let Ok(metadata) = std::fs::metadata(absolute_path) else {
            return empty;
        };
        if metadata.is_dir() {
            return empty;
        }
        Thumbnail { path, data_url: self.data_url_for(absolute_path, &metadata, renderer) }
    }

    fn data_url_for(&self, absolute_path: &Path, metadata: &Metadata, renderer: Renderer) -> String {
        let uri = cache::source_uri(absolute_path);
        let modified_at = cache::modified_stamp(metadata);
        let Some(cache_file) = cache::cache_path(self.cache_directory.as_deref(), &uri) else {
            return String::new();
        };
        if let Some(cached) = cache::read_cached(&cache_file, &uri, &modified_at) {
            return encode_data_url(&cached);
        }
        let rendered = render_and_cache(absolute_path, &cache_file, &uri, &modified_at, renderer);
        match rendered {
            Some(image) => encode_data_url(&image),
            None => String::new(),
        }
    }
}

/// Draws one file at the size a details pane shows, which is large enough that
/// it is not worth keeping in the shared cache: it is asked for once per
/// selection rather than once per tile. Empty when nothing can draw the file.
pub fn render_preview(absolute_path: &Path) -> String {
    let Some(renderer) = Renderer::for_path(absolute_path) else {
        return String::new();
    };
    match render::render_within_limits(absolute_path, renderer, PREVIEW_SIZE) {
        Some(image) => encode_data_url(&image),
        None => String::new(),
    }
}

/// The preview image for one file, using the user's shared thumbnail cache.
pub fn render(absolute_path: &Path) -> Thumbnail {
    Thumbnailer::new().render(absolute_path)
}

/// The thumbnail for a path as the UI names it (`~` and relative paths are
/// resolved). A path that cannot be resolved comes back with an empty image
/// rather than an error, so a listing full of plain files reports nothing to
/// handle.
pub fn thumbnail_for(requested_path: &str) -> Thumbnail {
    match resolve_path(requested_path) {
        Ok(absolute_path) => render(&absolute_path),
        Err(_) => Thumbnail { path: requested_path.to_string(), data_url: String::new() },
    }
}

/// Whether `render` could produce an image for a path, which lets a caller skip
/// the request entirely for plain files.
pub fn supports(path: &Path) -> bool {
    Renderer::for_path(path).is_some()
}

fn render_and_cache(
    absolute_path: &Path,
    cache_file: &Path,
    uri: &str,
    modified_at: &str,
    renderer: Renderer,
) -> Option<Vec<u8>> {
    let image = render::render_within_limits(absolute_path, renderer, THUMBNAIL_SIZE)?;
    let annotated = png_text::with_thumbnail_metadata(&image, uri, modified_at).ok()?;
    cache::write_cached(cache_file, &annotated);
    Some(annotated)
}

fn encode_data_url(image: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(image))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{ImageFormat, Rgba, RgbaImage};

    use super::*;

    fn sample_png(width: u32, height: u32) -> Vec<u8> {
        let mut sample = RgbaImage::new(width, height);
        for x in 0..width {
            sample.put_pixel(x, 0, Rgba([x as u8, 128, 64, 255]));
        }
        let mut encoded = Cursor::new(Vec::new());
        sample.write_to(&mut encoded, ImageFormat::Png).unwrap();
        encoded.into_inner()
    }

    fn write_png(directory: &Path, width: u32, height: u32) -> PathBuf {
        let path = directory.join("image.png");
        std::fs::write(&path, sample_png(width, height)).unwrap();
        path
    }

    fn set_modified(path: &Path, seconds: i64) {
        let stamp = filetime::FileTime::from_unix_time(seconds, 0);
        filetime::set_file_times(path, stamp, stamp).unwrap();
    }

    fn cached_file_count(cache: &Path) -> usize {
        std::fs::read_dir(cache).map(|entries| entries.count()).unwrap_or(0)
    }

    fn decode_data_url(data_url: &str) -> image::DynamicImage {
        let (_, encoded) = data_url.split_once(',').expect("a data URL");
        let bytes = STANDARD.decode(encoded).unwrap();
        image::load_from_memory(&bytes).unwrap()
    }

    #[test]
    fn rendering_produces_a_data_url_and_caches_it() {
        let cache = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let path = write_png(source.path(), 400, 200);

        let rendered = Thumbnailer::with_cache_directory(cache.path()).render(&path);

        assert!(rendered.data_url.starts_with("data:image/png;base64,"));
        assert_eq!(cached_file_count(cache.path()), 1);
    }

    #[test]
    fn a_cached_thumbnail_is_used_again() {
        let cache = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let thumbnailer = Thumbnailer::with_cache_directory(cache.path());
        let path = write_png(source.path(), 400, 200);
        set_modified(&path, 1_700_000_000);
        let first = thumbnailer.render(&path);

        std::fs::write(&path, sample_png(40, 20)).unwrap();
        set_modified(&path, 1_700_000_000);
        let second = thumbnailer.render(&path);

        assert_eq!(second.data_url, first.data_url);
    }

    #[test]
    fn a_cached_thumbnail_of_an_older_version_is_ignored() {
        let cache = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let thumbnailer = Thumbnailer::with_cache_directory(cache.path());
        let path = write_png(source.path(), 400, 200);
        set_modified(&path, 1_700_000_000);
        let first = thumbnailer.render(&path);

        std::fs::write(&path, sample_png(40, 20)).unwrap();
        set_modified(&path, 1_700_000_600);
        let second = thumbnailer.render(&path);

        assert_ne!(second.data_url, first.data_url);
    }

    #[test]
    fn thumbnail_for_names_the_requested_path_even_without_a_preview() {
        let found = thumbnail_for("/definitely/not/here.png");
        assert_eq!((found.path.as_str(), found.data_url.as_str()), ("/definitely/not/here.png", ""));
    }

    #[test]
    fn files_with_no_preview_are_refused() {
        let cache = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let path = source.path().join("notes.txt");
        std::fs::write(&path, "hello").unwrap();

        assert!(Thumbnailer::with_cache_directory(cache.path()).render(&path).data_url.is_empty());
        assert!(!supports(&path), "supports rules the file out before a render is asked for");
    }

    #[test]
    fn supports_covers_the_renderable_types() {
        for path in ["/photos/a.JPG", "/clips/b.mkv", "/papers/c.pdf", "/music/d.flac"] {
            assert!(supports(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn the_longest_side_is_capped() {
        let cache = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let path = write_png(source.path(), 1000, 500);

        let rendered = decode_data_url(&Thumbnailer::with_cache_directory(cache.path()).render(&path).data_url);

        assert_eq!((rendered.width(), rendered.height()), (THUMBNAIL_SIZE, THUMBNAIL_SIZE / 2));
    }

    #[test]
    fn a_small_image_is_not_enlarged() {
        let cache = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let path = write_png(source.path(), 40, 20);

        let rendered = decode_data_url(&Thumbnailer::with_cache_directory(cache.path()).render(&path).data_url);

        assert_eq!((rendered.width(), rendered.height()), (40, 20));
    }

    #[test]
    fn the_cached_file_records_its_source() {
        let cache = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let path = write_png(source.path(), 40, 20);
        set_modified(&path, 1_700_000_000);

        Thumbnailer::with_cache_directory(cache.path()).render(&path);

        let uri = cache::source_uri(&path);
        let cache_file = cache::cache_path(Some(cache.path()), &uri).unwrap();
        let stored = std::fs::read(cache_file).unwrap();
        let (stored_uri, stored_modified_at) = png_text::thumbnail_metadata(&stored);
        assert_eq!((stored_uri, stored_modified_at), (uri, "1700000000".to_string()));
    }

    #[test]
    fn a_pdf_is_drawn_when_pdftoppm_is_installed() {
        if std::process::Command::new("pdftoppm").arg("-v").output().is_err() {
            return;
        }
        let cache = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let path = source.path().join("paper.pdf");
        std::fs::write(&path, SAMPLE_PDF).unwrap();

        let rendered = Thumbnailer::with_cache_directory(cache.path()).render(&path);

        assert!(rendered.data_url.starts_with("data:image/png;base64,"));
    }

    /// One blank page, which is all a renderer needs to be handed something it
    /// can draw.
    const SAMPLE_PDF: &str = "%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 100]>>endobj
trailer<</Root 1 0 R/Size 4>>
";
}
