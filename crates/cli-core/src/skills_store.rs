//! Skills store — reusable prompt templates loaded into the agent's system prompt.
//!
//! Skills are stored as individual `.md` (with YAML frontmatter) or `.yaml` files
//! inside `~/.hermes/skills/`. When the directory is empty, built-in skills are
//! returned by `list_skills()` without being written to disk.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ── Types ──────────────────────────────────────────────────────────────────────

/// A reusable prompt template that can be loaded into the agent's system prompt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    /// The system prompt template injected when the skill is activated.
    pub prompt: String,
    /// Optional grouping category (e.g. "coding", "language").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// Internal representation used when deserialising YAML skill files.
#[derive(Debug, Deserialize)]
struct YamlSkillFile {
    name: String,
    description: String,
    prompt: String,
    #[serde(default)]
    category: Option<String>,
}

/// Internal representation of YAML frontmatter found in Markdown skill files.
#[derive(Debug, Deserialize)]
struct MdFrontmatter {
    #[serde(default)]
    description: String,
    #[serde(default)]
    category: Option<String>,
}

// ── SkillStore ─────────────────────────────────────────────────────────────────

/// Manages skill files on disk under `~/.hermes/skills/`.
pub struct SkillStore {
    skills_dir: PathBuf,
}

impl SkillStore {
    /// Create a new `SkillStore`, ensuring the skills directory exists.
    pub fn new() -> Result<Self> {
        let skills_dir = Self::default_skills_dir();
        fs::create_dir_all(&skills_dir)
            .with_context(|| format!("failed to create skills directory {:?}", skills_dir))?;
        Ok(Self { skills_dir })
    }

    /// Create a `SkillStore` pointed at an arbitrary directory (useful for testing).
    pub fn with_dir(dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create skills directory {:?}", dir))?;
        Ok(Self { skills_dir: dir })
    }

    /// Return the path to the skills directory.
    pub fn skill_path(&self) -> PathBuf {
        self.skills_dir.clone()
    }

    // ── Load ────────────────────────────────────────────────────────────────

    /// Load a single skill by name.
    ///
    /// Looks for `{name}.md` first, then `{name}.yaml`.
    pub fn load_skill(&self, name: &str) -> Result<Skill> {
        // Sanitise: reject path separators to prevent directory traversal.
        if name.contains(std::path::MAIN_SEPARATOR) || name.contains('/') || name.contains('\\') {
            anyhow::bail!("invalid skill name '{}': path separators are not allowed", name);
        }

        let md_path = self.skills_dir.join(format!("{}.md", name));
        if md_path.exists() {
            return self.parse_md_skill(name, &md_path);
        }

        let yaml_path = self.skills_dir.join(format!("{}.yaml", name));
        if yaml_path.exists() {
            return self.parse_yaml_skill(&yaml_path);
        }

        // Fall back to built-in skills.
        if let Some(builtin) = Self::builtin_skills().iter().find(|s| s.name == name) {
            return Ok(builtin.clone());
        }

        anyhow::bail!("skill '{}' not found", name)
    }

    // ── List ────────────────────────────────────────────────────────────────

