use sentinel::domain::tender::{
    QuestionType, TenderQuestion, TenderService, TenderStructure,
};
use sentinel::sentinel_core::documents::{DocumentService, ParsedDocument};

// ---------------------------------------------------------------------------
// Test helper: build a DOCX in memory and parse it via DocumentService
// ---------------------------------------------------------------------------

fn build_and_parse(docx: docx_rs::Docx) -> ParsedDocument {
    let mut buf = Vec::new();
    docx.build()
        .pack(&mut std::io::Cursor::new(&mut buf))
        .unwrap();
    DocumentService::parse_bytes(&buf, "test.docx").unwrap()
}

// ---------------------------------------------------------------------------
// Helper: generate N words of filler text
// ---------------------------------------------------------------------------

fn filler_words(n: usize) -> String {
    (0..n)
        .map(|i| format!("word{}", i))
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// Helper: create a tender-like DOCX with questions in tables
// ---------------------------------------------------------------------------

fn create_test_tender() -> ParsedDocument {
    use docx_rs::*;

    let docx = Docx::new()
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Test Tender Document")))
        .add_table(Table::new(vec![
            // Header row (not a question)
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Ref"))),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Question"))),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Marks"))),
            ]),
            // Scored question 5.1
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("5.1"))),
                TableCell::new().add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text(
                        "Describe your approach to project management. This will be scored. Word count: Limited to 500 words.",
                    )),
                ),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("20%"))),
            ]),
            // Answer row for 5.1
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
                TableCell::new().add_paragraph(
                    Paragraph::new()
                        .add_run(Run::new().add_text(&filler_words(450))),
                ),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
            ]),
            // Scored question 5.2
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("5.2"))),
                TableCell::new().add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text(
                        "Explain your quality assurance process. Scoring applies. Maximum 300 words.",
                    )),
                ),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("15%"))),
            ]),
            // Answer row for 5.2
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
                TableCell::new().add_paragraph(
                    Paragraph::new()
                        .add_run(Run::new().add_text(&filler_words(200))),
                ),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
            ]),
        ]))
        .add_table(Table::new(vec![
            // Pass/fail question 6.1
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("6.1"))),
                TableCell::new().add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text(
                        "Please confirm you hold the required certifications. Pass/Fail.",
                    )),
                ),
            ]),
            // Answer row for 6.1
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
                TableCell::new().add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text(
                        "We confirm we hold ISO 9001, ISO 14001, and ISO 27001 certifications as required.",
                    )),
                ),
            ]),
            // Pass/fail question 6.2 (unanswered)
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("6.2"))),
                TableCell::new().add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text(
                        "Confirm your company has relevant insurance coverage. Pass/Fail.",
                    )),
                ),
            ]),
            // Empty answer row for 6.2
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
            ]),
        ]));

    build_and_parse(docx)
}

// ---------------------------------------------------------------------------
// Helper: create a TenderStructure with known data for compliance tests
// ---------------------------------------------------------------------------

