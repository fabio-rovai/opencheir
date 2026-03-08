use anyhow::Result;
use serde::Serialize;

use crate::store::state::StateDb;

#[derive(Debug, Clone, Serialize)]
pub struct Learning {
    pub id: i64,
    pub domain: String,
    pub lesson: String,
    pub context: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
}

pub struct MemoryService;

impl MemoryService {
    /// Store a learning. Returns the generated row ID.
    pub fn store(
        db: &StateDb,
        domain: &str,
        lesson: &str,
        context: Option<&str>,
        tags: &[&str],
    ) -> Result<i64> {
        let tags_csv = if tags.is_empty() {
            None
        } else {
            Some(tags.join(","))
        };

        let conn = db.conn();
        conn.execute(
            "INSERT INTO learnings (domain, lesson, context, tags) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![domain, lesson, context, tags_csv],
        )?;
        let id = conn.last_insert_rowid();
        Ok(id)
    }

    /// FTS5 search over learnings. Returns up to `limit` results.
    pub fn recall(db: &StateDb, query: &str, limit: usize) -> Result<Vec<Learning>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT l.id, l.domain, l.lesson, l.context, l.tags, l.created_at
             FROM learnings l
             JOIN learnings_fts f ON l.id = f.rowid
             WHERE learnings_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(rusqlite::params![query, limit as i64], |row| {
            let tags_raw: Option<String> = row.get(4)?;
            Ok(Learning {
                id: row.get(0)?,
                domain: row.get(1)?,
                lesson: row.get(2)?,
                context: row.get(3)?,
                tags: tags_raw
                    .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                created_at: row.get(5)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// List learnings by domain.
    pub fn by_domain(db: &StateDb, domain: &str) -> Result<Vec<Learning>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, domain, lesson, context, tags, created_at
             FROM learnings
             WHERE domain = ?1
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map(rusqlite::params![domain], |row| {
            let tags_raw: Option<String> = row.get(4)?;
            Ok(Learning {
                id: row.get(0)?,
                domain: row.get(1)?,
                lesson: row.get(2)?,
                context: row.get(3)?,
                tags: tags_raw
                    .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                created_at: row.get(5)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Count learnings.
    pub fn count(db: &StateDb) -> Result<usize> {
        let conn = db.conn();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM learnings", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}
