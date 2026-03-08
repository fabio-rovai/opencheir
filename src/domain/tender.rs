use std::path::Path;

use regex::Regex;
use serde::Serialize;

use crate::sentinel_core::documents::{DocumentService, ParsedDocument};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum QuestionType {
    Scored,
    PassFail,
    Information,
    Declaration,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenderQuestion {
    pub id: String,
    pub section: String,
    pub text: String,
    pub question_type: QuestionType,
    pub word_limit: Option<usize>,
    pub marks: Option<String>,
    pub table_index: usize,
    pub row_index: usize,
    pub cell_index: usize,
    pub answer_text: Option<String>,
    pub current_word_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenderStructure {
    pub filename: String,
    pub title: Option<String>,
    pub questions: Vec<TenderQuestion>,
    pub total_tables: usize,
    pub total_sections: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceResult {
    pub question_id: String,
    pub question_type: String,
    pub word_count: usize,
    pub word_limit: Option<usize>,
    pub usage_pct: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PassFailResult {
    pub question_id: String,
    pub status: String, // "ANSWERED" or "EMPTY"
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FullCheckResult {
    pub compliance: Vec<ComplianceResult>,
    pub pass_fail: Vec<PassFailResult>,
    pub file_checks: Vec<FileCheckResult>,
    pub summary: CheckSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckSummary {
    pub optimal: usize,
    pub over_limit: usize,
    pub under_target: usize,
    pub significantly_under: usize,
    pub empty_pass_fail: usize,
    pub missing_files: usize,
    pub ready_to_submit: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileCheckResult {
    pub section: String,
    pub filename: String,
    pub found: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadResult {
    pub table_index: usize,
    pub row_index: usize,
    pub cell_index: usize,
    pub text: String,
    pub word_count: usize,
}

// ---------------------------------------------------------------------------
// TenderService
// ---------------------------------------------------------------------------

pub struct TenderService;

impl TenderService {
    // -----------------------------------------------------------------------
    // Regex helpers (compiled once per call via lazy patterns)
    // -----------------------------------------------------------------------

    /// Extract a question ID from the start of cell text.
    /// Matches: "5.4", "6.1(a)", "3-1(b)", "1.2.3"
    fn extract_question_id(text: &str) -> Option<String> {
        let re = Regex::new(r"^(\d+(?:[.\-]\d+)*(?:\([a-z]\))?)").unwrap();
        re.captures(text.trim()).map(|caps| caps[1].to_string())
    }

    /// Detect question type from text content (case-insensitive).
    fn detect_question_type(text: &str) -> QuestionType {
        let lower = text.to_lowercase();
        if lower.contains("pass/fail") || lower.contains("pass or fail") {
            QuestionType::PassFail
        } else if lower.contains("will be scored") || lower.contains("scoring") {
            QuestionType::Scored
        } else if lower.contains("please confirm") || lower.contains("i confirm") {
            QuestionType::Declaration
        } else {
            QuestionType::Information
        }
    }

    /// Extract word limit from text using common patterns.
    fn extract_word_limit(text: &str) -> Option<usize> {
        let patterns = [
            r"(?i)(?:word\s*count|limited?\s*to|max(?:imum)?)\s*[:.]?\s*(\d+)\s*words?",
            r"(?i)(\d+)\s*words?\s*(?:max|limit|maximum)",
            r"(?i)up\s*to\s*(\d+)\s*words?",
        ];

        for pattern in &patterns {
            let re = Regex::new(pattern).unwrap();
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

    /// Determine compliance status from usage percentage.
    fn compliance_status(word_count: usize, word_limit: Option<usize>) -> String {
        match word_limit {
            None => "NO_LIMIT".to_string(),
            Some(limit) if limit == 0 => "NO_LIMIT".to_string(),
            Some(limit) => {
                let pct = (word_count as f64 / limit as f64) * 100.0;
                if pct > 100.0 {
                    "OVER_LIMIT".to_string()
                } else if pct >= 89.0 {
                    "OPTIMAL".to_string()
                } else if pct >= 70.0 {
                    "UNDER_TARGET".to_string()
                } else {
                    "SIGNIFICANTLY_UNDER".to_string()
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // parse_tender
    // -----------------------------------------------------------------------

    /// Parse a tender DOCX, extracting all questions with cell coordinates.
    ///
    /// Iterates all tables looking for cells whose text starts with a question
    /// ID pattern. For each match, extracts the question text, type, word
    /// limit, marks, and locates the answer cell in an adjacent cell or the
    /// row below.
    pub fn parse_tender(doc: &ParsedDocument) -> TenderStructure {
        let mut questions = Vec::new();
        let mut sections_seen = std::collections::HashSet::new();

        // Derive a title from the first non-empty paragraph
        let title = doc
            .paragraphs
            .iter()
            .find(|p| !p.text.trim().is_empty())
            .map(|p| p.text.trim().to_string());

        let filename = Path::new(&doc.path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| doc.path.clone());

        for (ti, table) in doc.tables.iter().enumerate() {
            for (ri, row) in table.rows.iter().enumerate() {
                for (ci, cell) in row.cells.iter().enumerate() {
                    let trimmed = cell.text.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    if let Some(qid) = Self::extract_question_id(trimmed) {
                        // Derive section from the question ID (everything before
                        // the last dot/dash segment)
                        let section = qid
                            .split(|c: char| c == '.' || c == '-')
                            .next()
                            .unwrap_or(&qid)
                            .to_string();
                        sections_seen.insert(section.clone());

                        // The question text is typically in the next cell on
                        // the same row, or in the cell text itself after the ID
                        let question_text = if ci + 1 < row.cells.len() {
                            // Next cell contains the question body
                            row.cells[ci + 1].text.trim().to_string()
                        } else {
                            // Question text is the rest of this cell after the ID
                            let after_id = trimmed
                                .strip_prefix(&qid)
                                .unwrap_or(trimmed)
                                .trim()
                                .to_string();
                            if after_id.is_empty() {
                                trimmed.to_string()
                            } else {
                                after_id
                            }
                        };

                        // Detect question type from the question text
                        let question_type = Self::detect_question_type(&question_text);

                        // Extract word limit from question text
                        let word_limit = Self::extract_word_limit(&question_text);

                        // Look for marks/weighting in the cell after the
                        // question text (typically the 3rd cell)
                        let marks = if ci + 2 < row.cells.len() {
                            let marks_text = row.cells[ci + 2].text.trim().to_string();
                            if marks_text.is_empty() {
                                None
                            } else {
                                Some(marks_text)
                            }
                        } else {
                            None
                        };

                        // Look for answer text in the row below, same column
                        // as the question text (ci + 1), or in the same cell
                        let (answer_text, answer_word_count) =
                            Self::find_answer_text(table, ri, ci, &row.cells);

                        // If we found a scored question type from marks but
                        // not from text, upgrade
                        let question_type = if marks.is_some()
                            && question_type == QuestionType::Information
                        {
                            QuestionType::Scored
                        } else {
                            question_type
                        };

                        questions.push(TenderQuestion {
                            id: qid,
                            section: section.clone(),
                            text: question_text,
                            question_type,
                            word_limit,
                            marks,
                            table_index: ti,
                            row_index: ri,
                            cell_index: ci,
                            answer_text,
                            current_word_count: answer_word_count,
                        });
                    }
                }
            }
        }

        TenderStructure {
            filename,
            title,
            questions,
            total_tables: doc.tables.len(),
            total_sections: sections_seen.len(),
        }
    }

    /// Look for answer text in the row below the question, in the question
    /// text cell column (ci + 1 if available, else ci).
    fn find_answer_text(
        table: &crate::sentinel_core::documents::ParsedTable,
        question_row: usize,
        id_cell: usize,
        row_cells: &[crate::sentinel_core::documents::ParsedCell],
    ) -> (Option<String>, usize) {
        // Determine which column the answer would be in
        let answer_col = if id_cell + 1 < row_cells.len() {
            id_cell + 1
        } else {
            id_cell
        };

        // Look 1-2 rows below for the answer
        for offset in 1..=2 {
            let target_row = question_row + offset;
            if let Some(row) = table.rows.get(target_row) {
                if let Some(cell) = row.cells.get(answer_col) {
                    let text = cell.text.trim();
                    if !text.is_empty() {
                        let wc = DocumentService::word_count(text);
                        return (Some(text.to_string()), wc);
                    }
                }
            }
        }

        (None, 0)
    }

    // -----------------------------------------------------------------------
    // read_answer
    // -----------------------------------------------------------------------

    /// Read the current content of a specific cell.
    pub fn read_answer(
        doc: &ParsedDocument,
        table_idx: usize,
        row_idx: usize,
        cell_idx: usize,
    ) -> Option<ReadResult> {
        let text = DocumentService::get_cell_text(doc, table_idx, row_idx, cell_idx)?;
        let word_count = DocumentService::word_count(text);
        Some(ReadResult {
            table_index: table_idx,
            row_index: row_idx,
            cell_index: cell_idx,
            text: text.to_string(),
            word_count,
        })
    }

    // -----------------------------------------------------------------------
    // check_compliance
    // -----------------------------------------------------------------------

    /// Check word count compliance for all scored questions.
    pub fn check_compliance(structure: &TenderStructure) -> Vec<ComplianceResult> {
        structure
            .questions
            .iter()
            .filter(|q| q.question_type == QuestionType::Scored)
            .map(|q| {
                let usage_pct = q.word_limit.map(|limit| {
                    if limit == 0 {
                        0.0
                    } else {
                        (q.current_word_count as f64 / limit as f64) * 100.0
                    }
                });

                let status = Self::compliance_status(q.current_word_count, q.word_limit);

                ComplianceResult {
                    question_id: q.id.clone(),
                    question_type: format!("{:?}", q.question_type),
                    word_count: q.current_word_count,
                    word_limit: q.word_limit,
                    usage_pct,
                    status,
                }
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // check_pass_fail
    // -----------------------------------------------------------------------

    /// Check all pass/fail questions are answered.
    /// A question is considered answered if the answer text has more than 10
    /// characters (not words) to catch brief acknowledgements.
    pub fn check_pass_fail(structure: &TenderStructure) -> Vec<PassFailResult> {
        structure
            .questions
            .iter()
            .filter(|q| q.question_type == QuestionType::PassFail)
            .map(|q| {
                let is_answered = q
                    .answer_text
                    .as_ref()
                    .map(|t| t.trim().len() > 10)
                    .unwrap_or(false);

                let preview = q.answer_text.as_ref().map(|t| {
                    let trimmed = t.trim();
                    if trimmed.len() > 80 {
                        format!("{}...", &trimmed[..80])
                    } else {
                        trimmed.to_string()
                    }
                });

                PassFailResult {
                    question_id: q.id.clone(),
                    status: if is_answered {
                        "ANSWERED".to_string()
                    } else {
                        "EMPTY".to_string()
                    },
                    preview,
                }
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // check_submission_files
    // -----------------------------------------------------------------------

    /// Check submission files exist in folder.
    /// `expected` is a slice of (section, filename) pairs.
    pub fn check_submission_files(
        folder: &str,
        expected: &[(String, String)],
    ) -> Vec<FileCheckResult> {
        let base = Path::new(folder);

        expected
            .iter()
            .map(|(section, filename)| {
                let file_path = base.join(filename);
                let found = file_path.exists();

                FileCheckResult {
                    section: section.clone(),
                    filename: filename.clone(),
                    found,
                    status: if found {
                        "FOUND".to_string()
                    } else {
                        "MISSING".to_string()
                    },
                }
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // full_check
    // -----------------------------------------------------------------------

    /// Run full compliance check, aggregating all individual checks.
    pub fn full_check(
        structure: &TenderStructure,
        folder: Option<&str>,
        expected_files: Option<&[(String, String)]>,
    ) -> FullCheckResult {
        let compliance = Self::check_compliance(structure);
        let pass_fail = Self::check_pass_fail(structure);

        let file_checks = match (folder, expected_files) {
            (Some(f), Some(files)) => Self::check_submission_files(f, files),
            _ => Vec::new(),
        };

        // Aggregate counts
        let optimal = compliance
            .iter()
            .filter(|c| c.status == "OPTIMAL")
            .count();
        let over_limit = compliance
            .iter()
            .filter(|c| c.status == "OVER_LIMIT")
            .count();
        let under_target = compliance
            .iter()
            .filter(|c| c.status == "UNDER_TARGET")
            .count();
        let significantly_under = compliance
            .iter()
            .filter(|c| c.status == "SIGNIFICANTLY_UNDER")
            .count();
        let empty_pass_fail = pass_fail
            .iter()
            .filter(|p| p.status == "EMPTY")
            .count();
        let missing_files = file_checks.iter().filter(|f| !f.found).count();

        let ready_to_submit = over_limit == 0 && empty_pass_fail == 0;

        FullCheckResult {
            compliance,
            pass_fail,
            file_checks,
            summary: CheckSummary {
                optimal,
                over_limit,
                under_target,
                significantly_under,
                empty_pass_fail,
                missing_files,
                ready_to_submit,
            },
        }
    }
}