fn create_test_structure() -> TenderStructure {
    TenderStructure {
        filename: "test.docx".to_string(),
        title: Some("Test Tender".to_string()),
        total_tables: 1,
        total_sections: 2,
        questions: vec![
            // Scored, optimal (90% of 500 = 450)
            TenderQuestion {
                id: "5.1".to_string(),
                section: "5".to_string(),
                text: "Describe your approach.".to_string(),
                question_type: QuestionType::Scored,
                word_limit: Some(500),
                marks: Some("20%".to_string()),
                table_index: 0,
                row_index: 1,
                cell_index: 0,
                answer_text: Some(filler_words(450)),
                current_word_count: 450,
            },
            // Scored, over limit
            TenderQuestion {
                id: "5.2".to_string(),
                section: "5".to_string(),
                text: "Explain your quality process.".to_string(),
                question_type: QuestionType::Scored,
                word_limit: Some(300),
                marks: Some("15%".to_string()),
                table_index: 0,
                row_index: 3,
                cell_index: 0,
                answer_text: Some(filler_words(350)),
                current_word_count: 350,
            },
            // Scored, under target (75% of 400 = 300)
            TenderQuestion {
                id: "5.3".to_string(),
                section: "5".to_string(),
                text: "Outline your risk management.".to_string(),
                question_type: QuestionType::Scored,
                word_limit: Some(400),
                marks: Some("10%".to_string()),
                table_index: 0,
                row_index: 5,
                cell_index: 0,
                answer_text: Some(filler_words(300)),
                current_word_count: 300,
            },
            // Scored, significantly under (50% of 200 = 100)
            TenderQuestion {
                id: "5.4".to_string(),
                section: "5".to_string(),
                text: "Describe team qualifications.".to_string(),
                question_type: QuestionType::Scored,
                word_limit: Some(200),
                marks: Some("5%".to_string()),
                table_index: 0,
                row_index: 7,
                cell_index: 0,
                answer_text: Some(filler_words(100)),
                current_word_count: 100,
            },
            // Scored, no limit
            TenderQuestion {
                id: "5.5".to_string(),
                section: "5".to_string(),
                text: "Provide additional information.".to_string(),
                question_type: QuestionType::Scored,
                word_limit: None,
                marks: Some("5%".to_string()),
                table_index: 0,
                row_index: 9,
                cell_index: 0,
                answer_text: Some(filler_words(50)),
                current_word_count: 50,
            },
            // PassFail, answered
            TenderQuestion {
                id: "6.1".to_string(),
                section: "6".to_string(),
                text: "Confirm certifications. Pass/Fail.".to_string(),
                question_type: QuestionType::PassFail,
                word_limit: None,
                marks: None,
                table_index: 1,
                row_index: 0,
                cell_index: 0,
                answer_text: Some("We confirm we hold all required certifications.".to_string()),
                current_word_count: 8,
            },
            // PassFail, empty
            TenderQuestion {
                id: "6.2".to_string(),
                section: "6".to_string(),
                text: "Confirm insurance coverage. Pass/Fail.".to_string(),
                question_type: QuestionType::PassFail,
                word_limit: None,
                marks: None,
                table_index: 1,
                row_index: 2,
                cell_index: 0,
                answer_text: None,
                current_word_count: 0,
            },
        ],
    }
}

// ===========================================================================
// Tests: question ID extraction
// ===========================================================================

#[test]
fn test_parse_tender_finds_questions() {
    let doc = create_test_tender();
    let structure = TenderService::parse_tender(&doc);
    assert!(
        !structure.questions.is_empty(),
        "Should find at least one question"
    );
}

#[test]
fn test_parse_tender_filename() {
    let doc = create_test_tender();
    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.filename, "test.docx");
}

#[test]
fn test_parse_tender_title() {
    let doc = create_test_tender();
    let structure = TenderService::parse_tender(&doc);
    assert_eq!(
        structure.title.as_deref(),
        Some("Test Tender Document")
    );
}

#[test]
fn test_parse_tender_total_tables() {
    let doc = create_test_tender();
    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.total_tables, 2);
}

#[test]
fn test_question_id_simple_dotted() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("5.4"))),
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Some question"))),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions.len(), 1);
    assert_eq!(structure.questions[0].id, "5.4");
}

#[test]
fn test_question_id_with_sub_letter() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("6.1(a)"))),
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("Sub-question text")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions.len(), 1);
    assert_eq!(structure.questions[0].id, "6.1(a)");
}

#[test]
fn test_question_id_multi_level() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("1.2.3"))),
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("Multi-level question")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions.len(), 1);
    assert_eq!(structure.questions[0].id, "1.2.3");
}

#[test]
fn test_question_id_with_hyphen() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("3-1(b)"))),
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("Hyphenated question")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions.len(), 1);
    assert_eq!(structure.questions[0].id, "3-1(b)");
}

#[test]
fn test_no_question_id_in_plain_text() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("Not a question ID")),
            ),
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Some text"))),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert!(
        structure.questions.is_empty(),
        "Should not find questions in plain text"
    );
}

// ===========================================================================
// Tests: word limit extraction
// ===========================================================================

#[test]
fn test_word_limit_limited_to_pattern() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("7.1"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Describe X. Limited to 500 words.")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions[0].word_limit, Some(500));
}

#[test]
fn test_word_limit_maximum_pattern() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("7.2"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Describe Y. Maximum 300 words.")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions[0].word_limit, Some(300));
}

#[test]
fn test_word_limit_words_max_pattern() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("7.3"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Describe Z. 200 words maximum.")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions[0].word_limit, Some(200));
}

