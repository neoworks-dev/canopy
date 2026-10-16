//! The file index: crawler, SQLite store with full-text search, and the
//! filesystem watcher that keeps it current.

mod batch;
mod crawler;
mod error;
mod exclusions;
mod indexer;
mod schema;
mod search;
mod store;
mod watcher;

pub use batch::Batch;
pub use crawler::{CrawlHooks, Crawler, ScanReport};
pub use error::{IndexError, IndexResult};
pub use exclusions::Exclusions;
pub use indexer::{Indexer, IndexerOptions, Status};
pub use search::{DEFAULT_SEARCH_LIMIT, MAXIMUM_SEARCH_LIMIT, Query, SearchOutcome};
pub use store::{FileRecord, RootRecord, ScanSession, Store};
pub use watcher::Watcher;
