//! The freedesktop.org trash: moving paths into the trash directory that serves
//! their filesystem, listing what is there, restoring and emptying it.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, NaiveDateTime, SecondsFormat, TimeZone};
use serde::{Deserialize, Serialize};

use super::device::{device_of, on_same_volume};
use super::mounts::volume_trash_locations;
use super::names::{exists, split_extension, unique_name};
use crate::error::{Error, Result};
use crate::paths;
use crate::url_path::{escape_path, unescape_path};

const FILES_DIRECTORY: &str = "files";
const INFO_DIRECTORY: &str = "info";
const INFO_SUFFIX: &str = ".trashinfo";
const DATE_LAYOUT: &str = "%Y-%m-%dT%H:%M:%S";
const MAX_NAME_ATTEMPTS: u32 = 10_000;
const STICKY_BIT: u32 = 0o1000;

/// What a deletion date reads as when the record has none.
const UNKNOWN_DATE: &str = "0001-01-01T00:00:00Z";

/// One entry in a freedesktop.org trash directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashedItem {
    pub name: String,
    pub trash_path: String,
    pub original_path: String,
    pub deleted_at: String,
    pub size: i64,
    pub is_dir: bool,
}

/// One trash directory. `top_directory` is `None` for the home trash, where the
/// spec records absolute original paths, and is the mount point for a
/// per-volume trash, where it records paths relative to that mount.
#[derive(Debug, Clone)]
pub(super) struct TrashLocation {
    pub(super) directory: PathBuf,
    pub(super) top_directory: Option<PathBuf>,
}

/// The trash of one user. `Trash::system()` follows `$XDG_DATA_HOME`, which is
/// what the application uses; tests give it a directory of their own.
#[derive(Debug, Clone, Default)]
pub struct Trash {
    data_home: Option<PathBuf>,
}

impl Trash {
    /// The user's own trash, found through the environment on every call.
    pub fn system() -> Trash {
        Trash { data_home: None }
    }

    /// A trash whose home directory is `data_home/Trash`.
    pub fn in_data_home(data_home: impl Into<PathBuf>) -> Trash {
        Trash { data_home: Some(data_home.into()) }
    }

    /// Moves a path into the trash directory that serves its filesystem,
    /// writing the `.trashinfo` record that makes it restorable by any
    /// compliant tool.
    pub fn trash(&self, path: &Path) -> Result<TrashedItem> {
        let absolute_path = paths::absolute(path)?;
        let location = self.location_for(&absolute_path)?;
        let (name, info_file) = reserve_trash_name(&location.directory, &paths::base_name(&absolute_path))?;
        complete_trashing(&absolute_path, &name, info_file, &location)
    }

    /// Puts a trashed item back where it came from, reporting the path it
    /// ended up at.
    pub fn restore(&self, trash_path: &Path) -> Result<String> {
        let info_path = trash_info_path_for(trash_path);
        let (original_path, _) = read_trash_info(&info_path, top_directory_for_trash_path(trash_path).as_deref())?;
        let target = restore_target(&original_path)?;
        fs::rename(trash_path, &target)?;
        fs::remove_file(&info_path)?;
        Ok(target.to_string_lossy().into_owned())
    }

    /// Reads every trash directory reachable from this machine, newest deletion
    /// first.
    pub fn list(&self) -> Result<Vec<TrashedItem>> {
        let mut items = Vec::new();
        for location in self.locations()? {
            items.extend(items_in(&location));
        }
        items.sort_by(|first, second| second.deleted_at.cmp(&first.deleted_at));
        Ok(items)
    }

    /// Discards every trashed item permanently.
    pub fn empty(&self) -> Result<()> {
        for location in self.locations()? {
            clear_directory_contents(&location.directory.join(FILES_DIRECTORY))?;
            clear_directory_contents(&location.directory.join(INFO_DIRECTORY))?;
        }
        Ok(())
    }

