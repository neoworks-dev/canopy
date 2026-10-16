//! Packing sources into an archive and unpacking an archive into a folder.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use super::copy::{abandon_target, write_contents};
use super::names::{exists, free_name, suggest_name};
use super::operation::Operation;
use crate::archive::{self, Member};
use crate::error::{Error, Result};
use crate::paths;

/// The name an archive of several entries takes, since no single one of them
/// is the thing being packed.
const BUNDLE_NAME: &str = "Archive";

impl Operation {
    /// Unpacks one archive into the destination folder, creating it if it is
    /// not there yet: extracting into a folder of its own is what keeps an
    /// archive of loose files from scattering them over the folder beside it.
    pub(super) fn run_extract(&self, source: &Path) -> Result<()> {
        self.wait_while_paused()?;
        self.prepare_destination()?;
        let mut unpacked = HashSet::new();
        archive::walk(source, |member, contents| self.unpack(member, contents, &mut unpacked))
    }

    /// Makes the folder to unpack into, remembering it as the one thing to
    /// remove if the extraction is taken back.
    fn prepare_destination(&self) -> Result<()> {
        if exists(&self.destination) {
            return Ok(());
        }
        make_directories(&self.destination)?;
        self.remember_created(&self.destination);
        Ok(())
    }

    /// Writes one member. A member the user chose to skip leaves the rest of
    /// the archive alone, which is what makes the conflict prompt worth
    /// answering per entry rather than for the archive as a whole.
    fn unpack(&self, member: &Member, contents: &mut dyn Read, unpacked: &mut HashSet<String>) -> Result<()> {
        self.wait_while_paused()?;
        self.set_current(Path::new(&member.name));
        match self.unpack_member(member, contents, unpacked) {
            Err(Error::Skipped) => Ok(()),
            outcome => outcome,
        }
    }

    fn unpack_member(&self, member: &Member, contents: &mut dyn Read, unpacked: &mut HashSet<String>) -> Result<()> {
        let target = archive::destination_for(&self.destination, &member.name)?;
        self.remember_top_level(member, unpacked);
        if member.is_dir {
            return make_directories(&target);
        }
        self.unpack_file(member, contents, &target)
    }

    fn unpack_file(&self, member: &Member, contents: &mut dyn Read, target: &Path) -> Result<()> {
        make_directories(&paths::directory_of(target))?;
        let resolved = self.resolve_target(Path::new(&member.name), target)?;
        self.write_unpacked(&resolved, contents)
    }

    fn write_unpacked(&self, target: &Path, contents: &mut dyn Read) -> Result<()> {
        let mut file = OpenOptions::new().write(true).create(true).truncate(true).mode(0o644).open(target)?;
        if let Err(error) = write_contents(&mut file, contents, self) {
            return Err(abandon_target(file, target, error));
        }
        Ok(())
    }

    /// Records what the archive adds to a folder that was already there, which
    /// is what an undo of the extraction removes. Only the topmost name of each
    /// member is worth keeping: everything below it goes with it.
    fn remember_top_level(&self, member: &Member, unpacked: &mut HashSet<String>) {
        let top = member.name.split('/').next().unwrap_or_default();
        if top.is_empty() || !unpacked.insert(top.to_string()) {
            return;
        }
        if self.created_destination() {
            return;
        }
        self.remember_created(&self.destination.join(top));
    }

    /// Packs every source into one archive, which is why it runs over the whole
    /// batch rather than once per source like the other kinds.
    pub(super) fn run_compress(&self) {
        let target = match self.archive_target() {
            Ok(target) => target,
            Err(error) => {
                self.record_outcome(0, &self.destination, Err(error));
                return;
            }
        };
        let outcome = archive::create(&target, &self.sources, |name, written| self.report_packed(name, written));
        self.finish_compression(&target, outcome);
    }

    fn finish_compression(&self, target: &Path, outcome: Result<()>) {
        let last_index = self.sources.len().saturating_sub(1);
        if outcome.is_ok() {
            self.remember_created(target);
        } else {
            let _ = fs::remove_file(target);
        }
        self.record_outcome(last_index, target, outcome);
    }

    fn report_packed(&self, name: &str, written: i64) -> Result<()> {
        self.wait_while_paused()?;
        self.set_current(Path::new(name));
        self.add_bytes(written);
        Ok(())
    }

    /// Where the new archive lands: named after the one entry being packed, or
    /// after nothing in particular when there are several. A name already taken
    /// gains a counter rather than failing the operation.
    fn archive_target(&self) -> Result<PathBuf> {
        let name = suggest_name(&self.destination, &archive_name_for(&self.sources));
        free_name(&self.destination, &name)
    }
}

fn archive_name_for(sources: &[PathBuf]) -> String {
    if sources.len() == 1 {
        return archive::suggested_name(&paths::base_name(&sources[0]));
    }
    archive::suggested_name(BUNDLE_NAME)
}

fn make_directories(path: &Path) -> Result<()> {
    fs::DirBuilder::new().recursive(true).mode(0o755).create(path)?;
    Ok(())
}

/// Where an archive unpacks when nobody chose a folder: one named after the
/// archive, beside it, under a name nothing else has taken.
pub fn extraction_folder_for(archive_path: &Path) -> PathBuf {
    let directory = paths::directory_of(archive_path);
    let base_name = archive::base_name(&paths::base_name(archive_path));
    directory.join(suggest_name(&directory, &base_name))
}
