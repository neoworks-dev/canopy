use std::fs::{File, Metadata};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Datelike, Local, Timelike};
use flate2::write::GzEncoder;
use walkdir::WalkDir;

use super::format::{Format, format_of};
use crate::error::{Error, Result};

/// The writing half of a format. The two the explorer packs into differ in how
/// an entry is announced and in nothing else.
trait Packer {
    fn add(&mut self, name: &str, path: &Path, metadata: &Metadata) -> Result<()>;
    fn finish(self: Box<Self>) -> Result<()>;
}

/// Writes an archive holding the sources, keeping each source's own name as the
/// top of what it holds. `report` is called with every entry written, and an
/// error from it stops the run, which is how a caller cancels.
///
/// An entry that cannot be read is left out rather than failing the archive:
/// one unreadable file is a worse reason to have no archive at all.
pub fn create(archive_path: &Path, sources: &[PathBuf], mut report: impl FnMut(&str, i64) -> Result<()>) -> Result<()> {
    let mut packer = packer_for(archive_path)?;
    for source in sources {
        if let Err(error) = add_tree(packer.as_mut(), source, &mut report) {
            let _ = packer.finish();
            return Err(error);
        }
    }
    packer.finish()
}

fn add_tree(packer: &mut dyn Packer, source: &Path, report: &mut impl FnMut(&str, i64) -> Result<()>) -> Result<()> {
    let base = crate::paths::directory_of(source);
    let walker = WalkDir::new(source).follow_links(false).follow_root_links(false).sort_by_file_name();
    for entry in walker {
        let Ok(entry) = entry else {
            continue;
        };
        let Some((name, metadata)) = entry_to_pack(&base, &entry) else {
            continue;
        };
        packer.add(&name, entry.path(), &metadata)?;
        report(&name, metadata.len() as i64)?;
    }
    Ok(())
}

/// The name an entry takes inside the archive. Only files and directories go
/// in: a link or a device is not something to unpack again.
fn entry_to_pack(base: &Path, entry: &walkdir::DirEntry) -> Option<(String, Metadata)> {
    let file_type = entry.file_type();
    if !file_type.is_dir() && !file_type.is_file() {
        return None;
    }
    let metadata = entry.metadata().ok()?;
    let relative = entry.path().strip_prefix(base).ok()?;
    Some((relative.to_string_lossy().into_owned(), metadata))
}

fn packer_for(archive_path: &Path) -> Result<Box<dyn Packer>> {
    let format = format_of(&archive_path.to_string_lossy());
    let file = File::create(archive_path)?;
    match packer_of_format(file, format) {
        Ok(packer) => Ok(packer),
        Err(error) => {
            let _ = std::fs::remove_file(archive_path);
            Err(error)
        }
    }
}

fn packer_of_format(file: File, format: Format) -> Result<Box<dyn Packer>> {
    match format {
        Format::Zip => Ok(Box::new(ZipPacker { writer: zip::ZipWriter::new(file) })),
        Format::Tar => Ok(Box::new(TarPacker { builder: tar::Builder::new(TarSink::Plain(file)) })),
        Format::TarGzip => {
            let compressor = GzEncoder::new(file, flate2::Compression::default());
            Ok(Box::new(TarPacker { builder: tar::Builder::new(TarSink::Gzip(compressor)) }))
        }
        Format::TarBzip2 => Err(Error::CannotWriteFormat),
        Format::Unknown => Err(Error::UnsupportedFormat),
    }
}

struct ZipPacker {
    writer: zip::ZipWriter<File>,
}

impl Packer for ZipPacker {
    fn add(&mut self, name: &str, path: &Path, metadata: &Metadata) -> Result<()> {
        let options = zip_options(metadata);
        if metadata.is_dir() {
            self.writer.add_directory(name, options)?;
            return Ok(());
        }
        self.writer.start_file(name, options)?;
        let mut source = File::open(path)?;
        io::copy(&mut source, &mut self.writer)?;
        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<()> {
        self.writer.finish()?;
        Ok(())
    }
}

fn zip_options(metadata: &Metadata) -> zip::write::SimpleFileOptions {
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(metadata.permissions().mode() & 0o777)
        .large_file(metadata.len() > u64::from(u32::MAX));
    match zip_time(metadata) {
        Some(modified) => options.last_modified_time(modified),
        None => options,
    }
}

/// The modification time as the local calendar time a zip stores.
fn zip_time(metadata: &Metadata) -> Option<zip::DateTime> {
    let modified: SystemTime = metadata.modified().ok()?;
    let local: DateTime<Local> = modified.into();
    zip::DateTime::from_date_and_time(
        u16::try_from(local.year()).ok()?,
        local.month() as u8,
        local.day() as u8,
        local.hour() as u8,
        local.minute() as u8,
        local.second() as u8,
    )
    .ok()
}

/// Where a tar's bytes go: straight to the file, or through gzip first.
enum TarSink {
    Plain(File),
    Gzip(GzEncoder<File>),
}

impl Write for TarSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self {
            TarSink::Plain(file) => file.write(bytes),
            TarSink::Gzip(compressor) => compressor.write(bytes),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            TarSink::Plain(file) => file.flush(),
            TarSink::Gzip(compressor) => compressor.flush(),
        }
    }
}

struct TarPacker {
    builder: tar::Builder<TarSink>,
}

impl Packer for TarPacker {
    fn add(&mut self, name: &str, path: &Path, metadata: &Metadata) -> Result<()> {
        if metadata.is_dir() {
            self.builder.append_dir(name, path)?;
            return Ok(());
        }
        self.builder.append_path_with_name(path, name)?;
        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<()> {
        match self.builder.into_inner()? {
            TarSink::Plain(mut file) => file.flush()?,
            TarSink::Gzip(compressor) => {
                compressor.finish()?;
            }
        }
        Ok(())
    }
}
