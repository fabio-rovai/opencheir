use anyhow::Result;
use serde::Serialize;

use crate::store::state::StateDb;

#[derive(Debug, Clone, Serialize)]
pub struct Pattern {
    pub id: Option<i64>,
    pub category: String,
    pub description: String,
    pub evidence: Option<String>,
    pub confidence: f64,
    pub occurrences: usize,
    pub actionable: bool,
}

pub struct PatternService;

impl PatternService {
    /// Analyze enforcement log to discover patterns.
    /// Returns newly discovered patterns.
    pub fn analyze_enforcement(db: &StateDb) -> Result<Vec<Pattern>> {
        let conn = db.conn();
        let mut patterns = Vec::new();

        // Pattern 1: Rules that block frequently (3+ times)
        {
            let mut stmt = conn.prepare(
                "SELECT rule, COUNT(*) as cnt FROM enforcement \
                 WHERE action = 'block' \
                 GROUP BY rule HAVING cnt >= 3 \
                 ORDER BY cnt DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (rule, count) = row?;
                patterns.push(Pattern {
                    id: None,
                    category: "frequent_block".into(),
                    description: format!("Rule '{}' has blocked {} times", rule, count),
                    evidence: Some(format!("enforcement log: {} blocks", count)),
                    confidence: 0.8,
                    occurrences: count as usize,
                    actionable: true,
                });
            }
        }

        // Pattern 2: Rules that never fire (0 blocks across all enforcement entries)
        {
            let mut stmt = conn.prepare(
                "SELECT r.name FROM rules r \
                 WHERE r.enabled = 1 \
                 AND r.name NOT IN (SELECT DISTINCT rule FROM enforcement) \
                 AND (SELECT COUNT(*) FROM enforcement) > 10",
            )?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            for row in rows {
                let rule = row?;
                patterns.push(Pattern {
                    id: None,
                    category: "unused_rule".into(),
                    description: format!(
                        "Rule '{}' has never fired despite 10+ enforcement entries",
                        rule
                    ),
                    evidence: None,
                    confidence: 0.5,
                    occurrences: 0,
                    actionable: true,
                });
            }
        }

        // Pattern 3: Repeated blocks on same tool (friction point)
        {
            let mut stmt = conn.prepare(
                "SELECT tool_call, COUNT(*) as cnt FROM enforcement \
                 WHERE action = 'block' AND tool_call IS NOT NULL \
                 GROUP BY tool_call HAVING cnt >= 3 \
                 ORDER BY cnt DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (tool, count) = row?;
                patterns.push(Pattern {
                    id: None,
                    category: "friction_point".into(),
                    description: format!(
                        "Tool '{}' blocked {} times — possible friction point",
                        tool, count
                    ),
                    evidence: Some(format!("{} blocks on {}", count, tool)),
                    confidence: 0.7,
                    occurrences: count as usize,
                    actionable: true,
                });
            }
        }

        Ok(patterns)
    }

    /// Store a pattern in the patterns table. Returns the generated ID.
    pub fn store(db: &StateDb, pattern: &Pattern) -> Result<i64> {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO patterns (category, description, evidence, confidence, occurrences, actionable) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                pattern.category,
                pattern.description,
                pattern.evidence,
                pattern.confidence,
                pattern.occurrences as i64,
                pattern.actionable as i64,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get all stored patterns, optionally filtered by category.
    pub fn list(db: &StateDb, category: Option<&str>) -> Result<Vec<Pattern>> {
        let conn = db.conn();
        let mut results = Vec::new();

        if let Some(cat) = category {
            let mut stmt = conn.prepare(
                "SELECT id, category, description, evidence, confidence, occurrences, actionable \
                 FROM patterns WHERE category = ?1 ORDER BY confidence DESC",
            )?;
            let rows = stmt.query_map(rusqlite::params![cat], |row| {
                Ok(Pattern {
                    id: Some(row.get(0)?),
                    category: row.get(1)?,
                    description: row.get(2)?,
                    evidence: row.get(3)?,
                    confidence: row.get(4)?,
                    occurrences: row.get::<_, i64>(5)? as usize,
                    actionable: row.get::<_, i64>(6)? != 0,
                })
            })?;
            for row in rows {
                results.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, category, description, evidence, confidence, occurrences, actionable \
                 FROM patterns ORDER BY confidence DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(Pattern {
                    id: Some(row.get(0)?),
                    category: row.get(1)?,
                    description: row.get(2)?,
                    evidence: row.get(3)?,
                    confidence: row.get(4)?,
                    occurrences: row.get::<_, i64>(5)? as usize,
                    actionable: row.get::<_, i64>(6)? != 0,
                })
            })?;
            for row in rows {
                results.push(row?);
            }
        }

        Ok(results)
    }

    /// Count patterns.
    pub fn count(db: &StateDb) -> Result<usize> {
        let conn = db.conn();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM patterns", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}
