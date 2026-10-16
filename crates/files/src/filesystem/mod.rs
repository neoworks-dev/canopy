//! Reading the filesystem for the UI: directory listings, the sidebar's places
//! and devices, and the file preview. Entry types and sorting live in
//! `canopy-model`.

mod capacity;
mod devices;
mod path;
mod places;
mod preview;
mod reader;

pub use devices::{Device, devices};
pub use path::{is_hidden, parent_path, resolve_path};
pub use places::{Place, places};
pub use preview::{Preview, read_preview};
pub use reader::read_directory;

pub(crate) use devices::unescape_mount_field;