#[test]
fn test_word_limit_up_to_pattern() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("7.4"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Describe W. Up to 750 words.")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions[0].word_limit, Some(750));
}

#[test]
fn test_word_limit_word_count_pattern() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("7.5"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Describe V. Word count: 1000 words.")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions[0].word_limit, Some(1000));
}

#[test]
fn test_no_word_limit() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("7.6"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Describe U. No limit specified.")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions[0].word_limit, None);
}

// ===========================================================================
// Tests: question type detection
// ===========================================================================

#[test]
fn test_question_type_pass_fail() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("8.1"))),
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(
                    Run::new().add_text("Confirm you meet the requirements. Pass/Fail."),
                ),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions[0].question_type, QuestionType::PassFail);
}

#[test]
fn test_question_type_scored() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("8.2"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("This will be scored. Describe your method.")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions[0].question_type, QuestionType::Scored);
}

#[test]
fn test_question_type_scored_via_scoring_keyword() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("8.3"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Scoring will be applied to this response.")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(structure.questions[0].question_type, QuestionType::Scored);
}

#[test]
fn test_question_type_declaration() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("8.4"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Please confirm you agree to the terms.")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(
        structure.questions[0].question_type,
        QuestionType::Declaration
    );
}

#[test]
fn test_question_type_declaration_i_confirm() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("8.5"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("I confirm that we comply with the policy.")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(
        structure.questions[0].question_type,
        QuestionType::Declaration
    );
}

#[test]
fn test_question_type_information_default() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("8.6"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Provide your company registration number.")),
            ),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert_eq!(
        structure.questions[0].question_type,
        QuestionType::Information
    );
}

#[test]
fn test_question_type_upgraded_by_marks() {
    use docx_rs::*;

    // Question text has no scoring keywords but has marks in a third cell
    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("8.7"))),
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Describe your approach to delivery.")),
            ),
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("25%"))),
        ])])),
    );

    let structure = TenderService::parse_tender(&doc);
    // Should be upgraded from Information to Scored because marks present
    assert_eq!(structure.questions[0].question_type, QuestionType::Scored);
    assert_eq!(structure.questions[0].marks, Some("25%".to_string()));
}

// ===========================================================================
// Tests: check_compliance
// ===========================================================================

#[test]
fn test_check_compliance_optimal() {
    let structure = create_test_structure();
    let results = TenderService::check_compliance(&structure);

    // Question 5.1: 450/500 = 90% -> OPTIMAL
    let q51 = results.iter().find(|r| r.question_id == "5.1").unwrap();
    assert_eq!(q51.status, "OPTIMAL");
    assert_eq!(q51.word_count, 450);
    assert_eq!(q51.word_limit, Some(500));
    let pct = q51.usage_pct.unwrap();
    assert!((pct - 90.0).abs() < 0.1);
}

#[test]
fn test_check_compliance_over_limit() {
    let structure = create_test_structure();
    let results = TenderService::check_compliance(&structure);

    // Question 5.2: 350/300 = 116.7% -> OVER_LIMIT
    let q52 = results.iter().find(|r| r.question_id == "5.2").unwrap();
    assert_eq!(q52.status, "OVER_LIMIT");
    assert_eq!(q52.word_count, 350);
}

#[test]
fn test_check_compliance_under_target() {
    let structure = create_test_structure();
    let results = TenderService::check_compliance(&structure);

    // Question 5.3: 300/400 = 75% -> UNDER_TARGET
    let q53 = results.iter().find(|r| r.question_id == "5.3").unwrap();
    assert_eq!(q53.status, "UNDER_TARGET");
}

#[test]
fn test_check_compliance_significantly_under() {
    let structure = create_test_structure();
    let results = TenderService::check_compliance(&structure);

    // Question 5.4: 100/200 = 50% -> SIGNIFICANTLY_UNDER
    let q54 = results.iter().find(|r| r.question_id == "5.4").unwrap();
    assert_eq!(q54.status, "SIGNIFICANTLY_UNDER");
}

#[test]
fn test_check_compliance_no_limit() {
    let structure = create_test_structure();
    let results = TenderService::check_compliance(&structure);

    // Question 5.5: no limit -> NO_LIMIT
    let q55 = results.iter().find(|r| r.question_id == "5.5").unwrap();
    assert_eq!(q55.status, "NO_LIMIT");
    assert!(q55.usage_pct.is_none());
}