    fn home_trash_directory(&self) -> Result<PathBuf> {
        if let Some(data_home) = &self.data_home {
            return Ok(data_home.join("Trash"));
        }
        if let Some(data_home) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(data_home).join("Trash"));
        }
        let home_directory = std::env::home_dir().ok_or_else(|| Error::message("$HOME is not defined"))?;
        Ok(home_directory.join(".local").join("share").join("Trash"))
    }

    fn locations(&self) -> Result<Vec<TrashLocation>> {
        let home_trash = self.home_trash_directory()?;
        prepare_trash_directory(&home_trash)?;
        let mut locations = vec![TrashLocation { directory: home_trash, top_directory: None }];
        locations.extend(volume_trash_locations());
        Ok(locations)
    }

    fn location_for(&self, path: &Path) -> Result<TrashLocation> {
        let home_trash = self.home_trash_directory()?;
        prepare_trash_directory(&home_trash)?;
        if on_same_volume(&paths::directory_of(path), &home_trash)? {
            return Ok(TrashLocation { directory: home_trash, top_directory: None });
        }
        volume_trash_location_for(path)
    }
}

/// Moves a path into the user's trash.
pub fn trash(path: &Path) -> Result<TrashedItem> {
    Trash::system().trash(path)
}

/// Puts a trashed item back where it came from.
pub fn restore(trash_path: &Path) -> Result<String> {
    Trash::system().restore(trash_path)
}

/// Lists everything in the user's trash, newest deletion first.
pub fn list_trashed() -> Result<Vec<TrashedItem>> {
    Trash::system().list()
}

/// Discards everything in the user's trash permanently.
pub fn empty_trash() -> Result<()> {
    Trash::system().empty()
}

fn complete_trashing(
    absolute_path: &Path,
    name: &str,
    info_file: File,
    location: &TrashLocation,
) -> Result<TrashedItem> {
    let deleted_at = Local::now();
    let recorded_path = recorded_original_path(absolute_path, location.top_directory.as_deref());
    if let Err(error) = write_trash_info(info_file, &recorded_path, deleted_at) {
        return Err(abandon_trash_record(&location.directory, name, error));
    }
    let trash_path = location.directory.join(FILES_DIRECTORY).join(name);
    if let Err(error) = fs::rename(absolute_path, &trash_path) {
        return Err(abandon_trash_record(&location.directory, name, error.into()));
    }
    Ok(describe_trashed(&trash_path, absolute_path, Some(deleted_at)))
}

/// Drops the reserved `.trashinfo` file so a failed trashing leaves no orphan
/// record behind.
fn abandon_trash_record(trash_directory: &Path, name: &str, cause: Error) -> Error {
    let _ = fs::remove_file(trash_directory.join(INFO_DIRECTORY).join(format!("{name}{INFO_SUFFIX}")));
    cause
}

fn describe_trashed(trash_path: &Path, original_path: &Path, deleted_at: Option<DateTime<Local>>) -> TrashedItem {
    let mut item = TrashedItem {
        name: paths::base_name(original_path),
        trash_path: trash_path.to_string_lossy().into_owned(),
        original_path: original_path.to_string_lossy().into_owned(),
        deleted_at: format_deletion_date(deleted_at),
        size: 0,
        is_dir: false,
    };
    if let Ok(metadata) = fs::symlink_metadata(trash_path) {
        item.size = metadata.len() as i64;
        item.is_dir = metadata.is_dir();
    }
    item
}

fn format_deletion_date(deleted_at: Option<DateTime<Local>>) -> String {
    match deleted_at {
        Some(time) => time.to_rfc3339_opts(SecondsFormat::Secs, true),
        None => UNKNOWN_DATE.to_string(),
    }
}

/// Where a trashed item goes back to. Something else may have taken the
/// original name since the item was trashed, so this never overwrites.
fn restore_target(original_path: &Path) -> Result<PathBuf> {
    let parent = paths::directory_of(original_path);
    fs::DirBuilder::new().recursive(true).mode(0o755).create(&parent)?;
    if !exists(original_path) {
        return Ok(original_path.to_path_buf());
    }
    let name = unique_name(&parent, &paths::base_name(original_path))?;
    Ok(parent.join(name))
}

fn clear_directory_contents(directory: &Path) -> Result<()> {
    for child in fs::read_dir(directory)? {
        remove_all(&child?.path())?;
    }
    Ok(())
}

