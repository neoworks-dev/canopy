use super::error::{IndexError, IndexResult};
use super::store::{FileRecord, ScanSession, Store, UPSERT_FILE_STATEMENT, upsert_parameters};

/// Trades memory for write throughput. A crawl of a home directory commits a
/// few hundred transactions instead of hundreds of thousands.
const DEFAULT_BATCH_SIZE: usize = 2000;

/// Accumulates crawler output and writes it in transactions. One crawl owns one
/// batch.
pub struct Batch<'store> {
    store: &'store Store,
    session: ScanSession,
    pending: Vec<FileRecord>,
}

impl<'store> Batch<'store> {
    pub(super) fn new(store: &'store Store, session: ScanSession) -> Self {
        Batch { store, session, pending: Vec::with_capacity(DEFAULT_BATCH_SIZE) }
    }

    /// Queues a record, flushing once the threshold is reached.
    pub fn add(&mut self, record: FileRecord) -> IndexResult<()> {
        self.pending.push(record);
        if self.pending.len() < DEFAULT_BATCH_SIZE {
            return Ok(());
        }
        self.flush()
    }

    /// Writes and clears everything queued. Calling it on an empty batch is a
    /// no-op, so callers can flush unconditionally at the end of a scan.
    pub fn flush(&mut self) -> IndexResult<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        self.write_pending()?;
        self.pending.clear();
        Ok(())
    }

    fn write_pending(&self) -> IndexResult<()> {
        let mut database = self.store.writer();
        let transaction = database.transaction().map_err(IndexError::database("begin batch"))?;
        {
            let mut statement =
                transaction.prepare_cached(UPSERT_FILE_STATEMENT).map_err(IndexError::database("prepare upsert"))?;
            for record in &self.pending {
                statement
                    .execute(upsert_parameters(&self.session, record))
                    .map_err(IndexError::database("upsert file"))?;
            }
        }
        transaction.commit().map_err(IndexError::database("commit batch"))
    }
}
