use sentinel::sentinel_core::documents::{DocumentService, ParsedDocument};

// ---------------------------------------------------------------------------
// Test helper: build a DOCX in memory and parse it via DocumentService
// ---------------------------------------------------------------------------

fn build_and_parse(docx: docx_rs::Docx) -> ParsedDocument {
    let mut buf = Vec::new();
    docx.build().pack(&mut std::io::Cursor::new(&mut buf)).unwrap();
    DocumentService::parse_bytes(&buf, "test.docx").unwrap()
}

fn create_test_docx() -> ParsedDocument {
    use docx_rs::*;

    let docx = Docx::new()
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Hello World")))
        .add_paragraph(
            Paragraph::new()
                .style("Heading1")
                .add_run(Run::new().add_text("A Heading")),
        )
        .add_paragraph(
            Paragraph::new().add_run(Run::new().add_text("Second paragraph of text")),
        );

    build_and_parse(docx)
}

fn create_test_docx_with_table() -> ParsedDocument {
    use docx_rs::*;

    let docx = Docx::new()
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Before table")))
        .add_table(Table::new(vec![
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("R0C0"))),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("R0C1"))),
            ]),
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("R1C0"))),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text("R1C1"))),
            ]),
        ]))
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text("After table")));

    build_and_parse(docx)
}

fn create_test_docx_with_anomalies() -> ParsedDocument {
    use docx_rs::*;

    let docx = Docx::new()
        .add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text("This has an em dash \u{2014} here")),
        )
        .add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text("An en dash \u{2013} too")),
        )
        .add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text("\u{201C}Smart quotes\u{201D} present")),
        )
        .add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text("Single \u{2018}quotes\u{2019} also")),
        );

    build_and_parse(docx)
}

fn create_test_docx_with_fonts() -> ParsedDocument {
    use docx_rs::*;

    let docx = Docx::new()
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
        );

    build_and_parse(docx)
}

fn create_test_docx_with_formatting() -> ParsedDocument {
    use docx_rs::*;

    let docx = Docx::new()
        .add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text("Bold text").bold())
                .add_run(Run::new().add_text(" and "))
                .add_run(Run::new().add_text("italic text").italic())
                .add_run(Run::new().add_text(" and "))
                .add_run(Run::new().add_text("sized text").size(24)),
        );

    build_and_parse(docx)
}

// ---------------------------------------------------------------------------
// Tests: parsing
// ---------------------------------------------------------------------------

#[test]
fn test_parse_document_has_paragraphs() {
    let doc = create_test_docx();
    assert_eq!(doc.paragraphs.len(), 3);
    assert_eq!(doc.path, "test.docx");
}

#[test]
fn test_parse_document_paragraph_text() {
    let doc = create_test_docx();
    assert_eq!(doc.paragraphs[0].text, "Hello World");
    assert_eq!(doc.paragraphs[1].text, "A Heading");
    assert_eq!(doc.paragraphs[2].text, "Second paragraph of text");
}

#[test]
fn test_parse_document_paragraph_style() {
    let doc = create_test_docx();
    // First paragraph has no explicit style
    assert!(doc.paragraphs[0].style.is_none());
    // Second paragraph has Heading1 style
    assert_eq!(doc.paragraphs[1].style.as_deref(), Some("Heading1"));
}

#[test]
fn test_parse_document_runs() {
    let doc = create_test_docx();
    assert_eq!(doc.paragraphs[0].runs.len(), 1);
    assert_eq!(doc.paragraphs[0].runs[0].text, "Hello World");
}

// ---------------------------------------------------------------------------
// Tests: tables
// ---------------------------------------------------------------------------

#[test]
fn test_parse_table() {
    let doc = create_test_docx_with_table();
    assert_eq!(doc.tables.len(), 1);
    let table = &doc.tables[0];
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0].cells.len(), 2);
    assert_eq!(table.rows[1].cells.len(), 2);
}

#[test]
fn test_parse_table_cell_text() {
    let doc = create_test_docx_with_table();
    assert_eq!(doc.tables[0].rows[0].cells[0].text, "R0C0");
    assert_eq!(doc.tables[0].rows[0].cells[1].text, "R0C1");
    assert_eq!(doc.tables[0].rows[1].cells[0].text, "R1C0");
    assert_eq!(doc.tables[0].rows[1].cells[1].text, "R1C1");
}

#[test]
fn test_parse_table_cell_paragraphs() {
    let doc = create_test_docx_with_table();
    let cell = &doc.tables[0].rows[0].cells[0];
    assert_eq!(cell.paragraphs.len(), 1);
    assert_eq!(cell.paragraphs[0].text, "R0C0");
}

// ---------------------------------------------------------------------------
// Tests: extract_text
// ---------------------------------------------------------------------------

#[test]
fn test_extract_text() {
    let doc = create_test_docx();
    let text = DocumentService::extract_text(&doc);
    assert!(text.contains("Hello World"));
    assert!(text.contains("A Heading"));
    assert!(text.contains("Second paragraph of text"));
}

