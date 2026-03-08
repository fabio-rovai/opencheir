use regex::Regex;
use serde::Serialize;

use crate::sentinel_core::documents::{DocumentService, ParsedDocument};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct QaCheckResult {
    pub check_type: String,
    pub status: String, // "pass", "warning", "fail"
    pub issue_count: usize,
    pub issues: Vec<QaIssue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QaIssue {
    pub issue_type: String,
    pub location: String,
    pub context: String,
    pub severity: String, // "critical", "warning", "info"
}

#[derive(Debug, Clone, Serialize)]
pub struct FontCheckResult {
    pub primary_font: String,
    pub fonts: Vec<FontInfo>,
    pub inconsistencies: Vec<QaIssue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FontInfo {
    pub name: String,
    pub count: usize,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WordCountResult {
    pub cells: Vec<WordCountCell>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WordCountCell {
    pub location: String,
    pub word_limit: usize,
    pub word_count: usize,
    pub percentage: f64,
    pub status: String, // "OVER_LIMIT", "OPTIMAL", "UNDER_TARGET", "SIGNIFICANTLY_UNDER"
}

#[derive(Debug, Clone, Serialize)]
pub struct FullCheckResult {
    pub checks: Vec<QaCheckResult>,
    pub total_issues: usize,
    pub critical_count: usize,
    pub ready_to_submit: bool,
}

// ---------------------------------------------------------------------------
// QaService
// ---------------------------------------------------------------------------

pub struct QaService;

impl QaService {
    // -----------------------------------------------------------------------
    // check_fonts
    // -----------------------------------------------------------------------

    /// Analyse font usage in the document. Identifies the primary (most common)
    /// font and flags any other fonts as inconsistencies.
    pub fn check_fonts(doc: &ParsedDocument) -> FontCheckResult {
        let font_usages = DocumentService::extract_fonts(doc);

        if font_usages.is_empty() {
            return FontCheckResult {
                primary_font: String::new(),
                fonts: Vec::new(),
                inconsistencies: Vec::new(),
            };
        }

        // extract_fonts returns sorted by count descending, so first = primary
        let primary = &font_usages[0].font;

        let fonts: Vec<FontInfo> = font_usages
            .iter()
            .map(|fu| FontInfo {
                name: fu.font.clone(),
                count: fu.count,
                is_primary: fu.font == *primary,
            })
            .collect();

        let inconsistencies: Vec<QaIssue> = font_usages
            .iter()
            .filter(|fu| fu.font != *primary)
            .map(|fu| QaIssue {
                issue_type: "font_inconsistency".to_string(),
                location: "document".to_string(),
                context: format!(
                    "Font '{}' used {} time(s), expected primary font '{}'",
                    fu.font, fu.count, primary
                ),
                severity: "warning".to_string(),
            })
            .collect();

        FontCheckResult {
            primary_font: primary.clone(),
            fonts,
            inconsistencies,
        }
    }

    // -----------------------------------------------------------------------
    // check_dashes
    // -----------------------------------------------------------------------

    /// Find em dashes (U+2014) and en dashes (U+2013) in the document.
    pub fn check_dashes(doc: &ParsedDocument) -> QaCheckResult {
        let anomalies = DocumentService::find_anomalies(doc);

        let issues: Vec<QaIssue> = anomalies
            .iter()
            .filter(|a| a.anomaly_type == "em_dash" || a.anomaly_type == "en_dash")
            .map(|a| QaIssue {
                issue_type: a.anomaly_type.clone(),
                location: a.location.clone(),
                context: a.text.clone(),
                severity: "warning".to_string(),
            })
            .collect();

        let status = if issues.is_empty() {
            "pass".to_string()
        } else {
            "warning".to_string()
        };

        QaCheckResult {
            check_type: "dashes".to_string(),
            status,
            issue_count: issues.len(),
            issues,
        }
    }

    // -----------------------------------------------------------------------
    // check_word_counts
    // -----------------------------------------------------------------------

    /// Scan table cells for word-limit patterns and check answer lengths.
    pub fn check_word_counts(doc: &ParsedDocument) -> WordCountResult {
        let patterns = [
            r"(?i)(?:word\s*count|limited?\s*to|max(?:imum)?)\s*[:.]?\s*(\d+)\s*words?",
            r"(?i)(\d+)\s*words?\s*(?:max|limit|maximum)",
            r"(?i)up\s*to\s*(\d+)\s*words?",
        ];

        let regexes: Vec<Regex> = patterns.iter().map(|p| Regex::new(p).unwrap()).collect();

        let mut cells = Vec::new();

        for (ti, table) in doc.tables.iter().enumerate() {
            for (ri, row) in table.rows.iter().enumerate() {
                for (ci, cell) in row.cells.iter().enumerate() {
                    // Try to extract a word limit from this cell
                    let word_limit = Self::extract_word_limit(&cell.text, &regexes);
                    if let Some(limit) = word_limit {
                        // Look 1-3 rows below for an answer cell
                        if let Some(answer) =
                            Self::find_answer_cell(table, ri, ci, 1, 3)
                        {
                            let wc = DocumentService::word_count(&answer);
                            let percentage = if limit > 0 {
                                (wc as f64 / limit as f64) * 100.0
                            } else {
                                0.0
                            };
                            let status = Self::word_count_status(percentage);

                            cells.push(WordCountCell {
                                location: format!("table {}, row {}, cell {}", ti, ri, ci),
                                word_limit: limit,
                                word_count: wc,
                                percentage,
                                status,
                            });
                        }
                    }
                }
            }
        }

        WordCountResult { cells }
    }

    /// Try to extract a word limit number from the text using the compiled regexes.
    fn extract_word_limit(text: &str, regexes: &[Regex]) -> Option<usize> {
        for re in regexes {
            if let Some(caps) = re.captures(text) {
                if let Some(m) = caps.get(1) {
                    if let Ok(n) = m.as_str().parse::<usize>() {
                        return Some(n);
                    }
                }
            }
        }
        None
    }

    /// Look `min_offset` to `max_offset` rows below `row_idx` in the same column
    /// for a cell with >20 words (the answer cell).
    fn find_answer_cell(
        table: &crate::sentinel_core::documents::ParsedTable,
        row_idx: usize,
        cell_idx: usize,
        min_offset: usize,
        max_offset: usize,
    ) -> Option<String> {
        for offset in min_offset..=max_offset {
            let target_row = row_idx + offset;
            if let Some(row) = table.rows.get(target_row) {
                if let Some(cell) = row.cells.get(cell_idx) {
                    let wc = DocumentService::word_count(&cell.text);
                    if wc > 20 {
                        return Some(cell.text.clone());
                    }
                }
            }
        }
        None
    }

    /// Determine the status label for a word count percentage.
    fn word_count_status(percentage: f64) -> String {
        if percentage > 100.0 {
            "OVER_LIMIT".to_string()
        } else if percentage >= 89.0 {
            "OPTIMAL".to_string()
        } else if percentage >= 70.0 {
            "UNDER_TARGET".to_string()
        } else {
            "SIGNIFICANTLY_UNDER".to_string()
        }
    }

    // -----------------------------------------------------------------------
    // check_signatures
    // -----------------------------------------------------------------------

    /// Find signature fields in tables and check if they appear to be signed.
    pub fn check_signatures(doc: &ParsedDocument) -> QaCheckResult {
        let signature_keywords = [
            "signature",
            "signed by",
            "authorised signatory",
            "authorized signatory",
        ];

        let mut issues = Vec::new();

        for (ti, table) in doc.tables.iter().enumerate() {
            for (ri, row) in table.rows.iter().enumerate() {
                for (ci, cell) in row.cells.iter().enumerate() {
                    let cell_lower = cell.text.to_lowercase();
                    let is_signature_field = signature_keywords
                        .iter()
                        .any(|kw| cell_lower.contains(kw));

                    if is_signature_field {
                        // Check neighbouring cells for names / tick marks
                        let has_content = Self::check_nearby_cells_for_signature(
                            table, ri, ci,
                        );

                        if !has_content {
                            issues.push(QaIssue {
                                issue_type: "unsigned_field".to_string(),
                                location: format!(
                                    "table {}, row {}, cell {}",
                                    ti, ri, ci
                                ),
                                context: format!(
                                    "Signature field '{}' appears unsigned",
                                    cell.text.trim()
                                ),
                                severity: "critical".to_string(),
                            });
                        }
                    }
                }
            }
        }

        let status = if issues.is_empty() {
            "pass".to_string()
        } else {
            "fail".to_string()
        };

        QaCheckResult {
            check_type: "signatures".to_string(),
            status,
            issue_count: issues.len(),
            issues,
        }
    }

    /// Check cells adjacent to the signature label for actual content (a name
    /// or tick mark).
    fn check_nearby_cells_for_signature(
        table: &crate::sentinel_core::documents::ParsedTable,
        row_idx: usize,
        cell_idx: usize,
    ) -> bool {
        // Check the cell to the right on the same row
        if let Some(row) = table.rows.get(row_idx) {
            if let Some(cell) = row.cells.get(cell_idx + 1) {
                let trimmed = cell.text.trim();
                if !trimmed.is_empty() {
                    return true;
                }
            }
        }

        // Check the cell below in the same column
        if let Some(row) = table.rows.get(row_idx + 1) {
            if let Some(cell) = row.cells.get(cell_idx) {
                let trimmed = cell.text.trim();
                if !trimmed.is_empty() {
                    return true;
                }
            }
        }

        false
    }

    // -----------------------------------------------------------------------
    // check_smart_quotes
    // -----------------------------------------------------------------------

    /// Find curly/smart quotes in the document.
    pub fn check_smart_quotes(doc: &ParsedDocument) -> QaCheckResult {
        let anomalies = DocumentService::find_anomalies(doc);

        let issues: Vec<QaIssue> = anomalies
            .iter()
            .filter(|a| a.anomaly_type.starts_with("smart_quote"))
            .map(|a| QaIssue {
                issue_type: a.anomaly_type.clone(),
                location: a.location.clone(),
                context: a.text.clone(),
                severity: "warning".to_string(),
            })
            .collect();

        let status = if issues.is_empty() {
            "pass".to_string()
        } else {
            "warning".to_string()
        };

        QaCheckResult {
            check_type: "smart_quotes".to_string(),
            status,
            issue_count: issues.len(),
            issues,
        }
    }

    // -----------------------------------------------------------------------
    // full_check
    // -----------------------------------------------------------------------

    /// Run all QA checks on a document and aggregate results.
    pub fn full_check(doc: &ParsedDocument) -> FullCheckResult {
        let mut checks = Vec::new();

        // Font check — convert to QaCheckResult
        let font_result = Self::check_fonts(doc);
        checks.push(QaCheckResult {
            check_type: "fonts".to_string(),
            status: if font_result.inconsistencies.is_empty() {
                "pass".to_string()
            } else {
                "warning".to_string()
            },
            issue_count: font_result.inconsistencies.len(),
            issues: font_result.inconsistencies,
        });

        // Dashes
        checks.push(Self::check_dashes(doc));

        // Smart quotes
        checks.push(Self::check_smart_quotes(doc));

        // Word counts — convert to QaCheckResult
        let wc_result = Self::check_word_counts(doc);
        let wc_issues: Vec<QaIssue> = wc_result
            .cells
            .iter()
            .filter(|c| c.status == "OVER_LIMIT")
            .map(|c| QaIssue {
                issue_type: "word_count_over".to_string(),
                location: c.location.clone(),
                context: format!(
                    "Word count {} exceeds limit of {} ({:.0}%)",
                    c.word_count, c.word_limit, c.percentage
                ),
                severity: "critical".to_string(),
            })
            .collect();
        checks.push(QaCheckResult {
            check_type: "word_counts".to_string(),
            status: if wc_issues.is_empty() {
                "pass".to_string()
            } else {
                "fail".to_string()
            },
            issue_count: wc_issues.len(),
            issues: wc_issues,
        });

        // Signatures
        checks.push(Self::check_signatures(doc));

        // Aggregate
        let total_issues: usize = checks.iter().map(|c| c.issue_count).sum();
        let critical_count: usize = checks
            .iter()
            .flat_map(|c| c.issues.iter())
            .filter(|i| i.severity == "critical")
            .count();
        let ready_to_submit = critical_count == 0;

        FullCheckResult {
            checks,
            total_issues,
            critical_count,
            ready_to_submit,
        }
    }
}
