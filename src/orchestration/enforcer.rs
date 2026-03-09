use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::store::state::StateDb;

/// The action an enforcer rule produces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            // 5. Version before push
            Rule {
                name: "onto_version_before_push".into(),
                description: "Warn if pushing without a saved version snapshot".into(),
                action: Action::Warn,
                enabled: true,
                condition: RuleCondition::MissingInWindow {
                    trigger: "onto_push".into(),
                    required: "onto_version".into(),
                    window: 5,
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

    /// Seed the built-in hardcoded rules into the `rules` table.
    /// Uses INSERT OR IGNORE so existing DB customisations are preserved.
    pub fn seed_builtins_to_db(db: &StateDb) -> anyhow::Result<()> {
        let builtins = Enforcer::new();
        let conn = db.conn();
        for rule in &builtins.rules {
            let action_str = match rule.action {
                Action::Block => "block",
                Action::Warn => "warn",
                Action::Allow => "allow",
            };
            let condition_json = serde_json::to_string(&rule.condition)?;
            conn.execute(
                "INSERT OR IGNORE INTO rules (name, description, condition, action, enabled) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    rule.name,
                    rule.description,
                    condition_json,
                    action_str,
                    rule.enabled as i32,
                ],
            )?;
        }
        Ok(())
    }

    /// Replace the in-memory rule-set with everything in the `rules` table.
    /// The sliding window (recent_calls) is left untouched.
    pub fn reload_from_db(&mut self, db: &StateDb) -> anyhow::Result<()> {
        // Collect raw rows while holding the lock, then release it before processing.
        let raw: Vec<(String, String, String, String, i32)> = {
            let conn = db.conn();
            let mut stmt = conn.prepare(
                "SELECT name, description, condition, action, enabled FROM rules",
            )?;
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i32>(4)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect()
        }; // MutexGuard dropped here

        let rules: Vec<Rule> = raw
            .into_iter()
            .filter_map(|(name, description, condition_json, action_str, enabled)| {
                let condition: RuleCondition = serde_json::from_str(&condition_json)
                    .map_err(|e| {
                        tracing::warn!("skipping rule '{name}': invalid condition JSON: {e}");
                        e
                    })
                    .ok()?;
                let action = match action_str.as_str() {
                    "block" => Action::Block,
                    "warn" => Action::Warn,
                    "allow" => Action::Allow,
                    other => {
                        tracing::warn!("skipping rule '{name}': unknown action '{other}'");
                        return None;
                    }
                };
                Some(Rule { name, description, action, enabled: enabled != 0, condition })
            })
            .collect();

        self.rules = rules;
        Ok(())
    }

    /// Seed rules from TOML config into the DB.
    /// Uses INSERT OR REPLACE so TOML always wins for named rules.
    pub fn seed_config_rules_to_db(
        db: &StateDb,
        rules: &[crate::config::RuleConfig],
    ) -> anyhow::Result<()> {
        // Build all (name, description, condition_json, action_str, enabled) tuples
        // before acquiring the lock, so serialisation happens outside the mutex.
        let mut rows: Vec<(String, String, String, String, i32)> = Vec::new();
        for rule in rules {
            let condition = match Self::condition_from_config(&rule.condition) {
                Ok(Some(c)) => c,
                Ok(None) => {
                    tracing::warn!(
                        "skipping rule '{}': unrecognised condition type '{}'",
                        rule.name,
                        rule.condition.kind
                    );
                    continue;
                }
                Err(msg) => {
                    tracing::warn!("skipping rule '{}': {msg}", rule.name);
                    continue;
                }
            };
            let action_str = match rule.action.as_str() {
                s @ ("block" | "warn" | "allow") => s.to_string(),
                other => {
                    tracing::warn!("skipping rule '{}': unknown action '{other}'", rule.name);
                    continue;
                }
            };
            let condition_json = serde_json::to_string(&condition)?;
            let enabled = rule.enabled.unwrap_or(true) as i32;
            rows.push((
                rule.name.clone(),
                rule.description.as_deref().unwrap_or("").to_string(),
                condition_json,
                action_str,
                enabled,
            ));
        }

        // Acquire lock once for all inserts.
        let conn = db.conn();
        for (name, description, condition_json, action_str, enabled) in rows {
            conn.execute(
                "INSERT OR REPLACE INTO rules (name, description, condition, action, enabled) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![name, description, condition_json, action_str, enabled],
            )?;
        }
        Ok(())
    }

    fn condition_from_config(
        cfg: &crate::config::RuleConditionConfig,
    ) -> Result<Option<RuleCondition>, String> {
        match cfg.kind.as_str() {
            "MissingInWindow" => {
                let trigger = cfg.trigger.clone().ok_or("MissingInWindow requires 'trigger'")?;
                let required = cfg.required.clone().ok_or("MissingInWindow requires 'required'")?;
                let window = cfg.window.ok_or("MissingInWindow requires 'window'")?;
                Ok(Some(RuleCondition::MissingInWindow { trigger, required, window }))
            }
            "RepeatWithout" => {
                let category = cfg.category.clone().ok_or("RepeatWithout requires 'category'")?;
                let count = cfg.count.ok_or("RepeatWithout requires 'count'")?;
                let required = cfg.required.clone().ok_or("RepeatWithout requires 'required'")?;
                Ok(Some(RuleCondition::RepeatWithout { category, count, required }))
            }
            _ => Ok(None), // unknown kind — caller handles the warning
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::state::StateDb;
    use tempfile::tempdir;

    fn test_db() -> (tempfile::TempDir, StateDb) {
        let dir = tempdir().unwrap();
        let db = StateDb::open(&dir.path().join("test.db")).unwrap();
        (dir, db)
    }

    #[test]
    fn test_seed_and_reload() {
        let (_dir, db) = test_db();
        Enforcer::seed_builtins_to_db(&db).unwrap();
        let mut e = Enforcer::new();
        e.reload_from_db(&db).unwrap();
        assert!(!e.rules().is_empty());
        assert_eq!(
            e.rules().iter().find(|r| r.name == "qa_after_docx_write").unwrap().enabled,
            true
        );
    }

    #[test]
    fn test_seed_config_rules() {
        use crate::config::RuleConfig;
        let (_dir, db) = test_db();
        Enforcer::seed_builtins_to_db(&db).unwrap();

        let custom = vec![RuleConfig {
            name: "custom_test_rule".into(),
            description: Some("test".into()),
            action: "warn".into(),
            enabled: Some(true),
            condition: crate::config::RuleConditionConfig {
                kind: "MissingInWindow".into(),
                trigger: Some("foo".into()),
                required: Some("bar".into()),
                window: Some(2),
                category: None,
                count: None,
            },
        }];
        Enforcer::seed_config_rules_to_db(&db, &custom).unwrap();

        let mut e = Enforcer::new();
        e.reload_from_db(&db).unwrap();
        assert!(e.rules().iter().any(|r| r.name == "custom_test_rule"));
    }

    #[test]
    fn test_condition_roundtrip() {
        let cond = RuleCondition::MissingInWindow {
            trigger: "write_doc".into(),
            required: "qa_".into(),
            window: 3,
        };
        let json = serde_json::to_string(&cond).unwrap();
        let decoded: RuleCondition = serde_json::from_str(&json).unwrap();
        match decoded {
            RuleCondition::MissingInWindow { trigger, required, window } => {
                assert_eq!(trigger, "write_doc");
                assert_eq!(required, "qa_");
                assert_eq!(window, 3);
            }
            _ => panic!("wrong variant"),
        }
    }
}
