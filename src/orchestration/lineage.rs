use std::collections::{HashMap, HashSet};

use axum::extract::{Path, Query, State};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::store::state::StateDb;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEvent {
    pub seq: Option<i64>,
    pub session_id: Option<String>,
    pub timestamp: i64, // Unix millis
    pub event_type: String, // "tool_call", "tool_result", "file_read", "file_write"
    pub path: Option<String>,
    pub tool: Option<String>,
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventTimeline {
    pub events: Vec<LineageEvent>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyEdge {
    pub from_tool: String,
    pub to_tool: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyGraph {
    pub nodes: Vec<String>, // unique tool names
    pub edges: Vec<DependencyEdge>,
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

pub struct LineageService;

impl LineageService {
    /// Record a new event. Returns the auto-generated sequence number.
    pub fn record_event(db: &StateDb, event: &LineageEvent) -> anyhow::Result<i64> {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO events (session_id, timestamp, event_type, path, tool, meta)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                event.session_id,
                event.timestamp,
                event.event_type,
                event.path,
                event.tool,
                event.meta.as_ref().map(|m| m.to_string()),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get events, optionally filtered by session and/or event type.
    pub fn get_events(
        db: &StateDb,
        session_id: Option<&str>,
        event_type: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<LineageEvent>> {
        let conn = db.conn();

        let mut sql = String::from("SELECT seq, session_id, timestamp, event_type, path, tool, meta FROM events");
        let mut conditions: Vec<String> = Vec::new();

        if session_id.is_some() {
            conditions.push("session_id = ?".into());
        }
        if event_type.is_some() {
            conditions.push("event_type = ?".into());
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY seq ASC LIMIT ?");

        let mut stmt = conn.prepare(&sql)?;

        // Bind parameters dynamically based on which filters are present.
        let mut param_idx = 1u32;
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(sid) = session_id {
            params.push(Box::new(sid.to_string()));
            param_idx += 1;
        }
        if let Some(et) = event_type {
            params.push(Box::new(et.to_string()));
            param_idx += 1;
        }
        let _ = param_idx; // suppress unused warning
        params.push(Box::new(limit as i64));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let meta_str: Option<String> = row.get(6)?;
            let meta = meta_str.and_then(|s| serde_json::from_str(&s).ok());
            Ok(LineageEvent {
                seq: row.get(0)?,
                session_id: row.get(1)?,
                timestamp: row.get(2)?,
                event_type: row.get(3)?,
                path: row.get(4)?,
                tool: row.get(5)?,
                meta,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Get event timeline for a session.
    pub fn get_timeline(db: &StateDb, session_id: &str) -> anyhow::Result<EventTimeline> {
        let events = Self::get_events(db, Some(session_id), None, 1000)?;
        let total = events.len();
        Ok(EventTimeline { events, total })
    }

    /// Build a dependency graph from sequential tool_call events.
    pub fn build_graph(
        db: &StateDb,
        session_id: Option<&str>,
    ) -> anyhow::Result<DependencyGraph> {
        let events = Self::get_events(db, session_id, Some("tool_call"), 10000)?;

        let mut nodes = HashSet::new();
        let mut edge_counts: HashMap<(String, String), usize> = HashMap::new();

        for window in events.windows(2) {
            if let (Some(from), Some(to)) = (&window[0].tool, &window[1].tool) {
                nodes.insert(from.clone());
                nodes.insert(to.clone());
                *edge_counts.entry((from.clone(), to.clone())).or_insert(0) += 1;
            }
        }

        let mut sorted_nodes: Vec<String> = nodes.into_iter().collect();
        sorted_nodes.sort();

        let mut edges: Vec<DependencyEdge> = edge_counts
            .into_iter()
            .map(|((from, to), count)| DependencyEdge {
                from_tool: from,
                to_tool: to,
                count,
            })
            .collect();
        edges.sort_by(|a, b| a.from_tool.cmp(&b.from_tool).then(a.to_tool.cmp(&b.to_tool)));

        Ok(DependencyGraph {
            nodes: sorted_nodes,
            edges,
        })
    }

    /// Count events by type for a session.
    pub fn event_counts(
        db: &StateDb,
        session_id: &str,
    ) -> anyhow::Result<Vec<(String, usize)>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT event_type, COUNT(*) FROM events WHERE session_id = ?1 GROUP BY event_type ORDER BY event_type",
        )?;
        let rows = stmt.query_map(rusqlite::params![session_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Delete events older than `days` days. Returns number of rows deleted.
    pub fn cleanup(db: &StateDb, days: u32) -> anyhow::Result<usize> {
        let conn = db.conn();
        let cutoff = chrono::Utc::now().timestamp_millis() - (days as i64 * 86_400_000);
        let count = conn.execute(
            "DELETE FROM events WHERE timestamp < ?1",
            rusqlite::params![cutoff],
        )?;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// HTTP API handlers (for VSCode extension)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub session_id: Option<String>,
    pub event_type: Option<String>,
    pub limit: Option<usize>,
}

async fn get_events_handler(
    State(db): State<StateDb>,
    Query(params): Query<EventsQuery>,
) -> Json<Vec<LineageEvent>> {
    let limit = params.limit.unwrap_or(100);
    let events = LineageService::get_events(
        &db,
        params.session_id.as_deref(),
        params.event_type.as_deref(),
        limit,
    )
    .unwrap_or_default();
    Json(events)
}

async fn get_timeline_handler(
    State(db): State<StateDb>,
    Path(session_id): Path<String>,
) -> Json<EventTimeline> {
    let timeline = LineageService::get_timeline(&db, &session_id)
        .unwrap_or(EventTimeline {
            events: vec![],
            total: 0,
        });
    Json(timeline)
}

#[derive(Debug, Deserialize)]
pub struct GraphQuery {
    pub session_id: Option<String>,
}

async fn get_graph_handler(
    State(db): State<StateDb>,
    Query(params): Query<GraphQuery>,
) -> Json<DependencyGraph> {
    let graph = LineageService::build_graph(&db, params.session_id.as_deref())
        .unwrap_or(DependencyGraph {
            nodes: vec![],
            edges: vec![],
        });
    Json(graph)
}

async fn get_stats_handler(
    State(db): State<StateDb>,
    Path(session_id): Path<String>,
) -> Json<Vec<(String, usize)>> {
    let counts = LineageService::event_counts(&db, &session_id).unwrap_or_default();
    Json(counts)
}

/// Build the axum router for the lineage HTTP API.
///
/// Mount this under the main server in Phase 7.
pub fn lineage_router(db: StateDb) -> Router {
    Router::new()
        .route("/api/events", axum::routing::get(get_events_handler))
        .route(
            "/api/timeline/{session_id}",
            axum::routing::get(get_timeline_handler),
        )
        .route("/api/graph", axum::routing::get(get_graph_handler))
        .route(
            "/api/stats/{session_id}",
            axum::routing::get(get_stats_handler),
        )
        .with_state(db)
}