#[test]
fn test_check_compliance_only_scored() {
    let structure = create_test_structure();
    let results = TenderService::check_compliance(&structure);

    // Should only have results for scored questions (5.1 - 5.5), not pass/fail (6.1, 6.2)
    assert_eq!(results.len(), 5);
    for r in &results {
        assert!(r.question_id.starts_with("5."));
    }
}

// ===========================================================================
// Tests: check_pass_fail
// ===========================================================================

#[test]
fn test_check_pass_fail_answered() {
    let structure = create_test_structure();
    let results = TenderService::check_pass_fail(&structure);

    let q61 = results.iter().find(|r| r.question_id == "6.1").unwrap();
    assert_eq!(q61.status, "ANSWERED");
    assert!(q61.preview.is_some());
}

#[test]
fn test_check_pass_fail_empty() {
    let structure = create_test_structure();
    let results = TenderService::check_pass_fail(&structure);

    let q62 = results.iter().find(|r| r.question_id == "6.2").unwrap();
    assert_eq!(q62.status, "EMPTY");
}

#[test]
fn test_check_pass_fail_only_pass_fail_questions() {
    let structure = create_test_structure();
    let results = TenderService::check_pass_fail(&structure);

    // Should only have results for pass/fail questions (6.1, 6.2)
    assert_eq!(results.len(), 2);
    for r in &results {
        assert!(r.question_id.starts_with("6."));
    }
}

#[test]
fn test_check_pass_fail_short_answer_is_empty() {
    // An answer with <= 10 characters should be treated as EMPTY
    let mut structure = create_test_structure();
    // Override 6.1 answer to be very short
    structure.questions[5].answer_text = Some("Yes".to_string());

    let results = TenderService::check_pass_fail(&structure);
    let q61 = results.iter().find(|r| r.question_id == "6.1").unwrap();
    assert_eq!(q61.status, "EMPTY");
}

#[test]
fn test_check_pass_fail_preview_truncated() {
    let mut structure = create_test_structure();
    // Give a very long answer
    structure.questions[5].answer_text = Some(
        "This is a very long answer that should be truncated in the preview because it exceeds eighty characters in total length for display purposes."
            .to_string(),
    );

    let results = TenderService::check_pass_fail(&structure);
    let q61 = results.iter().find(|r| r.question_id == "6.1").unwrap();
    let preview = q61.preview.as_ref().unwrap();
    assert!(preview.ends_with("..."));
    // 80 chars + "..." = 83
    assert!(preview.len() <= 83);
}

// ===========================================================================
// Tests: check_submission_files
// ===========================================================================

#[test]
fn test_check_submission_files_found() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();

    // Create test files
    std::fs::write(dir.path().join("proposal.docx"), "test").unwrap();
    std::fs::write(dir.path().join("pricing.xlsx"), "test").unwrap();

    let expected = vec![
        ("Section 1".to_string(), "proposal.docx".to_string()),
        ("Section 2".to_string(), "pricing.xlsx".to_string()),
    ];

    let results = TenderService::check_submission_files(dir_path, &expected);
    assert_eq!(results.len(), 2);
    assert!(results[0].found);
    assert_eq!(results[0].status, "FOUND");
    assert!(results[1].found);
    assert_eq!(results[1].status, "FOUND");
}

#[test]
fn test_check_submission_files_missing() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();

    // Create only one file
    std::fs::write(dir.path().join("proposal.docx"), "test").unwrap();

    let expected = vec![
        ("Section 1".to_string(), "proposal.docx".to_string()),
        ("Section 2".to_string(), "missing.xlsx".to_string()),
    ];

    let results = TenderService::check_submission_files(dir_path, &expected);
    assert_eq!(results.len(), 2);
    assert!(results[0].found);
    assert_eq!(results[0].status, "FOUND");
    assert!(!results[1].found);
    assert_eq!(results[1].status, "MISSING");
}

#[test]
fn test_check_submission_files_empty_folder() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();

    let expected = vec![
        ("Section 1".to_string(), "proposal.docx".to_string()),
    ];

    let results = TenderService::check_submission_files(dir_path, &expected);
    assert_eq!(results.len(), 1);
    assert!(!results[0].found);
    assert_eq!(results[0].status, "MISSING");
}

