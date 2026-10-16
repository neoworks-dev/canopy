//! Canopy's filesystem services: listing and previewing files, thumbnails,
//! archives, the operation engine (copy, move, trash, undo), folder
//! measurement and duplicate search.
//!
//! Each module corresponds to one area of the explorer:
//!
//! - [`filesystem`]: directory listings, places, devices and file previews
//! - [`fileops`]: the operation engine, undo/redo, name checks and the trash
//! - [`archive`]: reading and writing zip and tar archives
//! - [`thumbnail`]: preview images and the shared thumbnail cache
//! - [`audio`]: sound file details and waveforms
//! - [`highlight`]: syntax highlighting into coloured runs
//! - [`diskusage`]: measuring what a folder holds
//! - [`duplicates`]: finding files with identical contents
//!
//! Long-running work (operations, measurements, duplicate searches) reports
//! through a callback the caller supplies, so any front end can forward the
//! events. Every event has a `name()` and a `payload()` that serialises to the
//! JSON the UI expects.

pub mod archive;
pub mod audio;
pub mod diskusage;
pub mod duplicates;
pub mod fileops;
pub mod filesystem;
pub mod highlight;
pub mod thumbnail;

mod error;
mod hex;
mod paths;
mod process;
mod url_path;

pub use error::{Error, Result};