/// Removes a file, a symlink, or a directory with everything in it. A path that
/// is already gone is not an error.
pub(super) fn remove_all(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.is_dir() {
        return Ok(fs::remove_dir_all(path)?);
    }
    Ok(fs::remove_file(path)?)
}

fn items_in(location: &TrashLocation) -> Vec<TrashedItem> {
    let files_directory = location.directory.join(FILES_DIRECTORY);
    let Ok(children) = sorted_children(&files_directory) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for trash_path in children {
        if let Some(item) = trashed_item_at(&trash_path, location) {
            items.push(item);
        }
    }
    items
}

fn sorted_children(directory: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut children = Vec::new();
    for child in fs::read_dir(directory)? {
        children.push(child?.path());
    }
    children.sort();
    Ok(children)
}

/// Anything without a readable record is not restorable and does not belong in
/// the list.
fn trashed_item_at(trash_path: &Path, location: &TrashLocation) -> Option<TrashedItem> {
    let info_path = trash_info_path_for(trash_path);
    let (original_path, deleted_at) = read_trash_info(&info_path, location.top_directory.as_deref()).ok()?;
    Some(describe_trashed(trash_path, &original_path, deleted_at))
}

fn reserve_trash_name(trash_directory: &Path, name: &str) -> Result<(String, File)> {
    let info_directory = trash_directory.join(INFO_DIRECTORY);
    for counter in 0..MAX_NAME_ATTEMPTS {
        let candidate = trash_candidate_name(name, counter);
        let info_path = info_directory.join(format!("{candidate}{INFO_SUFFIX}"));
        match OpenOptions::new().write(true).create_new(true).mode(0o600).open(info_path) {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(Error::message(format!("no free trash name for {name:?}")))
}

fn trash_candidate_name(name: &str, counter: u32) -> String {
    if counter == 0 {
        return name.to_string();
    }
    let (stem, extension) = split_extension(name);
    format!("{stem} ({counter}){extension}")
}

fn write_trash_info(mut info_file: File, recorded_path: &Path, deleted_at: DateTime<Local>) -> Result<()> {
    let contents = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        escape_path(&recorded_path.to_string_lossy()),
        deleted_at.format(DATE_LAYOUT)
    );
    info_file.write_all(contents.as_bytes())?;
    Ok(())
}

fn read_trash_info(info_path: &Path, top_directory: Option<&Path>) -> Result<(PathBuf, Option<DateTime<Local>>)> {
    let Ok(contents) = fs::read_to_string(info_path) else {
        return Err(Error::NotTrashed);
    };
    let (recorded_path, deleted_at) = parse_trash_info(&contents);
    if recorded_path.is_empty() {
        return Err(Error::NotTrashed);
    }
    Ok((absolute_original_path(&recorded_path, top_directory), deleted_at))
}

fn parse_trash_info(contents: &str) -> (String, Option<DateTime<Local>>) {
    let mut recorded_path = String::new();
    let mut deleted_at = None;
    for line in contents.split('\n') {
        if let Some(value) = line.strip_prefix("Path=") {
            recorded_path = value.trim().to_string();
        }
        if let Some(value) = line.strip_prefix("DeletionDate=") {
            deleted_at = parse_deletion_date(value.trim()).or(deleted_at);
        }
    }
    let unescaped = unescape_path(&recorded_path).unwrap_or(recorded_path);
    (unescaped, deleted_at)
}

fn parse_deletion_date(raw: &str) -> Option<DateTime<Local>> {
    let naive = NaiveDateTime::parse_from_str(raw, DATE_LAYOUT).ok()?;
    Local.from_local_datetime(&naive).earliest()
}

/// Absolute in the home trash and relative to the mount point in a per-volume
/// trash, so the volume stays movable.
fn recorded_original_path(absolute_path: &Path, top_directory: Option<&Path>) -> PathBuf {
    let Some(top_directory) = top_directory else {
        return absolute_path.to_path_buf();
    };
    match absolute_path.strip_prefix(top_directory) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => absolute_path.to_path_buf(),
    }
}

