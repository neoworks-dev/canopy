use std::fs::File;
use std::io::Read;
use std::path::Path;

use flate2::read::MultiGzDecoder;

use super::format::{Format, format_of};
use super::member::{Member, mode_text, tar_time_text, zip_time_text};
use crate::error::{Error, Result};
use crate::paths;

/// Hands each member of an archive to `visit`, in the order the archive holds
/// them. The reader is only good for the duration of the call: a tar is a
/// stream, so nothing can be read back once the walk has moved on.
///
/// A visit that returns an error stops the walk and that error is what `walk`
/// reports, which is how a caller cancels.
pub fn walk(archive_path: &Path, mut visit: impl FnMut(&Member, &mut dyn Read) -> Result<()>) -> Result<()> {
    let format = format_of(&archive_path.to_string_lossy());
    match format {
        Format::Zip => walk_zip(archive_path, &mut visit),
        Format::Unknown => Err(Error::UnsupportedFormat),
        _ => walk_tar(archive_path, format, &mut visit),
    }
}

/// Reports what an archive holds without reading any of it.
pub fn list(archive_path: &Path) -> Result<Vec<Member>> {
    let mut members = Vec::new();
    walk(archive_path, |member, _contents| {
        members.push(member.clone());
        Ok(())
    })?;
    Ok(members)
}

type Visit<'a> = &'a mut dyn FnMut(&Member, &mut dyn Read) -> Result<()>;

fn walk_zip(archive_path: &Path, visit: Visit) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(archive_path)?)?;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let member = zip_member(&file)?;
        if member.is_dir {
            visit(&member, &mut std::io::empty())?;
            continue;
        }
        visit(&member, &mut file)?;
    }
    Ok(())
}

fn zip_member(file: &zip::read::ZipFile<'_, File>) -> Result<Member> {
    let name = file.name()?.into_owned();
    let is_dir = file.is_dir() || name.ends_with('/');
    let permission_bits = match file.unix_mode() {
        Some(mode) => mode & 0o777,
        None if is_dir => 0o777,
        None => 0o666,
    };
    Ok(Member {
        name: member_name(&name),
        size: file.size() as i64,
        is_dir,
        mod_time: zip_time_text(file.last_modified()),
        mode: mode_text(is_dir, permission_bits),
    })
}

fn walk_tar(archive_path: &Path, format: Format, visit: Visit) -> Result<()> {
    let file = File::open(archive_path)?;
    match format {
        Format::TarGzip => visit_tar_entries(MultiGzDecoder::new(file), visit),
        Format::TarBzip2 => visit_tar_entries(bzip2::read::MultiBzDecoder::new(file), visit),
        _ => visit_tar_entries(file, visit),
    }
}

fn visit_tar_entries(stream: impl Read, visit: Visit) -> Result<()> {
    let mut archive = tar::Archive::new(stream);
    for entry in archive.entries()? {
        let mut entry = entry?;
        // Only files and directories are handed on. A tar can also hold links
        // and devices, which an extraction has no business recreating.
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            continue;
        }
        let member = tar_member(&entry)?;
        visit(&member, &mut entry)?;
    }
    Ok(())
}

fn tar_member(entry: &tar::Entry<'_, impl Read>) -> Result<Member> {
    let header = entry.header();
    let name = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
    let is_dir = header.entry_type().is_dir() || name.ends_with('/');
    Ok(Member {
        name: member_name(&name),
        size: header.size()? as i64,
        is_dir,
        mod_time: tar_time_text(header.mtime().ok()),
        mode: mode_text(is_dir, header.mode()? & 0o777),
    })
}

/// The name an archive holds a member under: no leading slash, and cleaned.
fn member_name(name: &str) -> String {
    let without_slash = name.strip_prefix('/').unwrap_or(name);
    paths::clean_slash_path(without_slash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_names_are_cleaned() {
        assert_eq!(member_name("project/nested/"), "project/nested");
        assert_eq!(member_name("/abs/file"), "abs/file");
        assert_eq!(member_name("./top.txt"), "top.txt");
    }
}