    /// List all available skills.
    ///
    /// Scans on-disk files and merges them with the built-in defaults (built-in
    /// skills are only included when the directory has no files at all).
    pub fn list_skills(&self) -> Result<Vec<Skill>> {
        let mut skills = Vec::new();

        // Scan .md and .yaml files in the skills directory.
        if let Ok(entries) = fs::read_dir(&self.skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

                match ext {
                    "md" => {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            if let Ok(skill) = self.parse_md_skill(name, &path) {
                                skills.push(skill);
                            }
                        }
                    }
                    "yaml" | "yml" => {
                        if let Ok(skill) = self.parse_yaml_skill(&path) {
                            skills.push(skill);
                        }
                    }
                    _ => {}
                }
            }
        }

        // If no files were found on disk, return built-in skills.
        if skills.is_empty() {
            skills = Self::builtin_skills();
        }

        Ok(skills)
    }

    // ── Install / Uninstall ─────────────────────────────────────────────────

    /// Save a new skill to disk as a Markdown file with YAML frontmatter.
    pub fn install_skill(&self, name: &str, content: &str) -> Result<()> {
        if name.contains(std::path::MAIN_SEPARATOR) || name.contains('/') || name.contains('\\') {
            anyhow::bail!("invalid skill name '{}': path separators are not allowed", name);
        }

        let path = self.skills_dir.join(format!("{}.md", name));
        fs::write(&path, content)
            .with_context(|| format!("failed to write skill to {:?}", path))?;
        Ok(())
    }

    /// Delete a skill file from disk. Returns `false` if nothing was deleted.
    pub fn uninstall_skill(&self, name: &str) -> Result<bool> {
        if name.contains(std::path::MAIN_SEPARATOR) || name.contains('/') || name.contains('\\') {
            anyhow::bail!("invalid skill name '{}': path separators are not allowed", name);
        }

        let md_path = self.skills_dir.join(format!("{}.md", name));
        if md_path.exists() {
            fs::remove_file(&md_path).with_context(|| format!("failed to delete {:?}", md_path))?;
            return Ok(true);
        }

        let yaml_path = self.skills_dir.join(format!("{}.yaml", name));
        if yaml_path.exists() {
            fs::remove_file(&yaml_path)
                .with_context(|| format!("failed to delete {:?}", yaml_path))?;
            return Ok(true);
        }

        Ok(false)
    }

    // ── Parsing helpers ─────────────────────────────────────────────────────

    /// Parse a Markdown skill file with YAML frontmatter.
    fn parse_md_skill(&self, name: &str, path: &PathBuf) -> Result<Skill> {
        let content =
            fs::read_to_string(path).with_context(|| format!("failed to read {:?}", path))?;
        let (fm, body) = split_frontmatter(&content);

        let description;
        let category;

        if let Some(ref fm_text) = fm {
            let parsed: MdFrontmatter = serde_yaml::from_str(fm_text)
                .with_context(|| format!("failed to parse frontmatter in {:?}", path))?;
            description = parsed.description;
            category = parsed.category;
        } else {
            description = String::new();
            category = None;
        }

        Ok(Skill { name: name.to_string(), description, prompt: body.trim().to_string(), category })
    }

    /// Parse a YAML skill file.
    fn parse_yaml_skill(&self, path: &PathBuf) -> Result<Skill> {
        let content =
            fs::read_to_string(path).with_context(|| format!("failed to read {:?}", path))?;
        let parsed: YamlSkillFile = serde_yaml::from_str(&content)
            .with_context(|| format!("failed to parse YAML skill {:?}", path))?;
        Ok(Skill {
            name: parsed.name,
            description: parsed.description,
            prompt: parsed.prompt,
            category: parsed.category,
        })
    }

    // ── Built-in skills ─────────────────────────────────────────────────────

    /// Return the hard-coded built-in skill set.
    fn builtin_skills() -> Vec<Skill> {
        vec![
            Skill {
                name: "code-review".into(),
                description: "Code review assistant".into(),
                prompt: "You are a code review expert. Analyze code for bugs, security issues, \
                    and performance problems. Provide constructive feedback with specific \
                    suggestions for improvement."
                    .into(),
                category: Some("coding".into()),
            },
            Skill {
                name: "explain".into(),
                description: "Code explanation".into(),
                prompt: "You are a patient code explainer. When given code, break it down \
                    step-by-step in plain language. Explain the purpose of each section, \
                    key algorithms, and potential gotchas."
                    .into(),
                category: Some("coding".into()),
            },
            Skill {
                name: "translate".into(),
                description: "Translation".into(),
                prompt: "You are a professional translator. Translate text accurately while \
                    preserving tone, idioms, and cultural nuances. When translating code \
                    comments, keep them natural in the target language."
                    .into(),
                category: Some("language".into()),
            },
            Skill {
                name: "summarize".into(),
                description: "Text summarization".into(),
                prompt: "You are a summarization specialist. Given any text, produce a clear, \
                    concise summary that captures the key points. Offer different lengths \
                    (one sentence, one paragraph, bullet points) when helpful."
                    .into(),
                category: Some("productivity".into()),
            },
        ]
    }

    // ── Directory helpers ───────────────────────────────────────────────────

    /// Resolve the default skills directory: `~/.hermes/skills/`.
    fn default_skills_dir() -> PathBuf {
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join(".hermes").join("skills");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".hermes").join("skills");
        }
        PathBuf::from(".hermes").join("skills")
    }
}

// ── Frontmatter splitter ───────────────────────────────────────────────────────

