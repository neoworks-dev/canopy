package index

import (
	"database/sql"
	"fmt"
)

// defaultBatchSize trades memory for write throughput. A crawl of a home
// directory commits a few hundred transactions instead of hundreds of thousands.
const defaultBatchSize = 2000

// Batch accumulates crawler output and writes it in transactions. It is not safe
// for concurrent use; one crawl owns one batch.
type Batch struct {
	store          *Store
	session        ScanSession
	flushThreshold int
	pending        []FileRecord
}

func (store *Store) NewBatch(session ScanSession) *Batch {
	return &Batch{
		store:          store,
		session:        session,
		flushThreshold: defaultBatchSize,
		pending:        make([]FileRecord, 0, defaultBatchSize),
	}
}

// Add queues a record, flushing once the threshold is reached.
func (batch *Batch) Add(record FileRecord) error {
	batch.pending = append(batch.pending, record)
	if len(batch.pending) < batch.flushThreshold {
		return nil
	}
	return batch.Flush()
}

// Flush writes and clears everything queued. Calling it on an empty batch is a
// no-op, so callers can flush unconditionally at the end of a scan.
func (batch *Batch) Flush() error {
	if len(batch.pending) == 0 {
		return nil
	}
	if err := batch.writePending(); err != nil {
		return err
	}
	batch.pending = batch.pending[:0]
	return nil
}

func (batch *Batch) writePending() error {
	transaction, err := batch.store.database.Begin()
	if err != nil {
		return err
	}
	defer transaction.Rollback()

	statement, err := transaction.Prepare(upsertFileStatement)
	if err != nil {
		return fmt.Errorf("prepare upsert: %w", err)
	}
	defer statement.Close()

	if err := batch.executePending(statement); err != nil {
		return err
	}
	return transaction.Commit()
}

func (batch *Batch) executePending(statement *sql.Stmt) error {
	for _, record := range batch.pending {
		_, err := statement.Exec(upsertArguments(batch.session, record)...)
		if err != nil {
			return fmt.Errorf("upsert %s: %w", record.Path, err)
		}
	}
	return nil
}
