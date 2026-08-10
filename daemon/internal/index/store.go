package index

import (
	"database/sql"
	"errors"
	"fmt"
	"net/url"
	"path/filepath"
	"time"

	_ "modernc.org/sqlite"
)

// ErrRootNotFound means the requested path was never registered as a root.
var ErrRootNotFound = errors.New("index: root not found")

// Store owns the on-disk index. It is safe for concurrent use: reads run in WAL
// mode alongside the single writer.
type Store struct {
	database *sql.DB
}

// FileRecord is one indexed filesystem entry.
type FileRecord struct {
	Path        string
	Name        string
	ParentPath  string
	IsDir       bool
	Size        int64
	ModTimeUnix int64
}

// RootRecord describes a registered root and how far its last scan got.
type RootRecord struct {
	RootId       int64
	Path         string
	FileCount    int64
	LastScanUnix int64
}

// ScanSession identifies one pass over a root. Every row written during the pass
// carries Generation, so anything left at an older generation afterwards no
// longer exists on disk and gets swept.
type ScanSession struct {
	RootId     int64
	RootPath   string
	Generation int64
}

// Open opens or creates the index database. WAL keeps searches responsive while
// a crawl is writing.
func Open(databasePath string) (*Store, error) {
	database, err := sql.Open("sqlite", buildDataSourceName(databasePath))
	if err != nil {
		return nil, err
	}
	database.SetMaxOpenConns(8)
	if err := migrate(database); err != nil {
		database.Close()
		return nil, err
	}
	return &Store{database: database}, nil
}

func (store *Store) Close() error {
	return store.database.Close()
}

// EnsureRoot registers a root, returning the existing id when it is already
// known.
func (store *Store) EnsureRoot(rootPath string) (int64, error) {
	_, err := store.database.Exec(
		`INSERT INTO roots (path, addedAtUnix) VALUES (?, ?) ON CONFLICT(path) DO NOTHING`,
		rootPath, time.Now().Unix(),
	)
	if err != nil {
		return 0, fmt.Errorf("ensure root: %w", err)
	}
	return store.rootId(rootPath)
}

func (store *Store) RemoveRoot(rootPath string) error {
	result, err := store.database.Exec(`DELETE FROM roots WHERE path = ?`, rootPath)
	if err != nil {
		return fmt.Errorf("remove root: %w", err)
	}
	affected, err := result.RowsAffected()
	if err != nil {
		return err
	}
	if affected == 0 {
		return ErrRootNotFound
	}
	return nil
}

func (store *Store) Roots() ([]RootRecord, error) {
	rows, err := store.database.Query(`
		SELECT roots.rootId, roots.path, roots.lastScanUnix, COUNT(files.fileId)
		FROM roots
		LEFT JOIN files ON files.rootId = roots.rootId
		GROUP BY roots.rootId
		ORDER BY roots.path`)
	if err != nil {
		return nil, fmt.Errorf("list roots: %w", err)
	}
	defer rows.Close()
	return scanRootRows(rows)
}

// BeginScan bumps the root's generation and returns the session that the crawler
// stamps onto every row it writes.
func (store *Store) BeginScan(rootId int64) (ScanSession, error) {
	_, err := store.database.Exec(
		`UPDATE roots SET scanGeneration = scanGeneration + 1 WHERE rootId = ?`, rootId)
	if err != nil {
		return ScanSession{}, fmt.Errorf("begin scan: %w", err)
	}
	return store.currentSession(rootId)
}

// CurrentSession returns the in-progress generation for a root, which the
// watcher needs so its incremental writes survive the next sweep.
func (store *Store) CurrentSession(rootId int64) (ScanSession, error) {
	return store.currentSession(rootId)
}

// FinishScan removes everything the completed pass did not touch and records the
// scan time.
func (store *Store) FinishScan(session ScanSession) (int64, error) {
	result, err := store.database.Exec(
		`DELETE FROM files WHERE rootId = ? AND scanGeneration <> ?`,
		session.RootId, session.Generation,
	)
	if err != nil {
		return 0, fmt.Errorf("sweep stale rows: %w", err)
	}
	removed, err := result.RowsAffected()
	if err != nil {
		return 0, err
	}
	_, err = store.database.Exec(
		`UPDATE roots SET lastScanUnix = ? WHERE rootId = ?`, time.Now().Unix(), session.RootId)
	if err != nil {
		return removed, fmt.Errorf("record scan time: %w", err)
	}
	return removed, nil
}

