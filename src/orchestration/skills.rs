use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum SkillSource {
    Builtin,
    Personal,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub command: Option<String>,
    pub path: PathBuf,
    pub source: SkillSource,
    pub sub_documents: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillHealthEntry {
    pub name: String,
    pub source: SkillSource,
    pub healthy: bool,
    pub error: Option<String>,
}

pub struct SkillsEngine {
    skills: HashMap<String, SkillMeta>,
    builtin_dir: PathBuf,
    personal_dir: PathBuf,
}

/// Parse YAML frontmatter from a SKILL.md file.
/// Extracts `name`, `description`, and `command` fields from the block
/// delimited by `---` at the very start of the file.
fn parse_frontmatter(content: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut name = None;
    let mut description = None;
    let mut command = None;

    let mut lines = content.lines();

    // First line must be "---"
    match lines.next() {
        Some(line) if line.trim() == "---" => {}
        _ => return (name, description, command),
    }

    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("name:") {
            name = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("description:") {
            description = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("command:") {
            command = Some(value.trim().to_string());
        }
    }

    (name, description, command)
}

/// List `.md` files in a directory, excluding `SKILL.md`.
fn list_sub_documents(skill_dir: &PathBuf) -> Vec<String> {
    let mut docs = Vec::new();
    if let Ok(entries) = fs::read_dir(skill_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.ends_with(".md") && file_name != "SKILL.md" {
                        docs.push(file_name.to_string());
                    }
                }
            }
        }
    }
    docs.sort();
    docs
}

impl SkillsEngine {
    /// Construct a new SkillsEngine and scan both directories for skills.
    pub fn new(builtin_dir: PathBuf, personal_dir: PathBuf) -> Self {
        let mut engine = Self {
            skills: HashMap::new(),
            builtin_dir,
            personal_dir,
        };
        engine.scan();
        engine
    }

    /// Scan both directories for SKILL.md files.
    /// Each immediate subdirectory containing a SKILL.md is treated as a skill.
    /// The directory name becomes the skill key.
    /// Personal skills shadow builtin skills with the same name.
    pub fn scan(&mut self) {
        self.skills.clear();

        // Scan builtin first
        self.scan_dir(&self.builtin_dir.clone(), SkillSource::Builtin);

        // Then scan personal -- these overwrite builtins with the same name
        self.scan_dir(&self.personal_dir.clone(), SkillSource::Personal);
    }

    fn scan_dir(&mut self, base_dir: &PathBuf, source: SkillSource) {
        let entries = match fs::read_dir(base_dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let skill_file = path.join("SKILL.md");
            if !skill_file.is_file() {
                continue;
            }

            let dir_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            let content = match fs::read_to_string(&skill_file) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let (fm_name, fm_desc, fm_command) = parse_frontmatter(&content);

            let name = fm_name.unwrap_or_else(|| dir_name.clone());
            let description = fm_desc.unwrap_or_default();

            let sub_documents = list_sub_documents(&path);

            let meta = SkillMeta {
                name: name.clone(),
                description,
                command: fm_command,
                path: skill_file,
                source: source.clone(),
                sub_documents,
            };

            // Insert by directory name -- personal overwrites builtin
            self.skills.insert(dir_name, meta);
        }
    }

    /// Return all skills sorted alphabetically by name.
    pub fn list(&self) -> Vec<&SkillMeta> {
        let mut skills: Vec<&SkillMeta> = self.skills.values().collect();
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        skills
    }

    /// Get a skill by its directory name.
    pub fn get(&self, name: &str) -> Option<&SkillMeta> {
        self.skills.get(name)
    }

    /// Read and return the full SKILL.md content for a skill.
    pub fn get_content(&self, name: &str) -> Option<String> {
        let meta = self.skills.get(name)?;
        fs::read_to_string(&meta.path).ok()
    }

    /// Read a sub-document from a skill directory.
    pub fn get_sub_document(&self, skill_name: &str, doc_name: &str) -> Option<String> {
        let meta = self.skills.get(skill_name)?;
        let skill_dir = meta.path.parent()?;
        let doc_path = skill_dir.join(doc_name);
        fs::read_to_string(doc_path).ok()
    }

    /// Resolve a skill by name. Since personal skills already shadow builtin
    /// during scan, this is equivalent to get().
    pub fn resolve(&self, name: &str) -> Option<&SkillMeta> {
        self.get(name)
    }

    /// Check health of all registered skills. Verifies that each SKILL.md
    /// still exists and is readable.
    pub fn health(&self) -> Vec<SkillHealthEntry> {
        let mut entries: Vec<SkillHealthEntry> = self
            .skills
            .values()
            .map(|meta| match fs::read_to_string(&meta.path) {
                Ok(_) => SkillHealthEntry {
                    name: meta.name.clone(),
                    source: meta.source.clone(),
                    healthy: true,
                    error: None,
                },
                Err(e) => SkillHealthEntry {
                    name: meta.name.clone(),
                    source: meta.source.clone(),
                    healthy: false,
                    error: Some(e.to_string()),
                },
            })
            .collect();

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }
}
