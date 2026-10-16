//! The one error type every module in this crate returns.
//!
//! The messages are what the UI shows, so they are written as sentences a
//! person can read.

use std::io;

/// Everything that can go wrong in this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Unwinds a running operation without recording a failure.
    #[error("operation cancelled")]
    Cancelled,
    /// Stops one source without failing the batch.
    #[error("item skipped")]
    Skipped,
    #[error("unknown operation")]
    UnknownRequest,
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
    #[error("a directory cannot be copied into itself")]
    IntoItself,
    #[error("name must not be empty")]
    EmptyName,
    #[error("name must not contain a path separator")]
    NameSeparator,
    #[error("name must not be \".\" or \"..\"")]
    ReservedName,
    #[error("an entry with this name already exists")]
    NameExists,
    #[error("no trash record for this path")]
    NotTrashed,
    #[error("this is not an archive the explorer can open")]
    UnsupportedFormat,
    #[error("this archive format can only be read")]
    CannotWriteFormat,
    #[error("this archive holds an entry that points outside the folder")]
    EscapingMember,
    #[error("only a folder can be measured")]
    NotAFolderToMeasure,
    #[error("unknown measurement")]
    UnknownMeasurement,
    #[error("only a folder can be searched")]
    NotAFolderToSearch,
    #[error("unknown search")]
    UnknownSearch,
    /// A failure described by a sentence of its own.
    #[error("{0}")]
    Message(String),
    #[error("{}", describe_io_error(.0))]
    Io(#[from] io::Error),
}

/// The result type used throughout this crate.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// A failure that has no variant of its own.
    pub fn message(text: impl Into<String>) -> Error {
        Error::Message(text.into())
    }

    /// Whether this is the error that unwinds a cancelled operation.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Error::Cancelled)
    }

    /// Whether this is the error that skips one source without failing the batch.
    pub fn is_skipped(&self) -> bool {
        matches!(self, Error::Skipped)
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(error: zip::result::ZipError) -> Error {
        Error::Message(error.to_string())
    }
}

/// An operating system error in the lowercase form people are used to
/// reading: "no such file or directory", without the "(os error 2)" suffix.
fn describe_io_error(error: &io::Error) -> String {
    let text = error.to_string();
    let text = match text.find(" (os error") {
        Some(position) => text[..position].to_string(),
        None => text,
    };
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_lowercase().collect::<String>() + characters.as_str(),
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_errors_read_like_system_messages() {
        let error = Error::from(io::Error::from_raw_os_error(2));
        assert_eq!(error.to_string(), "no such file or directory");
    }
}
