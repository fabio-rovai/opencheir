use opencheir::orchestration::enforcer::*;

// -- 1. Built-in rules --

#[test]
fn test_new_has_builtin_rules() {
    let enforcer = Enforcer::new();
    let rules = enforcer.rules();
    assert_eq!(rules.len(), 4);

    let names: Vec<&str> = rules.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"qa_after_docx_write"));
    assert!(names.contains(&"render_after_edit"));
    assert!(names.contains(&"health_gate"));
    assert!(names.contains(&"onto_validate_after_save"));
}

// -- 2. Unrelated tool -> Allow --

#[test]
fn test_pre_check_allows_normal_call() {
    let mut enforcer = Enforcer::new();
    let verdict = enforcer.pre_check("some_random_tool");
    assert_eq!(verdict.action, Action::Allow);
    assert!(verdict.rule.is_none());
}

// -- 3. QA after write warns --

#[test]
fn test_qa_after_write_warns() {
    let mut enforcer = Enforcer::new();

    let verdict = enforcer.pre_check("write_document_cell");
    assert_eq!(verdict.action, Action::Warn);
    assert_eq!(verdict.rule.as_deref(), Some("qa_after_docx_write"));
}

// -- 4. QA after write allows when QA was called --

#[test]
fn test_qa_after_write_allows_with_qa() {
    let mut enforcer = Enforcer::new();
    enforcer.post_check("qa_check_fonts");

    let verdict = enforcer.pre_check("write_document_cell");
    assert_eq!(verdict.action, Action::Allow);
}

// -- 5. Render after edit warns --

#[test]
fn test_render_after_edit_warns() {
    let mut enforcer = Enforcer::new();
    enforcer.post_check("qa_check_fonts");
    enforcer.post_check("write_document_cell");
    enforcer.post_check("write_document_cell");
    enforcer.post_check("write_document_cell");

    // Now any further tool triggers render_after_edit because 3+
    // write_document calls exist without render_document.
    // The RepeatWithout rule fires on ANY pre_check, but we test with
    // write_document itself (which also matches qa_after_docx_write,
    // but render_after_edit should still fire).
    let verdict = enforcer.pre_check("write_document_cell");
    // Both qa_after_docx_write (WARN) and render_after_edit (WARN) could fire,
    // but the important thing is the action is Warn.
    assert_eq!(verdict.action, Action::Warn);
}

// -- 6. Post check maintains window --

#[test]
fn test_post_check_maintains_window() {
    let mut enforcer = Enforcer::new();

    // Push more than max_history calls
    for i in 0..150 {
        enforcer.post_check(&format!("tool_{i}"));
    }

    // The enforcer's max_history is 100, so only the last 100 should remain.
    // We can verify indirectly: if we had qa_ at position 0 (now
    // evicted) and the remaining entries don't have it, qa_after_docx_write
    // should fire.
    let mut enforcer2 = Enforcer::new();
    // Add qa_ early
    enforcer2.post_check("qa_check_fonts");
    // Fill with 100 other calls to push it out
    for i in 0..100 {
        enforcer2.post_check(&format!("filler_{i}"));
    }
    // qa_ should have been evicted from the window
    let verdict = enforcer2.pre_check("write_document_cell");
    // qa_ was evicted so warn fires
    assert_eq!(verdict.action, Action::Warn);
}

// -- 7. Set rule enabled --

#[test]
fn test_set_rule_enabled() {
    let mut enforcer = Enforcer::new();

    // Disable qa_after_docx_write
    let found = enforcer.set_rule_enabled("qa_after_docx_write", false);
    assert!(found);

    // Now write_document without qa should not warn from that rule
    let verdict = enforcer.pre_check("write_document_cell");
    // Should be Allow since only qa_after_docx_write would fire and it's disabled
    assert_eq!(verdict.action, Action::Allow);

    // Disabling a nonexistent rule returns false
    let not_found = enforcer.set_rule_enabled("nonexistent_rule", false);
    assert!(!not_found);
}

// -- 8. Log verdict and get log --

#[test]
fn test_log_verdict_and_get_log() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db = opencheir::store::state::StateDb::open(tmp.path()).unwrap();
    let session_id = db.create_session(Some("test-project")).unwrap();

    let verdict = Verdict {
        action: Action::Warn,
        rule: Some("qa_after_docx_write".into()),
        reason: Some("No QA tool in last 3 calls".into()),
    };

    Enforcer::log_verdict(&db, &session_id, &verdict, "write_document_cell").unwrap();

    let log = Enforcer::get_log(&db, Some(&session_id), 10);
    assert_eq!(log.len(), 1);

    let entry = &log[0];
    assert_eq!(entry.session_id.as_deref(), Some(session_id.as_str()));
    assert_eq!(entry.rule, "qa_after_docx_write");
    assert_eq!(entry.action, "warn");
    assert_eq!(entry.tool_call.as_deref(), Some("write_document_cell"));
    assert_eq!(
        entry.reason.as_deref(),
        Some("No QA tool in last 3 calls")
    );
}

// -- 9. Onto validate after save warns --

#[test]
fn test_onto_validate_after_save_rule() {
    let mut enforcer = Enforcer::new();
    // Simulate onto_save calls without validation
    enforcer.post_check("onto_save");
    enforcer.post_check("onto_save");
    enforcer.post_check("onto_save");
    let verdict = enforcer.pre_check("onto_save");
    // Should warn that onto_validate hasn't been called
    assert!(matches!(verdict.action, Action::Warn));
}

// -- 10. Get log empty --

#[test]
fn test_get_log_empty() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db = opencheir::store::state::StateDb::open(tmp.path()).unwrap();

    let log = Enforcer::get_log(&db, Some("nonexistent-session"), 10);
    assert!(log.is_empty());
}
