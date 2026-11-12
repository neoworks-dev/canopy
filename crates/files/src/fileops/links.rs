//! Creating symlinks and hard links to existing entries.

use std::path::{Component, Path, PathBuf};

use super::names::suggest_name;
use crate::error::{Error, Result};

/// What a new link is and how it names its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    /// A symlink holding the target's absolute path.
    Absolute,
    /// A symlink holding the target's path relative to the link's directory,
    /// so it survives moving both together.
    Relative,
    Hard,
}

impl LinkKind {
    pub fn from_name(name: &str) -> Result<LinkKind> {
        match name {
            "absolute" => Ok(LinkKind::Absolute),
            "relative" => Ok(LinkKind::Relative),
            "hard" => Ok(LinkKind::Hard),
            other => Err(Error::message(format!("unknown link kind {other:?}"))),
        }
    }
}

/// Links each source into `directory` under the source's name, or the next
/// free variant of it, and answers with the paths created. Stops at the first
/// failure; links already made stay.
pub fn create_links(sources: &[String], directory: &str, kind: LinkKind) -> Result<Vec<String>> {
    let directory = std::fs::canonicalize(directory)?;
    let mut created = Vec::with_capacity(sources.len());
    for source in sources {
        let link_path = create_link(Path::new(source), &directory, kind)?;
        created.push(link_path.to_string_lossy().into_owned());
    }
    Ok(created)
}

fn create_link(source: &Path, directory: &Path, kind: LinkKind) -> Result<PathBuf> {
    let source = absolute_source(source)?;
    let Some(source_name) = source.file_name() else {
        return Err(Error::message(format!("{} has no name to link under", source.display())));
    };
    let link_path = directory.join(suggest_name(directory, &source_name.to_string_lossy()));
    match kind {
        LinkKind::Absolute => std::os::unix::fs::symlink(&source, &link_path)?,
        LinkKind::Relative => std::os::unix::fs::symlink(relative_path(directory, &source), &link_path)?,
        LinkKind::Hard => std::fs::hard_link(&source, &link_path)?,
    }
    Ok(link_path)
}

/// Resolves the source's parent but not the source itself, so linking to a
/// symlink links to that symlink rather than to what it points at. Sources
/// are expected to be absolute.
fn absolute_source(source: &Path) -> Result<PathBuf> {
    std::fs::symlink_metadata(source)?;
    let (Some(parent), Some(name)) = (source.parent(), source.file_name()) else {
        return Ok(std::fs::canonicalize(source)?);
    };
    Ok(std::fs::canonicalize(parent)?.join(name))
}

/// The path from `directory` to `target`, both absolute and free of `.` and `..`.
fn relative_path(directory: &Path, target: &Path) -> PathBuf {
    let directory_parts: Vec<Component> = directory.components().collect();
    let target_parts: Vec<Component> = target.components().collect();
    let shared = directory_parts.iter().zip(&target_parts).take_while(|(first, second)| first == second).count();
    let mut relative = PathBuf::new();
    for _ in shared..directory_parts.len() {
        relative.push("..");
    }
    for part in &target_parts[shared..] {
        relative.push(part);
    }
    relative
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn text(path: &Path) -> String {
        path.to_str().unwrap().to_string()
    }

    struct Fixture {
        _root: tempfile::TempDir,
        source: PathBuf,
        links: PathBuf,
    }

    /// `root/data/report.txt` and an empty `root/links/`, with `root` resolved
    /// so paths compare equal to what the links hold.
    fn fixture() -> Fixture {
        let root = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(root.path()).unwrap();
        fs::create_dir(base.join("data")).unwrap();
        fs::create_dir(base.join("links")).unwrap();
        let source = base.join("data").join("report.txt");
        fs::write(&source, "contents").unwrap();
        Fixture { _root: root, source, links: base.join("links") }
    }

    #[test]
    fn an_absolute_link_holds_the_full_path() {
        let fixture = fixture();

        let created = create_links(&[text(&fixture.source)], &text(&fixture.links), LinkKind::Absolute).unwrap();

        assert_eq!(created, [text(&fixture.links.join("report.txt"))]);
        assert_eq!(fs::read_link(&created[0]).unwrap(), fixture.source);
        assert_eq!(fs::read_to_string(&created[0]).unwrap(), "contents");
    }

    #[test]
    fn a_relative_link_holds_the_path_from_its_directory() {
        let fixture = fixture();

        let created = create_links(&[text(&fixture.source)], &text(&fixture.links), LinkKind::Relative).unwrap();

        assert_eq!(fs::read_link(&created[0]).unwrap(), Path::new("../data/report.txt"));
        assert_eq!(fs::read_to_string(&created[0]).unwrap(), "contents");
    }

    #[test]
    fn a_hard_link_shares_the_file() {
        use std::os::unix::fs::MetadataExt;
        let fixture = fixture();

        let created = create_links(&[text(&fixture.source)], &text(&fixture.links), LinkKind::Hard).unwrap();

        let link_metadata = fs::symlink_metadata(&created[0]).unwrap();
        assert!(!link_metadata.is_symlink());
        assert_eq!(link_metadata.ino(), fs::metadata(&fixture.source).unwrap().ino());
    }

    #[test]
    fn a_taken_name_gets_a_free_variant() {
        let fixture = fixture();
        fs::write(fixture.links.join("report.txt"), "other").unwrap();

        let created = create_links(&[text(&fixture.source)], &text(&fixture.links), LinkKind::Absolute).unwrap();

        assert_eq!(created, [text(&fixture.links.join("report (1).txt"))]);
    }

    #[test]
    fn linking_into_the_source_directory_does_not_clobber_the_source() {
        let fixture = fixture();
        let data = fixture.source.parent().unwrap();

        let created = create_links(&[text(&fixture.source)], &text(data), LinkKind::Relative).unwrap();

        assert_eq!(fs::read_link(&created[0]).unwrap(), Path::new("report.txt"));
        assert!(!fs::symlink_metadata(&fixture.source).unwrap().is_symlink());
    }

    #[test]
    fn a_missing_source_is_an_error() {
        let fixture = fixture();
        let missing = fixture.links.join("nothing");

        assert!(create_links(&[text(&missing)], &text(&fixture.links), LinkKind::Absolute).is_err());
    }

    #[test]
    fn an_unknown_kind_is_an_error() {
        assert!(LinkKind::from_name("soft").is_err());
        assert_eq!(LinkKind::from_name("hard").unwrap(), LinkKind::Hard);
    }

    #[test]
    fn relative_paths_climb_to_the_shared_ancestor() {
        assert_eq!(relative_path(Path::new("/a/b/c"), Path::new("/a/d/e")), Path::new("../../d/e"));
        assert_eq!(relative_path(Path::new("/a"), Path::new("/a/b")), Path::new("b"));
    }
}
