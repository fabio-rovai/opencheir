use opencheir::domain::qa::QaService;
use opencheir::store::documents::{DocumentService, ParsedDocument};

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

// ===========================================================================
// Tests: check_fonts
// ===========================================================================

#[test]
fn test_check_fonts_consistent() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new()
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("Hello")
                        .fonts(RunFonts::new().ascii("Arial")),
                ),
            )
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("World")
                        .fonts(RunFonts::new().ascii("Arial")),
                ),
            ),
    );

    let result = QaService::check_fonts(&doc);
    assert_eq!(result.primary_font, "Arial");
    assert_eq!(result.fonts.len(), 1);
    assert!(result.fonts[0].is_primary);
    assert!(
        result.inconsistencies.is_empty(),
        "Consistent fonts should have no inconsistencies"
    );
}

#[test]
fn test_check_fonts_inconsistent() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new()
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("Arial text")
                        .fonts(RunFonts::new().ascii("Arial")),
                ),
            )
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("More Arial")
                        .fonts(RunFonts::new().ascii("Arial")),
                ),
            )
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("Times text")
                        .fonts(RunFonts::new().ascii("Times New Roman")),
                ),
            ),
    );

    let result = QaService::check_fonts(&doc);
    assert_eq!(result.primary_font, "Arial");
    assert_eq!(result.fonts.len(), 2);
    assert_eq!(result.inconsistencies.len(), 1);
    assert_eq!(result.inconsistencies[0].issue_type, "font_inconsistency");
    assert!(result.inconsistencies[0].context.contains("Times New Roman"));
}

#[test]
fn test_check_fonts_empty_document() {
    let doc = build_and_parse(docx_rs::Docx::new());
    let result = QaService::check_fonts(&doc);
    assert!(result.primary_font.is_empty());
    assert!(result.fonts.is_empty());
    assert!(result.inconsistencies.is_empty());
}

// ===========================================================================
// Tests: check_dashes
// ===========================================================================

#[test]
fn test_check_dashes_finds_em_dashes() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new()
            .add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("This has an em dash \u{2014} here")),
            )
            .add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("And an en dash \u{2013} there")),
            ),
    );

    let result = QaService::check_dashes(&doc);
    assert_eq!(result.check_type, "dashes");
    assert_eq!(result.status, "warning");
    assert_eq!(result.issue_count, 2);

    let em = result
        .issues
        .iter()
        .find(|i| i.issue_type == "em_dash");
    assert!(em.is_some(), "Should find an em dash issue");

    let en = result
        .issues
        .iter()
        .find(|i| i.issue_type == "en_dash");
    assert!(en.is_some(), "Should find an en dash issue");
}

#[test]
fn test_check_dashes_clean_document() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_paragraph(
            Paragraph::new().add_run(Run::new().add_text("Normal text with a hyphen - here")),
        ),
    );

    let result = QaService::check_dashes(&doc);
    assert_eq!(result.status, "pass");
    assert_eq!(result.issue_count, 0);
    assert!(result.issues.is_empty());
}

// ===========================================================================
// Tests: check_word_counts
// ===========================================================================

#[test]
fn test_check_word_count_detects_limit_and_answer() {
    use docx_rs::*;

    // Row 0: "Word count: 50 words"
    // Row 1: answer with 30 words (within range)
    let answer_text = filler_words(30);

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![
            TableRow::new(vec![TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("Word count: 50 words")),
            )]),
            TableRow::new(vec![TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&answer_text)),
            )]),
        ])),
    );

    let result = QaService::check_word_counts(&doc);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].word_limit, 50);
    assert_eq!(result.cells[0].word_count, 30);
    assert_eq!(result.cells[0].status, "SIGNIFICANTLY_UNDER");
}

#[test]
fn test_check_word_count_over_limit() {
    use docx_rs::*;

    let answer_text = filler_words(60);

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![
            TableRow::new(vec![TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Maximum 50 words")),
            )]),
            TableRow::new(vec![TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&answer_text)),
            )]),
        ])),
    );

    let result = QaService::check_word_counts(&doc);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].word_limit, 50);
    assert_eq!(result.cells[0].word_count, 60);
    assert_eq!(result.cells[0].status, "OVER_LIMIT");
    let pct = result.cells[0].percentage;
    assert!((pct - 120.0).abs() < 0.1);
}

#[test]
fn test_check_word_count_optimal() {
    use docx_rs::*;

    // 45/50 = 90% -> OPTIMAL
    let answer_text = filler_words(45);

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![
            TableRow::new(vec![TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Limited to 50 words")),
            )]),
            TableRow::new(vec![TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&answer_text)),
            )]),
        ])),
    );

    let result = QaService::check_word_counts(&doc);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].status, "OPTIMAL");
}