fn absolute_original_path(recorded_path: &str, top_directory: Option<&Path>) -> PathBuf {
    let recorded = Path::new(recorded_path);
    if recorded.is_absolute() {
        return recorded.to_path_buf();
    }
    let top_directory = top_directory.unwrap_or(Path::new(""));
    paths::clean(&top_directory.join(recorded))
}

fn trash_info_path_for(trash_path: &Path) -> PathBuf {
    let trash_directory = paths::directory_of(&paths::directory_of(trash_path));
    let name = paths::base_name(trash_path);
    trash_directory.join(INFO_DIRECTORY).join(format!("{name}{INFO_SUFFIX}"))
}

/// Recovers the mount point a per-volume trash belongs to. The home trash sits
/// below the data directory and yields nothing, which is what
/// `absolute_original_path` expects for absolute records.
fn top_directory_for_trash_path(trash_path: &Path) -> Option<PathBuf> {
    let trash_directory = paths::directory_of(&paths::directory_of(trash_path));
    let name = paths::base_name(&trash_directory);
    if name.starts_with(".Trash-") {
        return Some(paths::directory_of(&trash_directory));
    }
    if name == current_user_id().to_string() {
        return Some(paths::directory_of(&paths::directory_of(&trash_directory)));
    }
    None
}

fn current_user_id() -> u32 {
    rustix::process::getuid().as_raw()
}

fn volume_trash_location_for(path: &Path) -> Result<TrashLocation> {
    let top_directory = top_directory_of(path)?;
    let directory = volume_trash_directory(&top_directory);
    prepare_trash_directory(&directory)?;
    Ok(TrashLocation { directory, top_directory: Some(top_directory) })
}

/// The administrator-provided `.Trash/$uid` when it is present and sticky, and
/// otherwise the `.Trash-$uid` the spec lets us create.
pub(super) fn volume_trash_directory(top_directory: &Path) -> PathBuf {
    let shared = top_directory.join(".Trash");
    if is_sticky_directory(&shared) {
        return shared.join(current_user_id().to_string());
    }
    top_directory.join(format!(".Trash-{}", current_user_id()))
}

fn is_sticky_directory(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(metadata) => metadata.is_dir() && metadata.mode() & STICKY_BIT != 0,
        Err(_) => false,
    }
}

/// Climbs until the filesystem changes, which is the mount point the volume
/// trash belongs to.
fn top_directory_of(path: &Path) -> Result<PathBuf> {
    let mut current = paths::directory_of(path);
    let device = device_of(&current)?;
    loop {
        let parent = paths::directory_of(&current);
        if parent == current || !is_same_device(&parent, device) {
            return Ok(current);
        }
        current = parent;
    }
}

fn is_same_device(path: &Path, device: u64) -> bool {
    match device_of(path) {
        Ok(parent_device) => parent_device == device,
        Err(_) => false,
    }
}

