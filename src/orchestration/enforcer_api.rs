use axum::extract::State;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::enforcer::{Action, Enforcer};
use crate::store::state::StateDb;

#[derive(Deserialize)]
pub struct LineageEvent {
    pub source: Option<String>,
    pub session_id: Option<String>,
    pub seq: Option<i64>,
    pub event_type: Option<String>,
    pub operation: String,
    pub details: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Serialize)]
pub struct VerdictResponse {
    pub action: String,
    pub rule: Option<String>,
    pub reason: Option<String>,
}

pub struct EnforcerApiState {
    pub enforcer: Arc<Mutex<Enforcer>>,
    pub db: StateDb,
}

async fn handle_event(
    State(state): State<Arc<EnforcerApiState>>,
    Json(event): Json<LineageEvent>,
) -> Json<VerdictResponse> {
    let mut enforcer = state.enforcer.lock().await;
    enforcer.post_check(&event.operation);
    let verdict = enforcer.pre_check(&event.operation);
    let session_id = event.session_id.as_deref().unwrap_or("external");
    let _ = Enforcer::log_verdict(&state.db, session_id, &verdict, &event.operation);
    let action_str = match verdict.action {
        Action::Block => "block",
        Action::Warn => "warn",
        Action::Allow => "allow",
    };
    Json(VerdictResponse {
        action: action_str.to_string(),
        rule: verdict.rule,
        reason: verdict.reason,
    })
}

pub fn enforcer_router(state: Arc<EnforcerApiState>) -> Router {
    Router::new()
        .route("/api/enforcer/event", axum::routing::post(handle_event))
        .with_state(state)
}