#[test]
fn test_check_word_count_under_target() {
    use docx_rs::*;

    // 38/50 = 76% -> UNDER_TARGET
    let answer_text = filler_words(38);

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![
            TableRow::new(vec![TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Up to 50 words")),
            )]),
            TableRow::new(vec![TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&answer_text)),
            )]),
        ])),
    );

    let result = QaService::check_word_counts(&doc);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].status, "UNDER_TARGET");
}

#[test]
fn test_check_word_count_no_tables() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_paragraph(
            Paragraph::new().add_run(Run::new().add_text("No tables here")),
        ),
    );

    let result = QaService::check_word_counts(&doc);
    assert!(result.cells.is_empty());
}

#[test]
fn test_check_word_count_limit_pattern_variations() {
    use docx_rs::*;

    // Test "500 words max" pattern
    let answer_text = filler_words(25);

    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![
            TableRow::new(vec![TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("500 words maximum")),
            )]),
            TableRow::new(vec![TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&answer_text)),
            )]),
        ])),
    );

    let result = QaService::check_word_counts(&doc);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].word_limit, 500);
}

// ===========================================================================
// Tests: check_signatures
// ===========================================================================

#[test]
fn test_check_signatures_unsigned() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(docx_rs::Table::new(vec![TableRow::new(vec![
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("Signature")),
            ),
            TableCell::new()
                .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
        ])])),
    );

    let result = QaService::check_signatures(&doc);
    assert_eq!(result.check_type, "signatures");
    assert_eq!(result.status, "fail");
    assert_eq!(result.issue_count, 1);
    assert_eq!(result.issues[0].issue_type, "unsigned_field");
}

#[test]
fn test_check_signatures_signed() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(docx_rs::Table::new(vec![TableRow::new(vec![
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("Signed by")),
            ),
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("John Smith")),
            ),
        ])])),
    );

    let result = QaService::check_signatures(&doc);
    assert_eq!(result.status, "pass");
    assert!(result.issues.is_empty());
}

#[test]
fn test_check_signatures_no_signature_fields() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_table(docx_rs::Table::new(vec![TableRow::new(vec![
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("Name")),
            ),
            TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("Value")),
            ),
        ])])),
    );

    let result = QaService::check_signatures(&doc);
    assert_eq!(result.status, "pass");
    assert!(result.issues.is_empty());
}

// ===========================================================================
// Tests: check_smart_quotes
// ===========================================================================

#[test]
fn test_check_smart_quotes_found() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new()
            .add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("\u{201C}Hello\u{201D}")),
            )
            .add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("It\u{2019}s fine")),
            ),
    );

    let result = QaService::check_smart_quotes(&doc);
    assert_eq!(result.check_type, "smart_quotes");
    assert_eq!(result.status, "warning");
    assert!(result.issue_count >= 2);
}

#[test]
fn test_check_smart_quotes_clean() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text("\"Straight quotes\" and 'apostrophes'")),
        ),
    );

    let result = QaService::check_smart_quotes(&doc);
    assert_eq!(result.status, "pass");
    assert_eq!(result.issue_count, 0);
}

// ===========================================================================
// Tests: full_check
// ===========================================================================

#[test]
fn test_full_check_clean_document() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new()
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("Clean text")
                        .fonts(RunFonts::new().ascii("Arial")),
                ),
            )
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("More text")
                        .fonts(RunFonts::new().ascii("Arial")),
                ),
            ),
    );

    let result = QaService::full_check(&doc);

    assert!(result.critical_count == 0);
    assert!(result.ready_to_submit);
    // Should have checks for: fonts, dashes, smart_quotes, word_counts, signatures
    assert!(result.checks.len() >= 5);
}

#[test]
fn test_full_check_with_issues() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new()
            .add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Em dash \u{2014} present")),
            ),
    );

    let result = QaService::full_check(&doc);

    assert!(result.total_issues > 0);
}

#[test]
fn test_full_check_aggregates_correctly() {
    use docx_rs::*;

    // Create doc with mixed fonts (warning) + unsigned signature (critical)
    let doc = build_and_parse(
        Docx::new()
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("Arial text")
                        .fonts(RunFonts::new().ascii("Arial")),
                ),
            )
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("Times text")
                        .fonts(RunFonts::new().ascii("Times New Roman")),
                ),
            )
            .add_table(docx_rs::Table::new(vec![TableRow::new(vec![
                TableCell::new().add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text("Signature")),
                ),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(""))),
            ])])),
    );

    let result = QaService::full_check(&doc);

    // Font inconsistency (warning) + unsigned signature (critical)
    assert!(result.total_issues >= 2);
    assert!(result.critical_count >= 1);
    assert!(!result.ready_to_submit);
}
