use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Skill metadata from frontmatter
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub related_skills: Vec<String>,
}

/// A skill with its metadata
#[derive(Debug, Clone)]
pub struct Skill {
    pub metadata: SkillMetadata,
    pub path: PathBuf,
}

/// Skills index (cached)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillsIndex {
    #[serde(default)]
    pub skills: HashMap<String, SkillMetadata>,
}

impl SkillsIndex {
    /// Load skills index from disk
    pub fn load() -> Result<Self> {
        let path = Self::skills_index_path();
        if !path.exists() {
            return Ok(SkillsIndex::default());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read skills index from {:?}", path))?;
        let index: SkillsIndex = serde_yaml::from_str(&content)
            .with_context(|| format!("failed to parse skills index from {:?}", path))?;
        Ok(index)
    }

    /// Save skills index to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::skills_index_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create skills directory {:?}", parent))?;
        }
        let content = serde_yaml::to_string(self).context("failed to serialize skills index")?;
        fs::write(&path, content)
            .with_context(|| format!("failed to write skills index to {:?}", path))?;
        Ok(())
    }

    /// Get skills index path
    fn skills_index_path() -> PathBuf {
        Self::skills_home().join(".hub").join("index.yaml")
    }

    /// Get skills home directory
    pub fn skills_home() -> PathBuf {
        if let Ok(home) = std::env::var("HERMES_HOME") {
            return PathBuf::from(home).join("skills");
        }
        if let Ok(profile) = std::env::var("HERMES_PROFILE") {
            if let Some(proj_dirs) =
                ProjectDirs::from("ai", "hermes", &format!("hermes-{}", profile))
            {
                return proj_dirs.data_dir().join("skills");
            }
        }
        if let Some(proj_dirs) = ProjectDirs::from("ai", "hermes", "hermes-cli") {
            return proj_dirs.data_dir().join("skills");
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join(".hermes").join("skills");
        }
        PathBuf::from(".hermes").join("skills")
    }

    /// Scan local skills directory and update index
    pub fn scan_local_skills(&mut self) -> Result<usize> {
        let skills_dir = Self::skills_home();
        let mut count = 0;

        if !skills_dir.exists() {
            return Ok(0);
        }

        // Clear existing skills
        self.skills.clear();

        // Scan for skill directories
        if let Ok(entries) = fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let skill_md = path.join("SKILL.md");
                    if skill_md.exists() {
                        if let Ok(content) = fs::read_to_string(&skill_md) {
                            if let Some(metadata) = parse_skill_frontmatter(&content) {
                                self.skills.insert(metadata.name.clone(), metadata);
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        self.save()?;
        Ok(count)
    }

    /// Get all skills
    pub fn get_all(&self) -> Vec<&SkillMetadata> {
        self.skills.values().collect()
    }

    /// Search skills by query
    pub fn search(&self, query: &str) -> Vec<&SkillMetadata> {
        let query_lower = query.to_lowercase();
        self.skills
            .values()
            .filter(|skill| {
                skill.name.to_lowercase().contains(&query_lower)
                    || skill.description.to_lowercase().contains(&query_lower)
                    || skill.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&SkillMetadata> {
        self.skills.get(name)
    }

    /// Add or update a skill
    pub fn add(&mut self, metadata: SkillMetadata) {
        self.skills.insert(metadata.name.clone(), metadata);
    }

    /// Remove a skill
    pub fn remove(&mut self, name: &str) -> bool {
        self.skills.remove(name).is_some()
    }
}

/// Parse frontmatter from SKILL.md content
fn parse_skill_frontmatter(content: &str) -> Option<SkillMetadata> {
    let content = content.trim();

    // Check for YAML frontmatter
    if !content.starts_with("---") {
        return None;
    }

    let end = content[3..].find("---")?;
    let frontmatter = &content[3..end];

    // Parse YAML frontmatter
    let metadata: SkillMetadata = serde_yaml::from_str(frontmatter).ok()?;

    Some(metadata)
}

/// Scan skills from bundled skills directory
pub fn scan_bundled_skills(bundled_path: &PathBuf) -> Result<Vec<Skill>> {
    let mut skills = Vec::new();

    if !bundled_path.exists() {
        return Ok(skills);
    }

    if let Ok(entries) = fs::read_dir(bundled_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    if let Ok(content) = fs::read_to_string(&skill_md) {
                        if let Some(metadata) = parse_skill_frontmatter(&content) {
                            skills.push(Skill { metadata, path: path.clone() });
                        }
                    }
                }
            }
        }
    }

    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skills_index_search() {
        let mut index = SkillsIndex::default();
        index.add(SkillMetadata {
            name: "test-skill".to_string(),
            description: "A test skill for testing".to_string(),
            tags: vec!["test".to_string()],
            ..Default::default()
        });
        index.add(SkillMetadata {
            name: "rust-programming".to_string(),
            description: "Rust programming help".to_string(),
            tags: vec!["rust".to_string(), "programming".to_string()],
            ..Default::default()
        });

        let results = index.search("rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust-programming");

        let results = index.search("test");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test-skill");
    }
}
