use anyhow::Result;
use serde::Serialize;

use crate::store::state::StateDb;

#[derive(Debug, Clone, Serialize)]
pub struct ClaimResult {
    pub token: String,
    pub domain: String,
    pub locked_by: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LockInfo {
    pub token: String,
    pub locked_by: String,
    pub expires_at: String,
}

pub struct LockService;

impl LockService {
    pub fn claim(db: &StateDb, domain: &str, locked_by: &str, ttl_seconds: u32) -> Result<ClaimResult> {
        let token = uuid::Uuid::new_v4().to_string();

        let expires_at = {
            let conn = db.conn();

            // Remove any expired lock for this domain
            conn.execute(
                "DELETE FROM locks WHERE domain = ?1 AND expires_at < datetime('now')",
                rusqlite::params![domain],
            )?;

            // Try to claim
            let result = conn.execute(
                "INSERT INTO locks (id, domain, locked_by, ttl_seconds, expires_at) \
                 VALUES (?1, ?2, ?3, ?4, datetime('now', ?5))",
                rusqlite::params![
                    token,
                    domain,
                    locked_by,
                    ttl_seconds,
                    format!("+{ttl_seconds} seconds"),
                ],
            );

            if let Err(rusqlite::Error::SqliteFailure(ref err, _)) = result {
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    let (holder, exp): (String, String) = conn.query_row(
                        "SELECT locked_by, expires_at FROM locks WHERE domain = ?1",
                        rusqlite::params![domain],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )?;
                    return Err(anyhow::anyhow!(
                        "domain '{}' is locked by '{}' until {}",
                        domain, holder, exp
                    ));
                }
            }
            result?;

            conn.query_row(
                "SELECT expires_at FROM locks WHERE id = ?1",
                rusqlite::params![token],
                |row| row.get::<_, String>(0),
            )?
        };

        Ok(ClaimResult {
            token,
            domain: domain.to_string(),
            locked_by: locked_by.to_string(),
            expires_at,
        })
    }

    pub fn release(db: &StateDb, domain: &str, token: &str) -> Result<()> {
        let conn = db.conn();
        conn.execute(
            "DELETE FROM locks WHERE domain = ?1 AND id = ?2",
            rusqlite::params![domain, token],
        )?;
        Ok(())
    }

    pub fn check(db: &StateDb, domain: &str) -> Result<Option<LockInfo>> {
        let conn = db.conn();

        // Purge expired
        conn.execute(
            "DELETE FROM locks WHERE domain = ?1 AND expires_at < datetime('now')",
            rusqlite::params![domain],
        )?;

        let result = conn.query_row(
            "SELECT id, locked_by, expires_at FROM locks WHERE domain = ?1",
            rusqlite::params![domain],
            |row| {
                Ok(LockInfo {
                    token: row.get(0)?,
                    locked_by: row.get(1)?,
                    expires_at: row.get(2)?,
                })
            },
        );

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_db() -> (tempfile::TempDir, StateDb) {
        let dir = tempdir().unwrap();
        let db = StateDb::open(&dir.path().join("test.db")).unwrap();
        (dir, db)
    }

    #[test]
    fn test_claim_and_release() {
        let (_dir, db) = test_db();
        let result = LockService::claim(&db, "ops", "agent-1", 60).unwrap();
        assert_eq!(result.domain, "ops");
        assert!(!result.token.is_empty());

        let info = LockService::check(&db, "ops").unwrap();
        assert!(info.is_some());
        assert_eq!(info.unwrap().locked_by, "agent-1");

        LockService::release(&db, "ops", &result.token).unwrap();
        assert!(LockService::check(&db, "ops").unwrap().is_none());
    }

    #[test]
    fn test_claim_conflict() {
        let (_dir, db) = test_db();
        LockService::claim(&db, "ops", "agent-1", 60).unwrap();
        let err = LockService::claim(&db, "ops", "agent-2", 60);
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("locked by"));
    }

    #[test]
    fn test_release_is_idempotent() {
        let (_dir, db) = test_db();
        let r = LockService::claim(&db, "ops", "agent-1", 60).unwrap();
        LockService::release(&db, "ops", &r.token).unwrap();
        LockService::release(&db, "ops", &r.token).unwrap();
    }
}
