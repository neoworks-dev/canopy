use std::fs::{self, OpenOptions};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::types::ToSql;
use rusqlite::{Connection, OptionalExtension, params};

use super::batch::Batch;
use super::error::{IndexError, IndexResult};
use super::schema::migrate;

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// One indexed filesystem entry.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileRecord {
    pub path: String,
    pub name: String,
    pub parent_path: String,
    pub is_dir: bool,
    pub size: i64,
    pub mod_time_unix: i64,
}

/// A registered root and how far its last scan got.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RootRecord {
    pub root_id: i64,
    pub path: String,
    pub file_count: i64,
    pub last_scan_unix: i64,
}

/// One pass over a root. Every row written during the pass carries
/// `generation`, so anything left at an older generation afterwards no longer
/// exists on disk and gets swept.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScanSession {
    pub root_id: i64,
    pub root_path: String,
    pub generation: i64,
}

/// Owns the on-disk index. Reads run on their own connection in WAL mode
/// alongside the single writer.
pub struct Store {
    writer: Mutex<Connection>,
    reader: Mutex<Connection>,
}

pub(super) const UPSERT_FILE_STATEMENT: &str = "
    INSERT INTO files (rootId, path, name, parentPath, isDir, size, modTimeUnix, scanGeneration)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
    ON CONFLICT(path) DO UPDATE SET
        rootId         = excluded.rootId,
        name           = excluded.name,
        parentPath     = excluded.parentPath,
        isDir          = excluded.isDir,
        size           = excluded.size,
        modTimeUnix    = excluded.modTimeUnix,
        scanGeneration = excluded.scanGeneration";

pub(super) fn upsert_parameters<'a>(session: &'a ScanSession, record: &'a FileRecord) -> [&'a dyn ToSql; 8] {
    [
        &session.root_id,
        &record.path,
        &record.name,
        &record.parent_path,
        &record.is_dir,
        &record.size,
        &record.mod_time_unix,
        &session.generation,
    ]
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs() as i64).unwrap_or(0)
}

