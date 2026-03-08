use std::collections::HashSet;

use anyhow::Result;
use serde::Serialize;

use crate::store::state::StateDb;

use super::planner::{Plan, TaskNode};
use super::spawner::Spawner;

#[derive(Debug, Clone, Serialize)]
pub struct GoalStatus {
    pub goal_id: String,
    pub description: String,
    pub status: String,
    pub tasks: Vec<TaskStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskStatus {
    pub id: String,
    pub description: String,
    pub status: String,
    pub depends_on: Vec<String>,
}

pub struct Coordinator {
    #[allow(dead_code)]
    spawner: Spawner,
    #[allow(dead_code)]
    max_agents: usize,
}

impl Coordinator {
    pub fn new(spawner: Spawner, max_agents: usize) -> Self {
        Self {
            spawner,
            max_agents,
        }
    }

    /// Get tasks that are ready to run (all dependencies completed).
    pub fn ready_tasks<'a>(plan: &'a Plan, completed: &HashSet<String>) -> Vec<&'a TaskNode> {
        plan.tasks
            .iter()
            .filter(|task| {
                // Skip tasks that are already completed
                if completed.contains(&task.id) {
                    return false;
                }
                // A task is ready when all its dependencies are completed
                task.depends_on.iter().all(|dep| completed.contains(dep))
            })
            .collect()
    }

    /// Update task status in DB.
    pub fn update_task_status(
        db: &StateDb,
        task_id: &str,
        status: &str,
        stdout: Option<&str>,
        stderr: Option<&str>,
    ) -> Result<()> {
        let conn = db.conn();
        conn.execute(
            "UPDATE tasks SET status = ?1, stdout = ?2, stderr = ?3 WHERE id = ?4",
            rusqlite::params![status, stdout, stderr, task_id],
        )?;
        Ok(())
    }

    /// Update goal status in DB.
    pub fn update_goal_status(db: &StateDb, goal_id: &str, status: &str) -> Result<()> {
        let conn = db.conn();
        conn.execute(
            "UPDATE goals SET status = ?1 WHERE id = ?2",
            rusqlite::params![status, goal_id],
        )?;
        Ok(())
    }

    /// Get goal status summary.
    pub fn goal_status(db: &StateDb, goal_id: &str) -> Result<GoalStatus> {
        let conn = db.conn();

        let (description, status): (String, String) = conn.query_row(
            "SELECT description, status FROM goals WHERE id = ?1",
            rusqlite::params![goal_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let mut stmt = conn.prepare(
            "SELECT id, description, status, depends_on FROM tasks WHERE goal_id = ?1 ORDER BY created_at",
        )?;

        let rows = stmt.query_map(rusqlite::params![goal_id], |row| {
            let deps_raw: Option<String> = row.get(3)?;
            Ok(TaskStatus {
                id: row.get(0)?,
                description: row.get(1)?,
                status: row.get(2)?,
                depends_on: deps_raw
                    .map(|d| d.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
            })
        })?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }

        Ok(GoalStatus {
            goal_id: goal_id.to_string(),
            description,
            status,
            tasks,
        })
    }
}
