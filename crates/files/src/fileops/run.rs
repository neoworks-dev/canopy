//! Running the sources of one operation: copy, move, trash and delete. Packing
//! and unpacking archives are in `archives.rs`.

use std::path::Path;

use super::copy::copy_resolved;
use super::operation::Operation;
use super::relocate::move_resolved;
use super::trash::remove_all;
use super::types::Kind;
use crate::error::{Error, Result};

impl Operation {
    /// Runs every source in turn, or packs them all into one archive.
    pub(super) fn run_all(&self) {
        self.measure();
        // Packing produces one archive out of every source together, so it is
        // the one kind that runs over the batch rather than over each source.
        if self.kind == Kind::Compress {
            self.run_compress();
            return;
        }
        for (index, source) in self.sources.iter().enumerate() {
            match self.run_one(source) {
                Err(Error::Cancelled) => return,
                outcome => self.record_outcome(index, source, outcome),
            }
        }
    }

    fn run_one(&self, source: &Path) -> Result<()> {
        match self.kind {
            Kind::Copy => self.run_copy(source),
            Kind::Move => self.run_move(source),
            Kind::Trash => self.run_trash(source),
            Kind::Delete => self.run_delete(source),
            Kind::Extract => self.run_extract(source),
            Kind::Compress => Err(Error::UnknownRequest),
        }
    }

    fn target_in_destination(&self, source: &Path) -> std::path::PathBuf {
        self.destination.join(source.file_name().unwrap_or_default())
    }

    fn run_copy(&self, source: &Path) -> Result<()> {
        let target = self.resolve_target(source, &self.target_in_destination(source))?;
        copy_resolved(source, &target, self)?;
        self.remember_created(&target);
        Ok(())
    }

    fn run_move(&self, source: &Path) -> Result<()> {
        let target = self.resolve_target(source, &self.target_in_destination(source))?;
        move_resolved(source, &target, self)?;
        self.remember_moved(source, &target);
        Ok(())
    }

    fn run_trash(&self, source: &Path) -> Result<()> {
        self.wait_while_paused()?;
        self.set_current(source);
        let item = self.trash.trash(source)?;
        self.remember_trashed(item);
        Ok(())
    }

    fn run_delete(&self, source: &Path) -> Result<()> {
        self.wait_while_paused()?;
        self.set_current(source);
        remove_all(source)
    }
}
