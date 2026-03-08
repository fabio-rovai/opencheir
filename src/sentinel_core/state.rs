use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const MIGRATION: &str = include_str!("migrations/001_initial.sql");

#[derive(Clone)]
pub struct StateDb {
    conn: Arc<Mutex<Connection>>,
}

impl StateDb {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // PRAGMAs that return results (like journal_mode) must be run
        // individually via pragma_update or query, not execute_batch.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", "-64000")?;

        // Strip PRAGMA lines from the migration SQL before running DDL,
        // since we already applied them above.
        let ddl: String = MIGRATION
            .lines()
            .filter(|line| !line.trim_start().starts_with("PRAGMA"))
            .collect::<Vec<_>>()
            .join("\n");

        conn.execute_batch(&ddl)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn list_tables(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master \
             WHERE type='table' AND name NOT LIKE 'sqlite_%' \
             ORDER BY name",
        )?;
        let tables = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(tables)
    }

    pub fn create_session(&self, project: Option<&str>) -> Result<String> {
        let id = Uuid::new_v4().to_string()[..8].to_string();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, project) VALUES (?1, ?2)",
            rusqlite::params![id, project],
        )?;
        Ok(id)
    }

    pub fn seed_company(&self) -> Result<()> {
        let json_str = include_str!("../../data/company.json");
        let data: serde_json::Value = serde_json::from_str(json_str)?;

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO company (key, value, sensitive, category) VALUES (?1, ?2, ?3, ?4)"
        )?;

        for section in ["company", "psc", "insurance", "turnover", "reference_contracts"] {
            if let Some(items) = data.get(section).and_then(|v| v.as_array()) {
                for item in items {
                    stmt.execute(rusqlite::params![
                        item["key"].as_str().unwrap_or_default(),
                        item["value"].as_str().unwrap_or_default(),
                        item["sensitive"].as_i64().unwrap_or(0),
                        item["category"].as_str().unwrap_or_default(),
                    ])?;
                }
            }
        }
        Ok(())
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

    /// Record health status for a component in the health table.
    pub fn record_health(
        &self,
        component: &str,
        kind: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO health (component, kind, status, last_check, error, restart_count)
             VALUES (?1, ?2, ?3, datetime('now'), ?4, 0)
             ON CONFLICT(component) DO UPDATE SET
                status = excluded.status,
                last_check = excluded.last_check,
                error = excluded.error",
            rusqlite::params![component, kind, status, error],
        )?;
        Ok(())
    }

    /// Increment the restart count for a component in the health table.
    pub fn increment_restart_count(&self, component: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE health SET restart_count = restart_count + 1 WHERE component = ?1",
            rusqlite::params![component],
        )?;
        Ok(())
    }

    /// Get the health status for a component.
    pub fn get_health(&self, component: &str) -> Result<Option<(String, String, Option<String>, i64)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT kind, status, error, restart_count FROM health WHERE component = ?1",
        )?;
        let result = stmt.query_row(rusqlite::params![component], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
            ))
        });
        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