/// Split Markdown content into optional YAML frontmatter and body.
///
/// Returns `(None, content)` when no frontmatter is found.
fn split_frontmatter(content: &str) -> (Option<String>, String) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (None, content.to_string());
    }

    // Find the closing `---`.
    let after_first = &trimmed[3..];
    if let Some(end_offset) = after_first.find("---") {
        let frontmatter = after_first[..end_offset].to_string();
        let body = after_first[end_offset + 3..].to_string();
        return (Some(frontmatter), body);
    }

    (None, content.to_string())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (SkillStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = SkillStore::with_dir(dir.path().to_path_buf()).expect("store");
        (store, dir)
    }

    #[test]
    fn test_create_skillstore_with_temp_dir() {
        let (store, _dir) = temp_store();
        assert!(store.skill_path().exists());
        assert!(store.skill_path().is_dir());
    }

    #[test]
    fn test_install_and_load_skill() {
        let (store, _dir) = temp_store();

        let content =
            "---\ndescription: Test skill\ncategory: test\n---\nYou are a test assistant.";
        store.install_skill("my-test", content).expect("install");

        let skill = store.load_skill("my-test").expect("load");
        assert_eq!(skill.name, "my-test");
        assert_eq!(skill.description, "Test skill");
        assert_eq!(skill.category, Some("test".to_string()));
        assert_eq!(skill.prompt, "You are a test assistant.");
    }

    #[test]
    fn test_list_skills_returns_builtins_when_empty() {
        let (store, _dir) = temp_store();
        let skills = store.list_skills().expect("list");
        assert!(!skills.is_empty());
        // We define 4 built-ins.
        assert_eq!(skills.len(), 4);
        assert!(skills.iter().any(|s| s.name == "code-review"));
        assert!(skills.iter().any(|s| s.name == "explain"));
        assert!(skills.iter().any(|s| s.name == "translate"));
        assert!(skills.iter().any(|s| s.name == "summarize"));
    }

    #[test]
    fn test_list_skills_returns_disk_skills_when_present() {
        let (store, _dir) = temp_store();

        store.install_skill("custom", "---\ndescription: Custom\n---\nDo stuff.").expect("install");

        let skills = store.list_skills().expect("list");
        // On-disk skills should replace built-ins entirely.
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "custom");
    }

    #[test]
    fn test_uninstall_skill() {
        let (store, _dir) = temp_store();

        store
            .install_skill("temp-skill", "---\ndescription: Temp\n---\nTemp prompt.")
            .expect("install");

        assert!(store.uninstall_skill("temp-skill").expect("uninstall"));
        assert!(!store.uninstall_skill("temp-skill").expect("uninstall again"));
    }

    #[test]
    fn test_parse_markdown_frontmatter() {
        let content = "---\ndescription: Code review assistant\ncategory: coding\n---\nYou are a code review expert.";
        let (fm, body) = split_frontmatter(content);
        assert!(fm.is_some());
        let fm = fm.unwrap();
        assert!(fm.contains("description: Code review assistant"));
        assert!(fm.contains("category: coding"));
        assert_eq!(body.trim(), "You are a code review expert.");
    }

    #[test]
    fn test_parse_markdown_no_frontmatter() {
        let content = "Just a plain prompt with no frontmatter.";
        let (fm, body) = split_frontmatter(content);
        assert!(fm.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn test_load_yaml_skill() {
        let (store, _dir) = temp_store();

        let yaml = "name: my-yaml-skill\ndescription: A YAML skill\ncategory: misc\nprompt: |\n  You are a YAML-powered assistant.\n";
        let yaml_path = store.skill_path().join("my-yaml-skill.yaml");
        fs::write(&yaml_path, yaml).expect("write yaml");

        let skill = store.load_skill("my-yaml-skill").expect("load yaml");
        assert_eq!(skill.name, "my-yaml-skill");
        assert_eq!(skill.description, "A YAML skill");
        assert_eq!(skill.category, Some("misc".to_string()));
        assert!(skill.prompt.contains("YAML-powered assistant"));
    }

    #[test]
    fn test_reject_path_traversal() {
        let (store, _dir) = temp_store();
        assert!(store.load_skill("../etc/passwd").is_err());
        assert!(store.install_skill("a/b", "x").is_err());
        assert!(store.uninstall_skill("a\\b").is_err());
    }
}
