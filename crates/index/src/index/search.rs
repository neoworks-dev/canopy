use rusqlite::params;

use super::error::{IndexError, IndexResult};
use super::store::{FileRecord, Store};

pub const DEFAULT_SEARCH_LIMIT: i64 = 50;
pub const MAXIMUM_SEARCH_LIMIT: i64 = 500;

const SEARCH_STATEMENT: &str = "
    SELECT files.path, files.name, files.parentPath, files.isDir, files.size, files.modTimeUnix
    FROM fileSearch
    JOIN files ON files.fileId = fileSearch.rowid
    WHERE fileSearch MATCH ?1
        AND (?2 = 0 OR files.isDir = 1)
        AND (?3 = '' OR (files.path >= ?4 AND files.path < ?5))
    ORDER BY bm25(fileSearch), length(files.path), files.name
    LIMIT ?6";

/// One search request.
#[derive(Debug, Clone, Default)]
pub struct Query {
    pub text: String,
    pub limit: i64,
    pub directories_only: bool,
    /// Limits results to a directory and everything under it. Empty searches
    /// every indexed root.
    pub scope: String,
}

/// Matches indexed names. `truncated` reports that more matches existed than
/// the limit allowed.
#[derive(Debug, Clone, Default)]
pub struct SearchOutcome {
    pub records: Vec<FileRecord>,
    pub truncated: bool,
}

impl Store {
    pub fn search(&self, query: &Query) -> IndexResult<SearchOutcome> {
        let match_expression = build_match_expression(&query.text);
        if match_expression.is_empty() {
            return Ok(SearchOutcome::default());
        }
        let effective_limit = clamp_search_limit(query.limit);
        let (lower_bound, upper_bound) = scope_bounds(&query.scope);
        let mut records = self.run_search(&match_expression, query, &lower_bound, &upper_bound, effective_limit + 1)?;
        let truncated = records.len() as i64 > effective_limit;
        records.truncate(effective_limit as usize);
        Ok(SearchOutcome { records, truncated })
    }

    fn run_search(
        &self,
        match_expression: &str,
        query: &Query,
        lower_bound: &str,
        upper_bound: &str,
        row_limit: i64,
    ) -> IndexResult<Vec<FileRecord>> {
        let database = self.reader();
        let mut statement = database.prepare_cached(SEARCH_STATEMENT).map_err(IndexError::database("search"))?;
        let rows = statement
            .query_map(
                params![match_expression, query.directories_only, query.scope, lower_bound, upper_bound, row_limit],
                |row| {
                    Ok(FileRecord {
                        path: row.get(0)?,
                        name: row.get(1)?,
                        parent_path: row.get(2)?,
                        is_dir: row.get(3)?,
                        size: row.get(4)?,
                        mod_time_unix: row.get(5)?,
                    })
                },
            )
            .map_err(IndexError::database("search"))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(IndexError::database("search"))
    }
}

/// Turns a directory into the half-open path range covering everything below
/// it. '0' is the byte after '/', so the range needs no pattern matching and
/// the unique index on path can serve it. A scope of "/" bounds every absolute
/// path, which is what searching the root should mean.
pub(super) fn scope_bounds(scope: &str) -> (String, String) {
    let trimmed = scope.strip_suffix('/').unwrap_or(scope);
    (format!("{trimmed}/"), format!("{trimmed}0"))
}

pub(super) fn clamp_search_limit(limit: i64) -> i64 {
    if limit <= 0 {
        return DEFAULT_SEARCH_LIMIT;
    }
    limit.min(MAXIMUM_SEARCH_LIMIT)
}

/// Turns free text into an FTS5 query. Each term becomes a quoted prefix match,
/// so "rep inv" finds "report-invoice.pdf" while user input can never inject
/// FTS5 operators. An unusable query yields an empty string.
pub(super) fn build_match_expression(text: &str) -> String {
    let quoted_terms: Vec<String> = split_search_terms(text).iter().map(|term| format!("\"{term}\"*")).collect();
    quoted_terms.join(" ")
}

/// Splits on the same separators the tokenizer uses, so typing a full filename
/// matches the words it was indexed under.
fn split_search_terms(text: &str) -> Vec<String> {
    text.split(is_term_separator)
        .map(|field| field.replace('"', ""))
        .filter(|term| contains_searchable_character(term))
        .collect()
}

fn is_term_separator(character: char) -> bool {
    character.is_whitespace() || "_-./\\".contains(character)
}

/// Drops terms the tokenizer would reduce to nothing, such as a lone "!!",
/// which would make the FTS5 expression invalid.
fn contains_searchable_character(term: &str) -> bool {
    term.chars().any(char::is_alphanumeric)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_expression_splits_on_filename_punctuation() {
        let cases = [
            ("report", "\"report\"*"),
            ("annual report", "\"annual\"* \"report\"*"),
            ("annual_report", "\"annual\"* \"report\"*"),
            ("notes.txt", "\"notes\"* \"txt\"*"),
            ("src/lib", "\"src\"* \"lib\"*"),
            ("wei\"rd", "\"weird\"*"),
            ("!! ??", ""),
            ("   ", ""),
        ];
        for (text, expected) in cases {
            assert_eq!(build_match_expression(text), expected, "text {text:?}");
        }
    }

    #[test]
    fn clamp_search_limit_applies_bounds() {
        assert_eq!(clamp_search_limit(0), DEFAULT_SEARCH_LIMIT);
        assert_eq!(clamp_search_limit(-5), DEFAULT_SEARCH_LIMIT);
        assert_eq!(clamp_search_limit(MAXIMUM_SEARCH_LIMIT + 1), MAXIMUM_SEARCH_LIMIT);
        assert_eq!(clamp_search_limit(10), 10);
    }

    #[test]
    fn scope_bounds_cover_everything_below_a_directory() {
        let (lower, upper) = scope_bounds("/home/moritz/Downloads");
        let contains = |path: &str| path >= lower.as_str() && path < upper.as_str();
        assert!(contains("/home/moritz/Downloads/a.txt"));
        assert!(contains("/home/moritz/Downloads/deep/b.txt"));
        assert!(!contains("/home/moritz/Downloads2/a.txt"));
        assert!(!contains("/home/moritz/Downloads"));
        assert!(!contains("/home/moritz/a"));
    }

    #[test]
    fn scope_bounds_cover_every_absolute_path_at_the_root() {
        let (lower, upper) = scope_bounds("/");
        for path in ["/a", "/home/moritz/x", "/zzz"] {
            assert!(path >= lower.as_str() && path < upper.as_str(), "{path}");
        }
    }
}
