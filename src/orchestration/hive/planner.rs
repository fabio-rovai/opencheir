use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::sentinel_core::state::StateDb;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: String,
    pub description: String,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub goal_id: String,
    pub tasks: Vec<TaskNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GoalSummary {
    pub id: String,
    pub description: String,
    pub status: String,
    pub task_count: usize,
}

pub struct Planner;

impl Planner {
    /// Create a plan from a goal description. Persists goal + tasks to DB.
    /// In production this would spawn Claude CLI for decomposition.
    /// For now, creates a single-task plan from the goal description.
    pub fn create_plan(db: &StateDb, session_id: &str, description: &str) -> Result<Plan> {
        let goal_id = Uuid::new_v4().to_string();
        let task_id = Uuid::new_v4().to_string();

        let conn = db.conn();

        conn.execute(
            "INSERT INTO goals (id, session_id, description, status) VALUES (?1, ?2, ?3, 'pending')",
            rusqlite::params![goal_id, session_id, description],
        )?;

        conn.execute(
            "INSERT INTO tasks (id, goal_id, description, status) VALUES (?1, ?2, ?3, 'pending')",
            rusqlite::params![task_id, goal_id, description],
        )?;

        let task = TaskNode {
            id: task_id,
            description: description.to_string(),
            depends_on: Vec::new(),
        };

        Ok(Plan {
            goal_id,
            tasks: vec![task],
        })
    }

    /// Load a plan from the DB by goal_id.
    pub fn load_plan(db: &StateDb, goal_id: &str) -> Result<Option<Plan>> {
        let conn = db.conn();

        // Check goal exists
        let goal_exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM goals WHERE id = ?1",
            rusqlite::params![goal_id],
            |row| {
                let count: i64 = row.get(0)?;
                Ok(count > 0)
            },
        )?;

        if !goal_exists {
            return Ok(None);
        }

        let mut stmt = conn.prepare(
            "SELECT id, description, depends_on FROM tasks WHERE goal_id = ?1 ORDER BY created_at",
        )?;

        let rows = stmt.query_map(rusqlite::params![goal_id], |row| {
            let deps_raw: Option<String> = row.get(2)?;
            Ok(TaskNode {
                id: row.get(0)?,
                description: row.get(1)?,
                depends_on: deps_raw
                    .map(|d| d.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
            })
        })?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }

        Ok(Some(Plan {
            goal_id: goal_id.to_string(),
            tasks,
        }))
    }

    /// Get all goals for a session.
    pub fn list_goals(db: &StateDb, session_id: &str) -> Result<Vec<GoalSummary>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT g.id, g.description, g.status,
                    (SELECT COUNT(*) FROM tasks t WHERE t.goal_id = g.id)
             FROM goals g
             WHERE g.session_id = ?1
             ORDER BY g.created_at",
        )?;

        let rows = stmt.query_map(rusqlite::params![session_id], |row| {
            Ok(GoalSummary {
                id: row.get(0)?,
                description: row.get(1)?,
                status: row.get(2)?,
                task_count: row.get::<_, i64>(3)? as usize,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}
