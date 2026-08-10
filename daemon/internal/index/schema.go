package index

import (
	"database/sql"
	"fmt"
)

const schemaVersion = 1

// schemaStatements is the version 1 schema. fileSearch is an external-content
// FTS5 table: it stores only the tokenized name and reads everything else back
// from files through the triggers below, which keeps the index compact.
var schemaStatements = []string{
	`CREATE TABLE roots (
		rootId         INTEGER PRIMARY KEY,
		path           TEXT    NOT NULL UNIQUE,
		addedAtUnix    INTEGER NOT NULL,
		lastScanUnix   INTEGER NOT NULL DEFAULT 0,
		scanGeneration INTEGER NOT NULL DEFAULT 0
	)`,
	`CREATE TABLE files (
		fileId         INTEGER PRIMARY KEY,
		rootId         INTEGER NOT NULL REFERENCES roots(rootId) ON DELETE CASCADE,
		path           TEXT    NOT NULL UNIQUE,
		name           TEXT    NOT NULL,
		parentPath     TEXT    NOT NULL,
		isDir          INTEGER NOT NULL,
		size           INTEGER NOT NULL,
		modTimeUnix    INTEGER NOT NULL,
		scanGeneration INTEGER NOT NULL
	)`,
	`CREATE INDEX filesByRoot ON files(rootId, scanGeneration)`,
	`CREATE INDEX filesByParent ON files(parentPath)`,
	// Separators split "annual_report.tar.gz" into searchable words so a query
	// for "report" matches without needing substring search.
	`CREATE VIRTUAL TABLE fileSearch USING fts5(
		name,
		content='files',
		content_rowid='fileId',
		tokenize="unicode61 separators '_-.'"
	)`,
	`CREATE TRIGGER filesAfterInsert AFTER INSERT ON files BEGIN
		INSERT INTO fileSearch(rowid, name) VALUES (new.fileId, new.name);
	END`,
	`CREATE TRIGGER filesAfterDelete AFTER DELETE ON files BEGIN
		INSERT INTO fileSearch(fileSearch, rowid, name) VALUES ('delete', old.fileId, old.name);
	END`,
	`CREATE TRIGGER filesAfterUpdate AFTER UPDATE ON files BEGIN
		INSERT INTO fileSearch(fileSearch, rowid, name) VALUES ('delete', old.fileId, old.name);
		INSERT INTO fileSearch(rowid, name) VALUES (new.fileId, new.name);
	END`,
}

// migrate brings an empty or older database up to schemaVersion. The index is
// rebuildable from the filesystem, so a future migration is always free to drop
// and recreate rather than transform.
func migrate(database *sql.DB) error {
	currentVersion, err := readSchemaVersion(database)
	if err != nil {
		return err
	}
	if currentVersion >= schemaVersion {
		return nil
	}
	return applySchema(database)
}

func readSchemaVersion(database *sql.DB) (int, error) {
	var version int
	err := database.QueryRow(`PRAGMA user_version`).Scan(&version)
	if err != nil {
		return 0, fmt.Errorf("read schema version: %w", err)
	}
	return version, nil
}

func applySchema(database *sql.DB) error {
	transaction, err := database.Begin()
	if err != nil {
		return err
	}
	defer transaction.Rollback()

	if err := executeAll(transaction, schemaStatements); err != nil {
		return err
	}
	if _, err := transaction.Exec(fmt.Sprintf(`PRAGMA user_version = %d`, schemaVersion)); err != nil {
		return fmt.Errorf("set schema version: %w", err)
	}
	return transaction.Commit()
}

func executeAll(transaction *sql.Tx, statements []string) error {
	for _, statement := range statements {
		if _, err := transaction.Exec(statement); err != nil {
			return fmt.Errorf("apply schema: %w", err)
		}
	}
	return nil
}