#[test]
fn test_check_submission_files_preserves_section() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();

    std::fs::write(dir.path().join("doc.pdf"), "test").unwrap();

    let expected = vec![
        ("Quality Submission".to_string(), "doc.pdf".to_string()),
    ];

    let results = TenderService::check_submission_files(dir_path, &expected);
    assert_eq!(results[0].section, "Quality Submission");
    assert_eq!(results[0].filename, "doc.pdf");
}

// ===========================================================================
// Tests: full_check
// ===========================================================================

#[test]
fn test_full_check_ready_to_submit() {
    // Create a structure where everything is fine
    let structure = TenderStructure {
        filename: "test.docx".to_string(),
        title: Some("Test".to_string()),
        total_tables: 1,
        total_sections: 1,
        questions: vec![
            TenderQuestion {
                id: "1.1".to_string(),
                section: "1".to_string(),
                text: "Question text".to_string(),
                question_type: QuestionType::Scored,
                word_limit: Some(500),
                marks: Some("20%".to_string()),
                table_index: 0,
                row_index: 0,
                cell_index: 0,
                answer_text: Some(filler_words(450)),
                current_word_count: 450,
            },
            TenderQuestion {
                id: "2.1".to_string(),
                section: "2".to_string(),
                text: "Pass/Fail question".to_string(),
                question_type: QuestionType::PassFail,
                word_limit: None,
                marks: None,
                table_index: 1,
                row_index: 0,
                cell_index: 0,
                answer_text: Some(
                    "We confirm all requirements are met and documented.".to_string(),
                ),
                current_word_count: 8,
            },
        ],
    };

    let result = TenderService::full_check(&structure, None, None);
    assert!(result.summary.ready_to_submit);
    assert_eq!(result.summary.over_limit, 0);
    assert_eq!(result.summary.empty_pass_fail, 0);
    assert_eq!(result.summary.optimal, 1);
}

#[test]
fn test_full_check_not_ready_over_limit() {
    let structure = TenderStructure {
        filename: "test.docx".to_string(),
        title: Some("Test".to_string()),
        total_tables: 1,
        total_sections: 1,
        questions: vec![TenderQuestion {
            id: "1.1".to_string(),
            section: "1".to_string(),
            text: "Question".to_string(),
            question_type: QuestionType::Scored,
            word_limit: Some(100),
            marks: Some("20%".to_string()),
            table_index: 0,
            row_index: 0,
            cell_index: 0,
            answer_text: Some(filler_words(150)),
            current_word_count: 150,
        }],
    };

    let result = TenderService::full_check(&structure, None, None);
    assert!(!result.summary.ready_to_submit);
    assert_eq!(result.summary.over_limit, 1);
}

#[test]
fn test_full_check_not_ready_empty_pass_fail() {
    let structure = TenderStructure {
        filename: "test.docx".to_string(),
        title: Some("Test".to_string()),
        total_tables: 1,
        total_sections: 1,
        questions: vec![TenderQuestion {
            id: "1.1".to_string(),
            section: "1".to_string(),
            text: "Confirm. Pass/Fail.".to_string(),
            question_type: QuestionType::PassFail,
            word_limit: None,
            marks: None,
            table_index: 0,
            row_index: 0,
            cell_index: 0,
            answer_text: None,
            current_word_count: 0,
        }],
    };

    let result = TenderService::full_check(&structure, None, None);
    assert!(!result.summary.ready_to_submit);
    assert_eq!(result.summary.empty_pass_fail, 1);
}

#[test]
fn test_full_check_with_file_checks() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();

    std::fs::write(dir.path().join("proposal.docx"), "test").unwrap();

    let structure = TenderStructure {
        filename: "test.docx".to_string(),
        title: Some("Test".to_string()),
        total_tables: 1,
        total_sections: 1,
        questions: vec![],
    };

    let expected = vec![
        ("Section 1".to_string(), "proposal.docx".to_string()),
        ("Section 2".to_string(), "missing.xlsx".to_string()),
    ];

    let result = TenderService::full_check(&structure, Some(dir_path), Some(&expected));
    assert_eq!(result.file_checks.len(), 2);
    assert_eq!(result.summary.missing_files, 1);
}

