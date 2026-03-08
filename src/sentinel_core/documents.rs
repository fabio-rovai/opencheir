use std::collections::HashMap;

use serde::Serialize;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ParsedDocument {
    pub path: String,
    pub paragraphs: Vec<ParsedParagraph>,
    pub tables: Vec<ParsedTable>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedParagraph {
    pub text: String,
    pub style: Option<String>,
    pub runs: Vec<ParsedRun>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedRun {
    pub text: String,
    pub font: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub font_size: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedTable {
    pub rows: Vec<ParsedRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedRow {
    pub cells: Vec<ParsedCell>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedCell {
    pub text: String,
    pub paragraphs: Vec<ParsedParagraph>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FontUsage {
    pub font: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TextAnomaly {
    pub anomaly_type: String,
    pub text: String,
    pub location: String,
}

// ---------------------------------------------------------------------------
// Internal helpers for extracting private fields via serde
// ---------------------------------------------------------------------------

/// Extract font name from a `RunFonts` by serialising to JSON and reading
/// the first non-null font field (ascii > hi_ansi > east_asia > cs > themes).
fn extract_font_name(fonts: &docx_rs::RunFonts) -> Option<String> {
    let json = serde_json::to_value(fonts).ok()?;
    let obj = json.as_object()?;
    // Priority order for the "main" font name
    for key in &[
        "ascii",
        "hiAnsi",
        "eastAsia",
        "cs",
        "asciiTheme",
        "hiAnsiTheme",
        "eastAsiaTheme",
        "csTheme",
    ] {
        if let Some(serde_json::Value::String(s)) = obj.get(*key) {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
    }
    None
}

/// Extract the `val` (half-points) from a `Sz` via serde.
fn extract_sz_val(sz: &docx_rs::Sz) -> Option<f64> {
    let json = serde_json::to_value(sz).ok()?;
    // Sz serialises directly as a u32 value
    json.as_f64()
}

/// Extract the boolean value from `Bold` via serde.
fn extract_bold_val(bold: &docx_rs::Bold) -> bool {
    serde_json::to_value(bold)
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Extract the boolean value from `Italic` via serde.
fn extract_italic_val(italic: &docx_rs::Italic) -> bool {
    serde_json::to_value(italic)
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Paragraph / Run extraction helpers
// ---------------------------------------------------------------------------

fn extract_run(run: &docx_rs::Run) -> ParsedRun {
    let mut text = String::new();
    for child in &run.children {
        if let docx_rs::RunChild::Text(t) = child {
            text.push_str(&t.text);
        }
    }

    let font = run
        .run_property
        .fonts
        .as_ref()
        .and_then(extract_font_name);

    let bold = run
        .run_property
        .bold
        .as_ref()
        .map(extract_bold_val)
        .unwrap_or(false);

    let italic = run
        .run_property
        .italic
        .as_ref()
        .map(extract_italic_val)
        .unwrap_or(false);

    // sz is in half-points; convert to points
    let font_size = run
        .run_property
        .sz
        .as_ref()
        .and_then(extract_sz_val)
        .map(|hp| hp / 2.0);

    ParsedRun {
        text,
        font,
        bold,
        italic,
        font_size,
    }
}

fn extract_paragraph(para: &docx_rs::Paragraph) -> ParsedParagraph {
    let mut runs = Vec::new();
    let mut full_text = String::new();

    for child in &para.children {
        if let docx_rs::ParagraphChild::Run(run) = child {
            let parsed = extract_run(run);
            full_text.push_str(&parsed.text);
            runs.push(parsed);
        }
    }

    let style = para.property.style.as_ref().map(|s| s.val.clone());

    ParsedParagraph {
        text: full_text,
        style,
        runs,
    }
}

fn extract_table(table: &docx_rs::Table) -> ParsedTable {
    let mut rows = Vec::new();

    for table_child in &table.rows {
        let docx_rs::TableChild::TableRow(row) = table_child;
        let mut cells = Vec::new();

        for row_child in &row.cells {
            let docx_rs::TableRowChild::TableCell(cell) = row_child;
            let mut cell_paragraphs = Vec::new();
            let mut cell_text = String::new();

            for content in &cell.children {
                if let docx_rs::TableCellContent::Paragraph(p) = content {
                    let parsed_para = extract_paragraph(p);
                    if !cell_text.is_empty() && !parsed_para.text.is_empty() {
                        cell_text.push('\n');
                    }
                    cell_text.push_str(&parsed_para.text);
                    cell_paragraphs.push(parsed_para);
                }
            }

            cells.push(ParsedCell {
                text: cell_text,
                paragraphs: cell_paragraphs,
            });
        }

        rows.push(ParsedRow { cells });
    }

    ParsedTable { rows }
}

// ---------------------------------------------------------------------------
// Anomaly scanning
// ---------------------------------------------------------------------------

struct AnomalyPattern {
    ch: char,
    anomaly_type: &'static str,
}

const ANOMALY_PATTERNS: &[AnomalyPattern] = &[
    AnomalyPattern {
        ch: '\u{2014}',
        anomaly_type: "em_dash",
    },
    AnomalyPattern {
        ch: '\u{2013}',
        anomaly_type: "en_dash",
    },
    AnomalyPattern {
        ch: '\u{201C}',
        anomaly_type: "smart_quote_open_double",
    },
    AnomalyPattern {
        ch: '\u{201D}',
        anomaly_type: "smart_quote_close_double",
    },
    AnomalyPattern {
        ch: '\u{2018}',
        anomaly_type: "smart_quote_open_single",
    },
    AnomalyPattern {
        ch: '\u{2019}',
        anomaly_type: "smart_quote_close_single",
    },
];

fn scan_text_for_anomalies(text: &str, location: &str, results: &mut Vec<TextAnomaly>) {
    for pattern in ANOMALY_PATTERNS {
        if text.contains(pattern.ch) {
            results.push(TextAnomaly {
                anomaly_type: pattern.anomaly_type.to_string(),
                text: text.to_string(),
                location: location.to_string(),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// DocumentService
// ---------------------------------------------------------------------------

pub struct DocumentService;

impl DocumentService {
    /// Parse a DOCX file from disk and return structured content.
    pub fn parse(path: &str) -> anyhow::Result<ParsedDocument> {
        let file = std::fs::read(path)?;
        Self::parse_bytes(&file, path)
    }

    /// Parse a DOCX from raw bytes (useful for tests that build DOCX in memory).
    pub fn parse_bytes(bytes: &[u8], path: &str) -> anyhow::Result<ParsedDocument> {
        let docx = docx_rs::read_docx(bytes)
            .map_err(|e| anyhow::anyhow!("Failed to read DOCX: {:?}", e))?;

        let mut paragraphs = Vec::new();
        let mut tables = Vec::new();

        for child in &docx.document.children {
            match child {
                docx_rs::DocumentChild::Paragraph(para) => {
                    paragraphs.push(extract_paragraph(para));
                }
                docx_rs::DocumentChild::Table(table) => {
                    tables.push(extract_table(table));
                }
                _ => {}
            }
        }

        Ok(ParsedDocument {
            path: path.to_string(),
            paragraphs,
            tables,
        })
    }

    /// Extract all plain text from a parsed document, joining paragraphs with
    /// newlines.
    pub fn extract_text(doc: &ParsedDocument) -> String {
        doc.paragraphs
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get text from a specific table cell by indices.
    pub fn get_cell_text(
        doc: &ParsedDocument,
        table_idx: usize,
        row_idx: usize,
        cell_idx: usize,
    ) -> Option<&str> {
        doc.tables
            .get(table_idx)
            .and_then(|t| t.rows.get(row_idx))
            .and_then(|r| r.cells.get(cell_idx))
            .map(|c| c.text.as_str())
    }

    /// Count words in the given text (splits on whitespace).
    pub fn word_count(text: &str) -> usize {
        text.split_whitespace().count()
    }

    /// Find all fonts used in the document, with occurrence counts.
    pub fn extract_fonts(doc: &ParsedDocument) -> Vec<FontUsage> {
        let mut counts: HashMap<String, usize> = HashMap::new();

        let all_runs = doc
            .paragraphs
            .iter()
            .flat_map(|p| p.runs.iter())
            .chain(
                doc.tables
                    .iter()
                    .flat_map(|t| t.rows.iter())
                    .flat_map(|r| r.cells.iter())
                    .flat_map(|c| c.paragraphs.iter())
                    .flat_map(|p| p.runs.iter()),
            );

        for run in all_runs {
            if let Some(ref font) = run.font {
                *counts.entry(font.clone()).or_insert(0) += 1;
            }
        }

        let mut result: Vec<FontUsage> = counts
            .into_iter()
            .map(|(font, count)| FontUsage { font, count })
            .collect();
        result.sort_by(|a, b| b.count.cmp(&a.count).then(a.font.cmp(&b.font)));
        result
    }

    /// Find text anomalies (em dashes, en dashes, smart quotes) across the
    /// entire document.
    pub fn find_anomalies(doc: &ParsedDocument) -> Vec<TextAnomaly> {
        let mut results = Vec::new();

        for (i, para) in doc.paragraphs.iter().enumerate() {
            let location = format!("paragraph {}", i);
            scan_text_for_anomalies(&para.text, &location, &mut results);
        }

        for (ti, table) in doc.tables.iter().enumerate() {
            for (ri, row) in table.rows.iter().enumerate() {
                for (ci, cell) in row.cells.iter().enumerate() {
                    let location = format!("table {}, row {}, cell {}", ti, ri, ci);
                    scan_text_for_anomalies(&cell.text, &location, &mut results);
                }
            }
        }

        results
    }
}