#[test]
fn test_extract_text_joins_with_newlines() {
    let doc = create_test_docx();
    let text = DocumentService::extract_text(&doc);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
}

// ---------------------------------------------------------------------------
// Tests: get_cell_text
// ---------------------------------------------------------------------------

#[test]
fn test_get_cell_text() {
    let doc = create_test_docx_with_table();
    assert_eq!(
        DocumentService::get_cell_text(&doc, 0, 0, 0),
        Some("R0C0")
    );
    assert_eq!(
        DocumentService::get_cell_text(&doc, 0, 1, 1),
        Some("R1C1")
    );
}

#[test]
fn test_get_cell_text_out_of_bounds() {
    let doc = create_test_docx_with_table();
    assert_eq!(DocumentService::get_cell_text(&doc, 5, 0, 0), None);
    assert_eq!(DocumentService::get_cell_text(&doc, 0, 10, 0), None);
    assert_eq!(DocumentService::get_cell_text(&doc, 0, 0, 10), None);
}

// ---------------------------------------------------------------------------
// Tests: word_count
// ---------------------------------------------------------------------------

#[test]
fn test_word_count_basic() {
    assert_eq!(DocumentService::word_count("hello world foo"), 3);
}

#[test]
fn test_word_count_empty() {
    assert_eq!(DocumentService::word_count(""), 0);
}

#[test]
fn test_word_count_whitespace_only() {
    assert_eq!(DocumentService::word_count("   \t\n  "), 0);
}

#[test]
fn test_word_count_single_word() {
    assert_eq!(DocumentService::word_count("hello"), 1);
}

#[test]
fn test_word_count_extra_whitespace() {
    assert_eq!(DocumentService::word_count("  hello   world  "), 2);
}

// ---------------------------------------------------------------------------
// Tests: extract_fonts
// ---------------------------------------------------------------------------

#[test]
fn test_extract_fonts() {
    let doc = create_test_docx_with_fonts();
    let fonts = DocumentService::extract_fonts(&doc);
    assert!(!fonts.is_empty());

    // Arial should appear twice
    let arial = fonts.iter().find(|f| f.font == "Arial");
    assert!(arial.is_some(), "Arial should be found in fonts");
    assert_eq!(arial.unwrap().count, 2);

    // Times New Roman should appear once
    let times = fonts.iter().find(|f| f.font == "Times New Roman");
    assert!(times.is_some(), "Times New Roman should be found in fonts");
    assert_eq!(times.unwrap().count, 1);
}

#[test]
fn test_extract_fonts_empty_doc() {
    let doc = build_and_parse(docx_rs::Docx::new());
    let fonts = DocumentService::extract_fonts(&doc);
    assert!(fonts.is_empty());
}

// ---------------------------------------------------------------------------
// Tests: find_anomalies
// ---------------------------------------------------------------------------

#[test]
fn test_find_anomalies_em_dash() {
    let doc = create_test_docx_with_anomalies();
    let anomalies = DocumentService::find_anomalies(&doc);
    let em_dashes: Vec<_> = anomalies
        .iter()
        .filter(|a| a.anomaly_type == "em_dash")
        .collect();
    assert!(!em_dashes.is_empty(), "Should detect em dashes");
    assert!(em_dashes[0].location.contains("paragraph"));
}

#[test]
fn test_find_anomalies_en_dash() {
    let doc = create_test_docx_with_anomalies();
    let anomalies = DocumentService::find_anomalies(&doc);
    let en_dashes: Vec<_> = anomalies
        .iter()
        .filter(|a| a.anomaly_type == "en_dash")
        .collect();
    assert!(!en_dashes.is_empty(), "Should detect en dashes");
}

#[test]
fn test_find_anomalies_smart_quotes() {
    let doc = create_test_docx_with_anomalies();
    let anomalies = DocumentService::find_anomalies(&doc);

    let open_double: Vec<_> = anomalies
        .iter()
        .filter(|a| a.anomaly_type == "smart_quote_open_double")
        .collect();
    assert!(
        !open_double.is_empty(),
        "Should detect smart double open quotes"
    );

    let close_double: Vec<_> = anomalies
        .iter()
        .filter(|a| a.anomaly_type == "smart_quote_close_double")
        .collect();
    assert!(
        !close_double.is_empty(),
        "Should detect smart double close quotes"
    );

    let open_single: Vec<_> = anomalies
        .iter()
        .filter(|a| a.anomaly_type == "smart_quote_open_single")
        .collect();
    assert!(
        !open_single.is_empty(),
        "Should detect smart single open quotes"
    );

    let close_single: Vec<_> = anomalies
        .iter()
        .filter(|a| a.anomaly_type == "smart_quote_close_single")
        .collect();
    assert!(
        !close_single.is_empty(),
        "Should detect smart single close quotes"
    );
}

#[test]
fn test_find_anomalies_clean_document() {
    use docx_rs::*;
    let doc = build_and_parse(
        Docx::new()
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Clean text here"))),
    );
    let anomalies = DocumentService::find_anomalies(&doc);
    assert!(anomalies.is_empty(), "Clean document should have no anomalies");
}