// UpsertFile writes a single entry. The crawler uses Batch instead; this is the
// watcher's path.
func (store *Store) UpsertFile(session ScanSession, record FileRecord) error {
	_, err := store.database.Exec(upsertFileStatement, upsertArguments(session, record)...)
	if err != nil {
		return fmt.Errorf("upsert file: %w", err)
	}
	return nil
}

// DeleteSubtree removes a path and everything beneath it, which is what a single
// directory-removal event implies.
func (store *Store) DeleteSubtree(path string) (int64, error) {
	result, err := store.database.Exec(
		`DELETE FROM files WHERE path = ? OR path LIKE ? ESCAPE '\'`,
		path, escapeLikePrefix(path)+string(filepath.Separator)+`%`,
	)
	if err != nil {
		return 0, fmt.Errorf("delete subtree: %w", err)
	}
	return result.RowsAffected()
}

func (store *Store) CountFiles() (int64, error) {
	var count int64
	err := store.database.QueryRow(`SELECT COUNT(*) FROM files`).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("count files: %w", err)
	}
	return count, nil
}

func (store *Store) rootId(rootPath string) (int64, error) {
	var rootId int64
	err := store.database.QueryRow(`SELECT rootId FROM roots WHERE path = ?`, rootPath).Scan(&rootId)
	if errors.Is(err, sql.ErrNoRows) {
		return 0, ErrRootNotFound
	}
	if err != nil {
		return 0, fmt.Errorf("look up root: %w", err)
	}
	return rootId, nil
}

func (store *Store) currentSession(rootId int64) (ScanSession, error) {
	session := ScanSession{RootId: rootId}
	err := store.database.QueryRow(
		`SELECT path, scanGeneration FROM roots WHERE rootId = ?`, rootId,
	).Scan(&session.RootPath, &session.Generation)
	if errors.Is(err, sql.ErrNoRows) {
		return ScanSession{}, ErrRootNotFound
	}
	if err != nil {
		return ScanSession{}, fmt.Errorf("read scan generation: %w", err)
	}
	return session, nil
}

func scanRootRows(rows *sql.Rows) ([]RootRecord, error) {
	roots := make([]RootRecord, 0)
	for rows.Next() {
		var record RootRecord
		if err := rows.Scan(&record.RootId, &record.Path, &record.LastScanUnix, &record.FileCount); err != nil {
			return nil, err
		}
		roots = append(roots, record)
	}
	return roots, rows.Err()
}

const upsertFileStatement = `
	INSERT INTO files (rootId, path, name, parentPath, isDir, size, modTimeUnix, scanGeneration)
	VALUES (?, ?, ?, ?, ?, ?, ?, ?)
	ON CONFLICT(path) DO UPDATE SET
		rootId         = excluded.rootId,
		name           = excluded.name,
		parentPath     = excluded.parentPath,
		isDir          = excluded.isDir,
		size           = excluded.size,
		modTimeUnix    = excluded.modTimeUnix,
		scanGeneration = excluded.scanGeneration`

func upsertArguments(session ScanSession, record FileRecord) []any {
	return []any{
		session.RootId,
		record.Path,
		record.Name,
		record.ParentPath,
		record.IsDir,
		record.Size,
		record.ModTimeUnix,
		session.Generation,
	}
}

// buildDataSourceName sets the pragmas at connection time; modernc's driver
// applies one _pragma parameter per setting.
func buildDataSourceName(databasePath string) string {
	parameters := url.Values{}
	parameters.Add("_pragma", "journal_mode(WAL)")
	parameters.Add("_pragma", "synchronous(NORMAL)")
	parameters.Add("_pragma", "foreign_keys(ON)")
	parameters.Add("_pragma", "busy_timeout(5000)")
	return "file:" + databasePath + "?" + parameters.Encode()
}

// escapeLikePrefix neutralizes LIKE wildcards in a path so a directory called
// "100% done" cannot match unrelated siblings.
func escapeLikePrefix(path string) string {
	escaped := make([]rune, 0, len(path))
	for _, character := range path {
		escaped = appendLikeRune(escaped, character)
	}
	return string(escaped)
}

func appendLikeRune(escaped []rune, character rune) []rune {
	if character == '%' || character == '_' || character == '\\' {
		return append(escaped, '\\', character)
	}
	return append(escaped, character)
}