fn prepare_trash_directory(trash_directory: &Path) -> Result<()> {
    for child in [FILES_DIRECTORY, INFO_DIRECTORY] {
        fs::DirBuilder::new().recursive(true).mode(0o700).create(trash_directory.join(child))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        data_home: tempfile::TempDir,
        trash: Trash,
    }

    fn fixture() -> Fixture {
        let data_home = tempfile::tempdir().unwrap();
        let trash = Trash::in_data_home(data_home.path());
        Fixture { data_home, trash }
    }

    fn write(path: &Path, contents: &str) {
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn trashing_writes_a_spec_compliant_record() {
        let fixture = fixture();
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("holiday photos.txt");
        write(&original, "body");

        let item = fixture.trash.trash(&original).unwrap();

        assert!(!original.exists(), "the file leaves its directory");
        assert_eq!(fs::read_to_string(&item.trash_path).unwrap(), "body");
        let record_path = fixture.data_home.path().join("Trash/info/holiday photos.txt.trashinfo");
        let record = fs::read_to_string(record_path).unwrap();
        assert!(record.starts_with("[Trash Info]\n"), "{record}");
        let escaped_original = original.to_str().unwrap().replace(' ', "%20");
        assert!(record.contains(&format!("Path={escaped_original}")), "{record}");
        assert!(record.contains("DeletionDate="), "{record}");
    }

    #[test]
    fn colliding_names_get_their_own_record() {
        let fixture = fixture();
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        write(&first.path().join("note.txt"), "first");
        write(&second.path().join("note.txt"), "second");

        let first_item = fixture.trash.trash(&first.path().join("note.txt")).unwrap();
        let second_item = fixture.trash.trash(&second.path().join("note.txt")).unwrap();

        assert_ne!(first_item.trash_path, second_item.trash_path);
        assert_eq!(fs::read_to_string(&second_item.trash_path).unwrap(), "second");
    }

    #[test]
    fn restoring_puts_the_item_back_and_drops_its_record() {
        let fixture = fixture();
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("note.txt");
        write(&original, "body");
        let item = fixture.trash.trash(&original).unwrap();

        let restored = fixture.trash.restore(Path::new(&item.trash_path)).unwrap();

        assert_eq!(restored, original.to_str().unwrap());
        assert!(!fixture.data_home.path().join("Trash/info/note.txt.trashinfo").exists());
    }

    #[test]
    fn restoring_does_not_overwrite_a_new_occupant() {
        let fixture = fixture();
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("note.txt");
        write(&original, "old");
        let item = fixture.trash.trash(&original).unwrap();
        write(&original, "written since");

        let restored = fixture.trash.restore(Path::new(&item.trash_path)).unwrap();

        assert_ne!(restored, original.to_str().unwrap());
        assert_eq!(fs::read_to_string(&original).unwrap(), "written since");
        assert_eq!(fs::read_to_string(&restored).unwrap(), "old");
    }

    #[test]
    fn the_list_reports_what_was_trashed() {
        let fixture = fixture();
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("note.txt");
        write(&original, "body");
        fixture.trash.trash(&original).unwrap();

        let items = fixture.trash.list().unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].original_path, original.to_str().unwrap());
        assert_eq!(items[0].name, "note.txt");
        assert_eq!(items[0].size, 4);
        assert!(!items[0].is_dir);
    }

    #[test]
    fn emptying_discards_everything() {
        let fixture = fixture();
        let directory = tempfile::tempdir().unwrap();
        write(&directory.path().join("note.txt"), "body");
        fixture.trash.trash(&directory.path().join("note.txt")).unwrap();

        fixture.trash.empty().unwrap();

        assert!(fixture.trash.list().unwrap().is_empty());
    }

    #[test]
    fn the_record_gives_the_path_and_date() {
        let (path, deleted_at) =
            parse_trash_info("[Trash Info]\nPath=/home/user/a%20b.txt\nDeletionDate=2026-08-10T22:15:00\n");
        assert_eq!(path, "/home/user/a b.txt");
        let deleted_at = deleted_at.expect("a deletion date");
        assert_eq!(deleted_at.format(DATE_LAYOUT).to_string(), "2026-08-10T22:15:00");
    }

    #[test]
    fn the_top_directory_stops_at_a_mount_boundary() {
        let directory = tempfile::tempdir().unwrap();
        let top_directory = top_directory_of(&directory.path().join("note.txt")).unwrap();
        assert!(directory.path().starts_with(&top_directory));
        assert!(top_directory.is_dir());
    }

    #[test]
    fn a_volume_without_a_sticky_trash_uses_the_owned_one() {
        let mount_point = tempfile::tempdir().unwrap();
        let directory = volume_trash_directory(mount_point.path());
        assert_eq!(directory.parent().unwrap(), mount_point.path());
        assert!(directory.file_name().unwrap().to_string_lossy().starts_with(".Trash-"));
    }

    #[test]
    fn a_relative_record_is_read_against_the_mount_point() {
        assert_eq!(
            absolute_original_path("docs/a.txt", Some(Path::new("/run/media/stick"))),
            PathBuf::from("/run/media/stick/docs/a.txt")
        );
        assert_eq!(absolute_original_path("/home/a.txt", None), PathBuf::from("/home/a.txt"));
    }
}
