use std::collections::VecDeque;

use serde::Serialize;

use crate::store::state::StateDb;

/// The action an enforcer rule produces.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Action {
    Block,
    Warn,
    Allow,
}

/// The result of evaluating all enforcer rules against a tool call.
#[derive(Debug, Clone, Serialize)]
pub struct Verdict {
    pub action: Action,
    pub rule: Option<String>,
    pub reason: Option<String>,
}

/// A tool call recorded in the enforcer's sliding window.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub timestamp: i64,
}

/// A single enforcer rule.
#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub description: String,
    pub action: Action,
    pub enabled: bool,
    pub condition: RuleCondition,
}

/// The condition that determines when a rule fires.
#[derive(Debug, Clone)]
pub enum RuleCondition {
    /// Tool X called without tool Y in the last N calls.
    MissingInWindow {
        trigger: String,
        required: String,
        window: usize,
    },
    /// N+ calls to same category without tool Y.
    RepeatWithout {
        category: String,
        count: usize,
        required: String,
    },
}

/// A row from the enforcement log table.
#[derive(Debug, Clone, Serialize)]
pub struct EnforcementEntry {
    pub seq: i64,
    pub session_id: Option<String>,
    pub timestamp: String,
    pub rule: String,
    pub action: String,
    pub tool_call: Option<String>,
    pub reason: Option<String>,
}

/// The enforcer engine. Evaluates rules against a sliding window of recent
/// tool calls and produces verdicts (Block / Warn / Allow).
pub struct Enforcer {
    rules: Vec<Rule>,
    recent_calls: VecDeque<ToolCall>,
    max_history: usize,
}

impl Enforcer {
    /// Create a new enforcer pre-loaded with the built-in rules.
    pub fn new() -> Self {
        let rules = vec![
            // 1. QA after docx write
            Rule {
                name: "qa_after_docx_write".into(),
                description: "Document write tool called but no qa_ tool in last 3 calls".into(),
                action: Action::Warn,
                enabled: true,
                condition: RuleCondition::MissingInWindow {
                    trigger: "write_document".into(),
                    required: "qa_".into(),
                    window: 3,
                },
            },
            // 2. Render after edit
            Rule {
                name: "render_after_edit".into(),
                description: "3+ document write calls without render".into(),
                action: Action::Warn,
                enabled: true,
                condition: RuleCondition::RepeatWithout {
                    category: "write_document".into(),
                    count: 3,
                    required: "render_document".into(),
                },
            },
            // 3. Health gate (stub)
            Rule {
                name: "health_gate".into(),
                description: "Stub -- health check integration is Phase 7.2".into(),
                action: Action::Allow,
                enabled: true,
                condition: RuleCondition::MissingInWindow {
                    trigger: "__never_match__".into(),
                    required: "__never_match__".into(),
                    window: 0,
                },
            },
            // 4. Ontology validate after save
            Rule {
                name: "onto_validate_after_save".into(),
                description: "Warn if ontology is saved 3+ times without validation".into(),
                action: Action::Warn,
                enabled: true,
                condition: RuleCondition::RepeatWithout {
                    category: "onto_save".into(),
                    count: 3,
                    required: "onto_validate".into(),
                },
            },
        ];

        Self {
            rules,
            recent_calls: VecDeque::new(),
            max_history: 100,
        }
    }

    /// Check all active rules against the current tool call. Returns the
    /// most severe verdict (Block > Warn > Allow).
    pub fn pre_check(&mut self, tool_name: &str) -> Verdict {
        let mut worst = Verdict {
            action: Action::Allow,
            rule: None,
            reason: None,
        };

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            // Stub rules that always allow skip evaluation.
            if rule.action == Action::Allow {
                continue;
            }

            if self.matches_condition(tool_name, &rule.condition) {
                // Take the most severe action seen so far.
                if severity(&rule.action) > severity(&worst.action) {
                    worst = Verdict {
                        action: rule.action.clone(),
                        rule: Some(rule.name.clone()),
                        reason: Some(rule.description.clone()),
                    };
                }
            }
        }