#[test]
fn test_full_check_summary_counts() {
    let structure = create_test_structure();
    let result = TenderService::full_check(&structure, None, None);

    assert_eq!(result.summary.optimal, 1);       // 5.1: 450/500 = 90%
    assert_eq!(result.summary.over_limit, 1);     // 5.2: 350/300 > 100%
    assert_eq!(result.summary.under_target, 1);   // 5.3: 300/400 = 75%
    assert_eq!(result.summary.significantly_under, 1); // 5.4: 100/200 = 50%
    assert_eq!(result.summary.empty_pass_fail, 1); // 6.2: no answer
    assert!(!result.summary.ready_to_submit);      // over_limit > 0 and empty > 0
}

#[test]
fn test_full_check_no_files_when_folder_none() {
    let structure = TenderStructure {
        filename: "test.docx".to_string(),
        title: None,
        total_tables: 0,
        total_sections: 0,
        questions: vec![],
    };

    let result = TenderService::full_check(&structure, None, None);
    assert!(result.file_checks.is_empty());
    assert_eq!(result.summary.missing_files, 0);
}

// ===========================================================================
// Tests: read_answer
// ===========================================================================

#[test]
fn test_read_answer_valid_cell() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("5.1"))),
                TableCell::new().add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text("Question text here")),
                ),
            ]),
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
                TableCell::new().add_paragraph(
                    Paragraph::new()
                        .add_run(Run::new().add_text("This is the answer content")),
                ),
            ]),
        ])),
    );

    let result = TenderService::read_answer(&doc, 0, 1, 1).unwrap();
    assert_eq!(result.text, "This is the answer content");
    assert_eq!(result.word_count, 5);
    assert_eq!(result.table_index, 0);
    assert_eq!(result.row_index, 1);
    assert_eq!(result.cell_index, 1);
}

#[test]
fn test_read_answer_out_of_bounds() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Only cell"))),
        ])])),
    );

    assert!(TenderService::read_answer(&doc, 0, 0, 5).is_none());
    assert!(TenderService::read_answer(&doc, 0, 5, 0).is_none());
    assert!(TenderService::read_answer(&doc, 5, 0, 0).is_none());
}

#[test]
fn test_read_answer_empty_cell() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
        ])])),
    );

    let result = TenderService::read_answer(&doc, 0, 0, 0).unwrap();
    assert_eq!(result.text, "");
    assert_eq!(result.word_count, 0);
}

// ===========================================================================
// Tests: section extraction
// ===========================================================================

#[test]
fn test_section_extraction_from_question_ids() {
    let doc = create_test_tender();
    let structure = TenderService::parse_tender(&doc);

    // Should have sections derived from question IDs
    assert!(structure.total_sections > 0);

    // Check that questions have section values
    for q in &structure.questions {
        assert!(!q.section.is_empty());
    }
}

// ===========================================================================
// Tests: marks extraction
// ===========================================================================

#[test]
fn test_marks_extraction() {
    let doc = create_test_tender();
    let structure = TenderService::parse_tender(&doc);

    // Find question 5.1 which has marks
    let q51 = structure.questions.iter().find(|q| q.id == "5.1");
    assert!(q51.is_some(), "Question 5.1 should be found");
    assert_eq!(q51.unwrap().marks, Some("20%".to_string()));
}

// ===========================================================================
// Tests: answer text extraction from DOCX
// ===========================================================================

#[test]
fn test_answer_text_found_in_row_below() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("9.1"))),
                TableCell::new().add_paragraph(
                    Paragraph::new()
                        .add_run(Run::new().add_text("Question text. Will be scored.")),
                ),
            ]),
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
                TableCell::new().add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text("The answer to the question.")),
                ),
            ]),
        ])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert!(!structure.questions.is_empty());
    let q = &structure.questions[0];
    assert!(q.answer_text.is_some());
    assert_eq!(
        q.answer_text.as_deref(),
        Some("The answer to the question.")
    );
}

#[test]
fn test_answer_text_not_found_when_row_below_empty() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("9.2"))),
                TableCell::new().add_paragraph(
                    Paragraph::new()
                        .add_run(Run::new().add_text("Question without answer.")),
                ),
            ]),
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
            ]),
        ])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert!(!structure.questions.is_empty());
    let q = &structure.questions[0];
    assert!(q.answer_text.is_none());
    assert_eq!(q.current_word_count, 0);
}

