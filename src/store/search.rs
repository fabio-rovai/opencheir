use crate::store::state::StateDb;
use anyhow::Result;
use serde::Serialize;

/// A single result from an FTS5 full-text search query.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub source: String,
    pub source_id: String,
    pub title: String,
    pub snippet: String,
    pub rank: f64,
}

/// Stateless search service built on SQLite FTS5.
///
/// All methods operate on a [`StateDb`] reference and use the `search_fts`
/// virtual table (created in the schema migration) for indexing and querying.
/// Ranking uses FTS5's built-in BM25 algorithm.
pub struct SearchService;

impl SearchService {
    /// Index a piece of content into the FTS5 search table.
    pub fn index(
        db: &StateDb,
        source: &str,
        source_id: &str,
        title: &str,
        content: &str,
        tags: &str,
    ) -> Result<()> {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO search_fts (source, source_id, title, content, tags) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![source, source_id, title, content, tags],
        )?;
        Ok(())
    }

    /// Search the FTS5 index. Returns results ranked by BM25 relevance.
    ///
    /// FTS5 `rank` values are negative — more negative means more relevant.
    /// Results are ordered ascending by rank so the best matches come first.
    ///
    /// When `source_filter` is `Some`, only rows whose `source` column
    /// matches the filter are returned.
    pub fn search(
        db: &StateDb,
        query: &str,
        source_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let conn = db.conn();

        let sql = if source_filter.is_some() {
            "SELECT source, source_id, title, \
                    snippet(search_fts, 3, '<b>', '</b>', '...', 32), rank \
             FROM search_fts \
             WHERE search_fts MATCH ?1 AND source = ?2 \
             ORDER BY rank \
             LIMIT ?3"
        } else {
            "SELECT source, source_id, title, \
                    snippet(search_fts, 3, '<b>', '</b>', '...', 32), rank \
             FROM search_fts \
             WHERE search_fts MATCH ?1 \
             ORDER BY rank \
             LIMIT ?2"
        };

        let mut stmt = conn.prepare(sql)?;

        let rows: Vec<SearchResult> = if let Some(filter) = source_filter {
            stmt.query_map(rusqlite::params![query, filter, limit], |row| {
                Ok(SearchResult {
                    source: row.get(0)?,
                    source_id: row.get(1)?,
                    title: row.get(2)?,
                    snippet: row.get(3)?,
                    rank: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect()
        } else {
            stmt.query_map(rusqlite::params![query, limit], |row| {
                Ok(SearchResult {
                    source: row.get(0)?,
                    source_id: row.get(1)?,
                    title: row.get(2)?,
                    snippet: row.get(3)?,
                    rank: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect()
        };

        Ok(rows)
    }

    /// Remove all indexed content for a given source.
    ///
    /// Returns the number of rows deleted.
    pub fn clear_source(db: &StateDb, source: &str) -> Result<usize> {
        let conn = db.conn();
        let count = conn.execute(
            "DELETE FROM search_fts WHERE source = ?1",
            rusqlite::params![source],
        )?;
        Ok(count)
    }

    /// Count total indexed documents in the FTS5 table.
    pub fn count(db: &StateDb) -> Result<usize> {
        let conn = db.conn();
        let count: usize = conn.query_row(
            "SELECT COUNT(*) FROM search_fts",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Find content similar to the given text by extracting key terms and
    /// querying FTS5 with OR-joined keywords.
    ///
    /// Words of 3 characters or fewer are skipped (common stop-words). Up to
    /// 10 significant words are used from the input text.
    pub fn find_similar(
        db: &StateDb,
        text: &str,
        source_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let words: Vec<&str> = text
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .take(10)
            .collect();

        if words.is_empty() {
            return Ok(vec![]);
        }

        let query = words.join(" OR ");
        Self::search(db, &query, source_filter, limit)
    }
}
