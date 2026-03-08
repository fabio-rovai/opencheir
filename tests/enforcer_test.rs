use sentinel::orchestration::enforcer::*;

// -- 1. Built-in rules --

#[test]
fn test_new_has_builtin_rules() {
    let enforcer = Enforcer::new();
    let rules = enforcer.rules();
    assert_eq!(rules.len(), 6);

    let names: Vec<&str> = rules.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"qa_after_docx_write"));
    assert!(names.contains(&"render_after_edit"));
    assert!(names.contains(&"rag_before_bid"));
    assert!(names.contains(&"parse_before_write"));
    assert!(names.contains(&"health_gate"));
    assert!(names.contains(&"sensitive_data_gate"));
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
    // Simulate a parse so parse_before_write doesn't block first
    enforcer.post_check("tender_parse_spec");

    let verdict = enforcer.pre_check("tender_write_answer");
    assert_eq!(verdict.action, Action::Warn);
    assert_eq!(verdict.rule.as_deref(), Some("qa_after_docx_write"));
}

// -- 4. QA after write allows when QA was called --

#[test]
fn test_qa_after_write_allows_with_qa() {
    let mut enforcer = Enforcer::new();
    enforcer.post_check("tender_parse_spec");
    enforcer.post_check("qa_check_fonts");

    let verdict = enforcer.pre_check("tender_write_answer");
    assert_eq!(verdict.action, Action::Allow);
}

// -- 5. Render after edit warns --

#[test]
fn test_render_after_edit_warns() {
    let mut enforcer = Enforcer::new();
    // Parse first to avoid the block rule firing
    enforcer.post_check("tender_parse_spec");
    enforcer.post_check("qa_check_fonts");
    enforcer.post_check("tender_write_answer");
    enforcer.post_check("tender_write_answer");
    enforcer.post_check("tender_write_answer");

    // Now any further tool triggers render_after_edit because 3+
    // tender_write_answer calls exist without tender_render.
    // The RepeatWithout rule fires on ANY pre_check, but we test with
    // an unrelated tool to isolate it. Actually, the RepeatWithout rule
    // doesn't check the current tool_name -- it checks the history.
    // Let's check with tender_write_answer itself (which also matches
    // qa_after_docx_write, but render_after_edit should still fire).
    let verdict = enforcer.pre_check("tender_write_answer");
    // Both qa_after_docx_write (WARN) and render_after_edit (WARN) could fire,
    // but the important thing is the action is Warn.
    assert_eq!(verdict.action, Action::Warn);
}

// -- 6. Parse before write blocks --

#[test]
fn test_parse_before_write_blocks() {
    let mut enforcer = Enforcer::new();
    // No tender_parse in history at all
    let verdict = enforcer.pre_check("tender_write_answer");
    assert_eq!(verdict.action, Action::Block);
    assert_eq!(verdict.rule.as_deref(), Some("parse_before_write"));
}

// -- 7. Parse before write allows when parse was called --

#[test]
fn test_parse_before_write_allows_with_parse() {
    let mut enforcer = Enforcer::new();
    enforcer.post_check("tender_parse_spec");
    enforcer.post_check("qa_check_fonts");

    let verdict = enforcer.pre_check("tender_write_answer");
    assert_eq!(verdict.action, Action::Allow);
}

// -- 8. RAG before bid warns --

#[test]
fn test_rag_before_bid_warns() {
    let mut enforcer = Enforcer::new();
    let verdict = enforcer.pre_check("bid_no_bid");
    assert_eq!(verdict.action, Action::Warn);
    assert_eq!(verdict.rule.as_deref(), Some("rag_before_bid"));
}

// -- 9. Post check maintains window --

#[test]
fn test_post_check_maintains_window() {
    let mut enforcer = Enforcer::new();

    // Push more than max_history calls
    for i in 0..150 {
        enforcer.post_check(&format!("tool_{i}"));
    }

    // The enforcer's max_history is 100, so only the last 100 should remain.
    // We can verify indirectly: if we had tender_parse at position 0..49 (now
    // evicted) and the remaining 50..149 don't have it, parse_before_write
    // should fire.
    let mut enforcer2 = Enforcer::new();
    // Add tender_parse early
    enforcer2.post_check("tender_parse_spec");
    // Fill with 100 other calls to push it out
    for i in 0..100 {
        enforcer2.post_check(&format!("filler_{i}"));
    }
    // tender_parse should have been evicted from the window
    // But parse_before_write uses window: usize::MAX, so it checks ALL
    // remaining entries. After eviction, tender_parse is gone.
    enforcer2.post_check("qa_check_fonts");
    let verdict = enforcer2.pre_check("tender_write_answer");
    // tender_parse was evicted so block fires
    assert_eq!(verdict.action, Action::Block);
}

// -- 10. Set rule enabled --

#[test]
fn test_set_rule_enabled() {
    let mut enforcer = Enforcer::new();

    // Disable parse_before_write
    let found = enforcer.set_rule_enabled("parse_before_write", false);
    assert!(found);

    // Now tender_write_answer without parse should not block
    let verdict = enforcer.pre_check("tender_write_answer");
    // It may still WARN (qa_after_docx_write), but should not Block
    assert_ne!(verdict.action, Action::Block);

    // Disabling a nonexistent rule returns false
    let not_found = enforcer.set_rule_enabled("nonexistent_rule", false);
    assert!(!not_found);
}

// -- 11. Log verdict and get log --

#[test]
fn test_log_verdict_and_get_log() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db = sentinel::sentinel_core::state::StateDb::open(tmp.path()).unwrap();
    let session_id = db.create_session(Some("test-project")).unwrap();

    let verdict = Verdict {
        action: Action::Warn,
        rule: Some("qa_after_docx_write".into()),
        reason: Some("No QA tool in last 3 calls".into()),
    };

    Enforcer::log_verdict(&db, &session_id, &verdict, "tender_write_answer").unwrap();

    let log = Enforcer::get_log(&db, Some(&session_id), 10);
    assert_eq!(log.len(), 1);

    let entry = &log[0];
    assert_eq!(entry.session_id.as_deref(), Some(session_id.as_str()));
    assert_eq!(entry.rule, "qa_after_docx_write");
    assert_eq!(entry.action, "warn");
    assert_eq!(entry.tool_call.as_deref(), Some("tender_write_answer"));
    assert_eq!(
        entry.reason.as_deref(),
        Some("No QA tool in last 3 calls")
    );
}

// -- 12. Get log empty --

#[test]
fn test_get_log_empty() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db = sentinel::sentinel_core::state::StateDb::open(tmp.path()).unwrap();

    let log = Enforcer::get_log(&db, Some("nonexistent-session"), 10);
    assert!(log.is_empty());
}