// ===========================================================================
// Tests: edge cases
// ===========================================================================

#[test]
fn test_empty_document_no_questions() {
    let doc = build_and_parse(docx_rs::Docx::new());
    let structure = TenderService::parse_tender(&doc);
    assert!(structure.questions.is_empty());
    assert_eq!(structure.total_tables, 0);
    assert_eq!(structure.total_sections, 0);
}

#[test]
fn test_table_without_question_ids() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Name"))),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Value"))),
            ]),
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Company"))),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Acme Ltd"))),
            ]),
        ])),
    );

    let structure = TenderService::parse_tender(&doc);
    assert!(structure.questions.is_empty());
}

#[test]
fn test_full_check_empty_structure_is_ready() {
    // No questions means nothing to fail on
    let structure = TenderStructure {
        filename: "empty.docx".to_string(),
        title: None,
        total_tables: 0,
        total_sections: 0,
        questions: vec![],
    };

    let result = TenderService::full_check(&structure, None, None);
    assert!(result.summary.ready_to_submit);
    assert_eq!(result.compliance.len(), 0);
    assert_eq!(result.pass_fail.len(), 0);
}

#[test]
fn test_compliance_exact_limit_is_optimal() {
    let structure = TenderStructure {
        filename: "test.docx".to_string(),
        title: None,
        total_tables: 1,
        total_sections: 1,
        questions: vec![TenderQuestion {
            id: "1.1".to_string(),
            section: "1".to_string(),
            text: "Question".to_string(),
            question_type: QuestionType::Scored,
            word_limit: Some(100),
            marks: None,
            table_index: 0,
            row_index: 0,
            cell_index: 0,
            answer_text: Some(filler_words(100)),
            current_word_count: 100,
        }],
    };

    let results = TenderService::check_compliance(&structure);
    // 100/100 = 100% -> OPTIMAL (>= 89%)
    assert_eq!(results[0].status, "OPTIMAL");
}

#[test]
fn test_compliance_at_89_pct_is_optimal() {
    let structure = TenderStructure {
        filename: "test.docx".to_string(),
        title: None,
        total_tables: 1,
        total_sections: 1,
        questions: vec![TenderQuestion {
            id: "1.1".to_string(),
            section: "1".to_string(),
            text: "Question".to_string(),
            question_type: QuestionType::Scored,
            word_limit: Some(100),
            marks: None,
            table_index: 0,
            row_index: 0,
            cell_index: 0,
            answer_text: Some(filler_words(89)),
            current_word_count: 89,
        }],
    };

    let results = TenderService::check_compliance(&structure);
    assert_eq!(results[0].status, "OPTIMAL");
}

#[test]
fn test_compliance_at_70_pct_is_under_target() {
    let structure = TenderStructure {
        filename: "test.docx".to_string(),
        title: None,
        total_tables: 1,
        total_sections: 1,
        questions: vec![TenderQuestion {
            id: "1.1".to_string(),
            section: "1".to_string(),
            text: "Question".to_string(),
            question_type: QuestionType::Scored,
            word_limit: Some(100),
            marks: None,
            table_index: 0,
            row_index: 0,
            cell_index: 0,
            answer_text: Some(filler_words(70)),
            current_word_count: 70,
        }],
    };

    let results = TenderService::check_compliance(&structure);
    assert_eq!(results[0].status, "UNDER_TARGET");
}

#[test]
fn test_compliance_at_69_pct_is_significantly_under() {
    let structure = TenderStructure {
        filename: "test.docx".to_string(),
        title: None,
        total_tables: 1,
        total_sections: 1,
        questions: vec![TenderQuestion {
            id: "1.1".to_string(),
            section: "1".to_string(),
            text: "Question".to_string(),
            question_type: QuestionType::Scored,
            word_limit: Some(100),
            marks: None,
            table_index: 0,
            row_index: 0,
            cell_index: 0,
            answer_text: Some(filler_words(69)),
            current_word_count: 69,
        }],
    };

    let results = TenderService::check_compliance(&structure);
    assert_eq!(results[0].status, "SIGNIFICANTLY_UNDER");
}
