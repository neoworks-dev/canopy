//! Reads and writes the archive formats a file manager is expected to open: zip
//! and the tar family. It knows about formats and members and nothing about
//! destinations, progress or conflicts, which belong to whoever is driving it.

mod create;
mod format;
mod member;
mod safe;
mod walk;

pub use create::create;
pub use format::{Format, base_name, format_of, is_archive, suggested_name};
pub use member::Member;
pub use safe::destination_for;
pub use walk::{list, walk};

#[cfg(test)]
mod tests;
