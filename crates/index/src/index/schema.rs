use rusqlite::Connection;

use super::error::{IndexError, IndexResult};

const SCHEMA_VERSION: i64 = 1;

/// The version 1 schema. `fileSearch` is an external-content FTS5 table: it
/// stores only the tokenized name and reads everything else back from `files`
/// through the triggers below, which keeps the index compact.
const SCHEMA_STATEMENTS: &[&str] = &[
    "CREATE TABLE roots (
        rootId         INTEGER PRIMARY KEY,
        path           TEXT    NOT NULL UNIQUE,
        addedAtUnix    INTEGER NOT NULL,
        lastScanUnix   INTEGER NOT NULL DEFAULT 0,
        scanGeneration INTEGER NOT NULL DEFAULT 0
    )",
    "CREATE TABLE files (
        fileId         INTEGER PRIMARY KEY,
        rootId         INTEGER NOT NULL REFERENCES roots(rootId) ON DELETE CASCADE,
        path           TEXT    NOT NULL UNIQUE,
        name           TEXT    NOT NULL,
        parentPath     TEXT    NOT NULL,
        isDir          INTEGER NOT NULL,
        size           INTEGER NOT NULL,
        modTimeUnix    INTEGER NOT NULL,
        scanGeneration INTEGER NOT NULL
    )",
    "CREATE INDEX filesByRoot ON files(rootId, scanGeneration)",
    "CREATE INDEX filesByParent ON files(parentPath)",
    // Separators split "annual_report.tar.gz" into searchable words so a query
    // for "report" matches without needing substring search.
    "CREATE VIRTUAL TABLE fileSearch USING fts5(
        name,
        content='files',
        content_rowid='fileId',
        tokenize=\"unicode61 separators '_-.'\"
    )",
    "CREATE TRIGGER filesAfterInsert AFTER INSERT ON files BEGIN
        INSERT INTO fileSearch(rowid, name) VALUES (new.fileId, new.name);
    END",
    "CREATE TRIGGER filesAfterDelete AFTER DELETE ON files BEGIN
        INSERT INTO fileSearch(fileSearch, rowid, name) VALUES ('delete', old.fileId, old.name);
    END",
    "CREATE TRIGGER filesAfterUpdate AFTER UPDATE ON files BEGIN
        INSERT INTO fileSearch(fileSearch, rowid, name) VALUES ('delete', old.fileId, old.name);
        INSERT INTO fileSearch(rowid, name) VALUES (new.fileId, new.name);
    END",
];

/// Brings an empty or older database up to `SCHEMA_VERSION`. The index is
/// rebuildable from the filesystem, so a future migration is always free to
/// drop and recreate rather than transform.
pub fn migrate(database: &mut Connection) -> IndexResult<()> {
    let current_version = read_schema_version(database)?;
    if current_version >= SCHEMA_VERSION {
        return Ok(());
    }
    apply_schema(database)
}

fn read_schema_version(database: &Connection) -> IndexResult<i64> {
    database
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(IndexError::database("read schema version"))
}

fn apply_schema(database: &mut Connection) -> IndexResult<()> {
    let transaction = database.transaction().map_err(IndexError::database("apply schema"))?;
    for statement in SCHEMA_STATEMENTS {
        transaction.execute_batch(statement).map_err(IndexError::database("apply schema"))?;
    }
    transaction
        .execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))
        .map_err(IndexError::database("set schema version"))?;
    transaction.commit().map_err(IndexError::database("apply schema"))
}