fn lock(connection: &Mutex<Connection>) -> MutexGuard<'_, Connection> {
    connection.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Store {
    /// Opens or creates the index database. WAL keeps searches responsive while
    /// a crawl is writing.
    pub fn open(database_path: &Path) -> IndexResult<Store> {
        ensure_private_database_file(database_path)?;
        let mut writer = open_connection(database_path)?;
        migrate(&mut writer)?;
        let reader = open_connection(database_path)?;
        Ok(Store { writer: Mutex::new(writer), reader: Mutex::new(reader) })
    }

    pub(super) fn writer(&self) -> MutexGuard<'_, Connection> {
        lock(&self.writer)
    }

    pub(super) fn reader(&self) -> MutexGuard<'_, Connection> {
        lock(&self.reader)
    }

    pub fn new_batch(&self, session: ScanSession) -> Batch<'_> {
        Batch::new(self, session)
    }

    /// Registers a root, returning the existing id when it is already known.
    pub fn ensure_root(&self, root_path: &str) -> IndexResult<i64> {
        let database = self.writer();
        database
            .execute(
                "INSERT INTO roots (path, addedAtUnix) VALUES (?1, ?2) ON CONFLICT(path) DO NOTHING",
                params![root_path, unix_now()],
            )
            .map_err(IndexError::database("ensure root"))?;
        look_up_root_id(&database, root_path)
    }

    pub fn remove_root(&self, root_path: &str) -> IndexResult<()> {
        let affected = self
            .writer()
            .execute("DELETE FROM roots WHERE path = ?1", params![root_path])
            .map_err(IndexError::database("remove root"))?;
        if affected == 0 {
            return Err(IndexError::RootNotFound);
        }
        Ok(())
    }

    pub fn roots(&self) -> IndexResult<Vec<RootRecord>> {
        let database = self.reader();
        let mut statement = database
            .prepare(
                "SELECT roots.rootId, roots.path, roots.lastScanUnix, COUNT(files.fileId)
                 FROM roots
                 LEFT JOIN files ON files.rootId = roots.rootId
                 GROUP BY roots.rootId
                 ORDER BY roots.path",
            )
            .map_err(IndexError::database("list roots"))?;
        let rows = statement
            .query_map([], |row| {
                Ok(RootRecord {
                    root_id: row.get(0)?,
                    path: row.get(1)?,
                    last_scan_unix: row.get(2)?,
                    file_count: row.get(3)?,
                })
            })
            .map_err(IndexError::database("list roots"))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(IndexError::database("list roots"))
    }

    /// Bumps the root's generation and returns the session that the crawler
    /// stamps onto every row it writes.
    pub fn begin_scan(&self, root_id: i64) -> IndexResult<ScanSession> {
        let database = self.writer();
        database
            .execute("UPDATE roots SET scanGeneration = scanGeneration + 1 WHERE rootId = ?1", params![root_id])
            .map_err(IndexError::database("begin scan"))?;
        read_session(&database, root_id)
    }

    /// The in-progress generation for a root, which the watcher needs so its
    /// incremental writes survive the next sweep.
    pub fn current_session(&self, root_id: i64) -> IndexResult<ScanSession> {
        read_session(&self.writer(), root_id)
    }

    /// Removes everything the completed pass did not touch and records the scan
    /// time.
    pub fn finish_scan(&self, session: &ScanSession) -> IndexResult<i64> {
        let database = self.writer();
        let removed = database
            .execute(
                "DELETE FROM files WHERE rootId = ?1 AND scanGeneration <> ?2",
                params![session.root_id, session.generation],
            )
            .map_err(IndexError::database("sweep stale rows"))?;
        database
            .execute("UPDATE roots SET lastScanUnix = ?1 WHERE rootId = ?2", params![unix_now(), session.root_id])
            .map_err(IndexError::database("record scan time"))?;
        Ok(removed as i64)
    }

    /// Writes a single entry. The crawler uses `Batch` instead; this is the
    /// watcher's path.
    pub fn upsert_file(&self, session: &ScanSession, record: &FileRecord) -> IndexResult<()> {
        self.writer()
            .execute(UPSERT_FILE_STATEMENT, upsert_parameters(session, record))
            .map_err(IndexError::database("upsert file"))?;
        Ok(())
    }

    /// Removes a path and everything beneath it, which is what a single
    /// directory-removal event implies.
    pub fn delete_subtree(&self, path: &str) -> IndexResult<i64> {
        let like_pattern = format!("{}/%", escape_like_prefix(path));
        let removed = self
            .writer()
            .execute("DELETE FROM files WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\'", params![path, like_pattern])
            .map_err(IndexError::database("delete subtree"))?;
        Ok(removed as i64)
    }

    pub fn count_files(&self) -> IndexResult<i64> {
        self.reader()
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
            .map_err(IndexError::database("count files"))
    }
}

fn open_connection(database_path: &Path) -> IndexResult<Connection> {
    let connection = Connection::open(database_path).map_err(IndexError::database("open database"))?;
    configure_connection(&connection).map_err(IndexError::database("configure database"))?;
    Ok(connection)
}

fn configure_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.busy_timeout(BUSY_TIMEOUT)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "foreign_keys", "ON")
}

