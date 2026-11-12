//! Types every part of Canopy shares: a directory entry, how a listing is
//! ordered, and where the application keeps its config files.

mod config;
mod entry;
mod mode;
mod owner;
mod sort;

pub use config::config_path;
pub use entry::{Entry, Listing, describe, entry_from_metadata, format_time};
pub use mode::mode_string;
pub use sort::{
    Order, SORT_BY_CREATED, SORT_BY_EXTENSION, SORT_BY_MODIFIED, SORT_BY_NAME, SORT_BY_NATURAL, SORT_BY_PERMISSIONS,
    SORT_BY_RANDOM, SORT_BY_SIZE, sort_entries,
};