        worst
    }

    /// Record a tool call into the sliding window after it has been executed.
    pub fn post_check(&mut self, tool_name: &str) {
        self.recent_calls.push_back(ToolCall {
            name: tool_name.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        });

        while self.recent_calls.len() > self.max_history {
            self.recent_calls.pop_front();
        }
    }

    /// Persist a verdict to the enforcement table in the state database.
    pub fn log_verdict(
        db: &StateDb,
        session_id: &str,
        verdict: &Verdict,
        tool_name: &str,
    ) -> anyhow::Result<()> {
        let action_str = match verdict.action {
            Action::Block => "block",
            Action::Warn => "warn",
            Action::Allow => "allow",
        };

        let rule_name = verdict.rule.as_deref().unwrap_or("none");

        let conn = db.conn();
        conn.execute(
            "INSERT INTO enforcement (session_id, rule, action, tool_call, reason) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                session_id,
                rule_name,
                action_str,
                tool_name,
                verdict.reason.as_deref(),
            ],
        )?;
        Ok(())
    }

    /// Read the enforcement log, optionally filtered by session.
    pub fn get_log(
        db: &StateDb,
        session_id: Option<&str>,
        limit: usize,
    ) -> Vec<EnforcementEntry> {
        let conn = db.conn();

        let (sql, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match session_id {
            Some(sid) => (
                "SELECT seq, session_id, timestamp, rule, action, tool_call, reason \
                 FROM enforcement WHERE session_id = ?1 ORDER BY seq DESC LIMIT ?2",
                vec![
                    Box::new(sid.to_string()) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(limit as i64),
                ],
            ),
            None => (
                "SELECT seq, session_id, timestamp, rule, action, tool_call, reason \
                 FROM enforcement ORDER BY seq DESC LIMIT ?1",
                vec![Box::new(limit as i64) as Box<dyn rusqlite::types::ToSql>],
            ),
        };

        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(EnforcementEntry {
                seq: row.get(0)?,
                session_id: row.get(1)?,
                timestamp: row.get(2)?,
                rule: row.get(3)?,
                action: row.get(4)?,
                tool_call: row.get(5)?,
                reason: row.get(6)?,
            })
        });

        match rows {
            Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Return a slice of all rules (active and inactive).
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Enable or disable a rule by name. Returns true if the rule was found.
    pub fn set_rule_enabled(&mut self, name: &str, enabled: bool) -> bool {
        for rule in &mut self.rules {
            if rule.name == name {
                rule.enabled = enabled;
                return true;
            }
        }
        false
    }

    // -- private helpers --

    fn matches_condition(&self, tool_name: &str, condition: &RuleCondition) -> bool {
        match condition {
            RuleCondition::MissingInWindow {
                trigger,
                required,
                window,
            } => {
                // Does the current tool match the trigger?
                if !tool_matches(tool_name, trigger) {
                    return false;
                }

                // Look at the last `window` entries; if none start with
                // `required`, the rule fires.
                let history_len = self.recent_calls.len();
                let start = if *window >= history_len {
                    0
                } else {
                    history_len - window
                };

                !self.recent_calls.iter().skip(start).any(|tc| {
                    tc.name.starts_with(required.as_str())
                })
            }
            RuleCondition::RepeatWithout {
                category,
                count,
                required,
            } => {
                let cat_count = self
                    .recent_calls
                    .iter()
                    .filter(|tc| tc.name.starts_with(category.as_str()))
                    .count();

                if cat_count < *count {
                    return false;
                }

                // Fire if none of the recent calls start with `required`.
                !self
                    .recent_calls
                    .iter()
                    .any(|tc| tc.name.starts_with(required.as_str()))
            }
        }
    }
}

/// Check whether a tool name matches a trigger pattern.
/// Matches if the tool name equals the trigger OR starts with it.
fn tool_matches(tool_name: &str, trigger: &str) -> bool {
    tool_name == trigger || tool_name.starts_with(trigger)
}

/// Numeric severity for comparison: Block > Warn > Allow.
fn severity(action: &Action) -> u8 {
    match action {
        Action::Allow => 0,
        Action::Warn => 1,
        Action::Block => 2,
    }
}