/// Creates or tightens the database before SQLite opens it. The index holds the
/// name of every file the user owns, and SQLite copies the main file's mode onto
/// the -wal and -shm files, so setting it here covers all three. A zero-length
/// file is a valid empty database.
fn ensure_private_database_file(database_path: &Path) -> IndexResult<()> {
    OpenOptions::new().create(true).truncate(false).read(true).write(true).mode(0o600).open(database_path)?;
    fs::set_permissions(database_path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

fn look_up_root_id(database: &Connection, root_path: &str) -> IndexResult<i64> {
    let root_id = database
        .query_row("SELECT rootId FROM roots WHERE path = ?1", params![root_path], |row| row.get(0))
        .optional()
        .map_err(IndexError::database("look up root"))?;
    root_id.ok_or(IndexError::RootNotFound)
}

fn read_session(database: &Connection, root_id: i64) -> IndexResult<ScanSession> {
    let session = database
        .query_row("SELECT path, scanGeneration FROM roots WHERE rootId = ?1", params![root_id], |row| {
            Ok(ScanSession { root_id, root_path: row.get(0)?, generation: row.get(1)? })
        })
        .optional()
        .map_err(IndexError::database("read scan generation"))?;
    session.ok_or(IndexError::RootNotFound)
}

/// Neutralizes LIKE wildcards in a path so a directory called "100% done"
/// cannot match unrelated siblings.
fn escape_like_prefix(path: &str) -> String {
    let mut escaped = String::with_capacity(path.len());
    for character in path.chars() {
        if matches!(character, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn open_test_store() -> (tempfile::TempDir, Store) {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::open(&directory.path().join("index.db")).unwrap();
        (directory, store)
    }

    fn begin_test_scan(store: &Store, root_path: &str) -> ScanSession {
        let root_id = store.ensure_root(root_path).unwrap();
        store.begin_scan(root_id).unwrap()
    }

    fn record_for(path: &str) -> FileRecord {
        let path_buffer = PathBuf::from(path);
        FileRecord {
            path: path.to_string(),
            name: path_buffer.file_name().unwrap().to_string_lossy().into_owned(),
            parent_path: path_buffer.parent().unwrap().to_string_lossy().into_owned(),
            ..FileRecord::default()
        }
    }

    fn upsert_path(store: &Store, session: &ScanSession, path: &str) {
        store.upsert_file(session, &record_for(path)).unwrap();
    }

    fn assert_private_mode(path: &Path) {
        let mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "{} has mode {:o}, want 600", path.display(), mode);
    }

    #[test]
    fn open_keeps_the_database_and_its_wal_private() {
        let (directory, store) = open_test_store();
        store.ensure_root("/tmp/root").unwrap();
        let database_path = directory.path().join("index.db");
        assert_private_mode(&database_path);
        assert_private_mode(&directory.path().join("index.db-wal"));
    }

    #[test]
    fn open_tightens_an_already_permissive_database() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("index.db");
        fs::write(&database_path, []).unwrap();
        fs::set_permissions(&database_path, fs::Permissions::from_mode(0o644)).unwrap();
        let _store = Store::open(&database_path).unwrap();
        assert_private_mode(&database_path);
    }

    #[test]
    fn delete_subtree_ignores_siblings_with_wildcard_characters() {
        let (_directory, store) = open_test_store();
        let session = begin_test_scan(&store, "/tmp/root");
        for path in ["/tmp/root/100% done/report.pdf", "/tmp/root/100% done", "/tmp/root/1000 done/report.pdf"] {
            upsert_path(&store, &session, path);
        }
        let removed = store.delete_subtree("/tmp/root/100% done").unwrap();
        assert_eq!(removed, 2, "the directory and its single child");
        assert_eq!(store.count_files().unwrap(), 1, "the sibling directory's file stays");
    }

    #[test]
    fn finish_scan_removes_rows_from_earlier_generations() {
        let (_directory, store) = open_test_store();
        let first_session = begin_test_scan(&store, "/tmp/root");
        upsert_path(&store, &first_session, "/tmp/root/stale.txt");
        upsert_path(&store, &first_session, "/tmp/root/kept.txt");

        let second_session = store.begin_scan(first_session.root_id).unwrap();
        upsert_path(&store, &second_session, "/tmp/root/kept.txt");

        let removed = store.finish_scan(&second_session).unwrap();
        assert_eq!(removed, 1, "only the file missing from the second scan");
    }

    #[test]
    fn removing_an_unknown_root_reports_it() {
        let (_directory, store) = open_test_store();
        assert!(matches!(store.remove_root("/nowhere"), Err(IndexError::RootNotFound)));
    }
}
