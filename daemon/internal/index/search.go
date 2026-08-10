package index

import (
	"database/sql"
	"fmt"
	"strings"
	"unicode"
)

const (
	defaultSearchLimit = 50
	maximumSearchLimit = 500
)

const searchStatement = `
	SELECT files.path, files.name, files.parentPath, files.isDir, files.size, files.modTimeUnix
	FROM fileSearch
	JOIN files ON files.fileId = fileSearch.rowid
	WHERE fileSearch MATCH ? AND (? = 0 OR files.isDir = 1)
	ORDER BY bm25(fileSearch), length(files.path), files.name
	LIMIT ?`

// Search matches indexed names. The second return value reports that more
// matches existed than the limit allowed.
func (store *Store) Search(text string, limit int, directoriesOnly bool) ([]FileRecord, bool, error) {
	matchExpression := buildMatchExpression(text)
	if matchExpression == "" {
		return []FileRecord{}, false, nil
	}
	effectiveLimit := clampSearchLimit(limit)
	rows, err := store.database.Query(searchStatement, matchExpression, directoriesOnly, effectiveLimit+1)
	if err != nil {
		return nil, false, fmt.Errorf("search: %w", err)
	}
	defer rows.Close()

	records, err := scanFileRows(rows)
	if err != nil {
		return nil, false, err
	}
	return truncateToLimit(records, effectiveLimit)
}

func truncateToLimit(records []FileRecord, limit int) ([]FileRecord, bool, error) {
	if len(records) <= limit {
		return records, false, nil
	}
	return records[:limit], true, nil
}

func clampSearchLimit(limit int) int {
	if limit <= 0 {
		return defaultSearchLimit
	}
	if limit > maximumSearchLimit {
		return maximumSearchLimit
	}
	return limit
}

func scanFileRows(rows *sql.Rows) ([]FileRecord, error) {
	records := make([]FileRecord, 0)
	for rows.Next() {
		record, err := scanFileRow(rows)
		if err != nil {
			return nil, err
		}
		records = append(records, record)
	}
	return records, rows.Err()
}

func scanFileRow(rows *sql.Rows) (FileRecord, error) {
	var record FileRecord
	err := rows.Scan(
		&record.Path, &record.Name, &record.ParentPath,
		&record.IsDir, &record.Size, &record.ModTimeUnix,
	)
	return record, err
}

// buildMatchExpression turns free text into an FTS5 query. Each term becomes a
// quoted prefix match, so "rep inv" finds "report-invoice.pdf" while user input
// can never inject FTS5 operators. An unusable query yields an empty string.
func buildMatchExpression(text string) string {
	terms := splitSearchTerms(text)
	if len(terms) == 0 {
		return ""
	}
	quotedTerms := make([]string, 0, len(terms))
	for _, term := range terms {
		quotedTerms = append(quotedTerms, `"`+term+`"*`)
	}
	return strings.Join(quotedTerms, " ")
}

// splitSearchTerms splits on the same separators the tokenizer uses, so typing a
// full filename matches the words it was indexed under.
func splitSearchTerms(text string) []string {
	fields := strings.FieldsFunc(text, isTermSeparator)
	terms := make([]string, 0, len(fields))
	for _, field := range fields {
		terms = appendUsableTerm(terms, field)
	}
	return terms
}

func isTermSeparator(character rune) bool {
	if unicode.IsSpace(character) {
		return true
	}
	return strings.ContainsRune(`_-./\`, character)
}

func appendUsableTerm(terms []string, field string) []string {
	term := strings.ReplaceAll(field, `"`, "")
	if !containsSearchableRune(term) {
		return terms
	}
	return append(terms, term)
}

// containsSearchableRune drops terms the tokenizer would reduce to nothing, such
// as a lone "!!", which would make the FTS5 expression invalid.
func containsSearchableRune(term string) bool {
	for _, character := range term {
		if unicode.IsLetter(character) || unicode.IsDigit(character) {
			return true
		}
	}
	return false
}