#[test]
fn test_find_anomalies_in_table() {
    use docx_rs::*;
    let doc = build_and_parse(
        Docx::new().add_table(Table::new(vec![TableRow::new(vec![
            TableCell::new().add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("Cell with em dash \u{2014} here")),
            ),
        ])])),
    );
    let anomalies = DocumentService::find_anomalies(&doc);
    assert!(!anomalies.is_empty());
    assert!(anomalies[0].location.contains("table"));
}

// ---------------------------------------------------------------------------
// Tests: formatting (bold, italic, size)
// ---------------------------------------------------------------------------

#[test]
fn test_parse_bold_run() {
    let doc = create_test_docx_with_formatting();
    let para = &doc.paragraphs[0];
    // First run is bold
    assert!(para.runs[0].bold, "First run should be bold");
    assert!(!para.runs[0].italic, "First run should not be italic");
}

#[test]
fn test_parse_italic_run() {
    let doc = create_test_docx_with_formatting();
    let para = &doc.paragraphs[0];
    // Third run (index 2) is italic
    assert!(para.runs[2].italic, "Third run should be italic");
    assert!(!para.runs[2].bold, "Third run should not be bold");
}

#[test]
fn test_parse_sized_run() {
    let doc = create_test_docx_with_formatting();
    let para = &doc.paragraphs[0];
    // Fifth run (index 4) has size 24 half-points = 12 points
    assert_eq!(
        para.runs[4].font_size,
        Some(12.0),
        "Fifth run should be 12pt (24 half-points)"
    );
}

// ---------------------------------------------------------------------------
// Tests: parse from file (roundtrip through disk)
// ---------------------------------------------------------------------------

#[test]
fn test_parse_from_file() {
    use docx_rs::*;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_roundtrip.docx");

    // Write a DOCX to disk
    let docx = Docx::new()
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text("File test")));
    let mut file = std::fs::File::create(&path).unwrap();
    docx.build().pack(&mut file).unwrap();

    // Parse from disk
    let doc = DocumentService::parse(path.to_str().unwrap()).unwrap();
    assert_eq!(doc.paragraphs[0].text, "File test");
    assert_eq!(doc.path, path.to_str().unwrap());
}

#[test]
fn test_parse_nonexistent_file() {
    let result = DocumentService::parse("/nonexistent/path/file.docx");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Tests: multiple runs in single paragraph
// ---------------------------------------------------------------------------

#[test]
fn test_paragraph_multiple_runs_concatenated() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new().add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text("Hello "))
                .add_run(Run::new().add_text("World")),
        ),
    );
    assert_eq!(doc.paragraphs[0].text, "Hello World");
    assert_eq!(doc.paragraphs[0].runs.len(), 2);
}

// ---------------------------------------------------------------------------
// Tests: fonts in table cells
// ---------------------------------------------------------------------------

#[test]
fn test_extract_fonts_includes_table_cells() {
    use docx_rs::*;

    let doc = build_and_parse(
        Docx::new()
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("Body text")
                        .fonts(RunFonts::new().ascii("Calibri")),
                ),
            )
            .add_table(Table::new(vec![TableRow::new(vec![
                TableCell::new().add_paragraph(
                    Paragraph::new().add_run(
                        Run::new()
                            .add_text("Table text")
                            .fonts(RunFonts::new().ascii("Calibri")),
                    ),
                ),
            ])])),
    );

    let fonts = DocumentService::extract_fonts(&doc);
    let calibri = fonts.iter().find(|f| f.font == "Calibri");
    assert!(calibri.is_some());
    // Should count both paragraph and table cell occurrences
    assert_eq!(calibri.unwrap().count, 2);
}

// ---------------------------------------------------------------------------
// Tests: word count on extracted document text
// ---------------------------------------------------------------------------

#[test]
fn test_word_count_on_extracted_text() {
    let doc = create_test_docx();
    let text = DocumentService::extract_text(&doc);
    let count = DocumentService::word_count(&text);
    // "Hello World" + "A Heading" + "Second paragraph of text" = 2+2+4 = 8
    assert_eq!(count, 8);
}

// ---------------------------------------------------------------------------
// Tests: empty document
// ---------------------------------------------------------------------------

#[test]
fn test_empty_document() {
    let doc = build_and_parse(docx_rs::Docx::new());
    assert!(doc.paragraphs.is_empty());
    assert!(doc.tables.is_empty());
    assert_eq!(DocumentService::extract_text(&doc), "");
    assert_eq!(DocumentService::word_count(&DocumentService::extract_text(&doc)), 0);
}

// ---------------------------------------------------------------------------
// Tests: cell with multiple paragraphs
// ---------------------------------------------------------------------------

#[test]
fn test_cell_multiple_paragraphs() {
    use docx_rs::*;

    let doc = build_and_parse(Docx::new().add_table(Table::new(vec![TableRow::new(vec![
        TableCell::new()
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Line one")))
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Line two"))),
    ])])));

    let cell = &doc.tables[0].rows[0].cells[0];
    assert_eq!(cell.paragraphs.len(), 2);
    assert_eq!(cell.text, "Line one\nLine two");
}
