use std::io;

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    /// The requested path was never registered as a root.
    #[error("index: root not found")]
    RootNotFound,
    #[error("index: {0} is not a directory")]
    NotADirectory(String),
    #[error("index: scan cancelled")]
    Cancelled,
    #[error("{context}: {source}")]
    Database {
        context: &'static str,
        #[source]
        source: rusqlite::Error,
    },
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl IndexError {
    pub fn database(context: &'static str) -> impl FnOnce(rusqlite::Error) -> IndexError {
        move |source| IndexError::Database { context, source }
    }
}

pub type IndexResult<Value> = Result<Value, IndexError>;
