use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Plugin configuration stored in plugins.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub source: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default = "default_installed_at")]
    pub installed_at: String,
    #[serde(default)]
    pub updated_at: String,
}

fn default_enabled() -> bool {
    true
}

fn default_installed_at() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Storage for plugin configurations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginStore {
    #[serde(default)]
    pub plugins: Vec<Plugin>,
}

impl PluginStore {
    /// Load plugin store from HERMES_HOME/plugins.json
    pub fn load() -> Result<Self> {
        let path = Self::plugins_path();
        if !path.exists() {
            return Ok(PluginStore::default());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read plugins store from {:?}", path))?;
        let store: PluginStore = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse plugins store from {:?}", path))?;
        Ok(store)
    }

    /// Save plugin store to HERMES_HOME/plugins.json
    pub fn save(&self) -> Result<()> {
        let path = Self::plugins_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create plugins directory {:?}", parent))?;
        }
        let content =
            serde_json::to_string_pretty(self).context("failed to serialize plugins store")?;
        fs::write(&path, content)
            .with_context(|| format!("failed to write plugins store to {:?}", path))?;
        Ok(())
    }

    /// Get plugins path
    pub fn plugins_path() -> PathBuf {
        if let Ok(home) = std::env::var("HERMES_HOME") {
            return PathBuf::from(home).join("plugins.json");
        }
        if let Ok(profile) = std::env::var("HERMES_PROFILE") {
            if let Some(proj_dirs) =
                ProjectDirs::from("ai", "hermes", &format!("hermes-{}", profile))
            {
                return proj_dirs.config_dir().join("plugins.json");
            }
        }
        if let Some(proj_dirs) = ProjectDirs::from("ai", "hermes", "hermes-cli") {
            return proj_dirs.config_dir().join("plugins.json");
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join(".hermes").join("plugins.json");
        }
        PathBuf::from(".hermes").join("plugins.json")
    }

    /// Add a plugin
    pub fn add_plugin(&mut self, plugin: Plugin) -> Result<()> {
        // Check if name already exists
        if self.plugins.iter().any(|p| p.name == plugin.name) {
            anyhow::bail!(
                "Plugin '{}' already installed. Remove it first or use update.",
                plugin.name
            );
        }

        self.plugins.push(plugin);
        Ok(())
    }

    /// Remove a plugin by name
    pub fn remove_plugin(&mut self, name: &str) -> Result<()> {
        let len = self.plugins.len();
        self.plugins.retain(|p| p.name != name);
        if self.plugins.len() == len {
            anyhow::bail!("Plugin '{}' not found", name);
        }
        Ok(())
    }

    /// Get a plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|p| p.name == name)
    }

    /// Get a mutable plugin by name
    pub fn get_plugin_mut(&mut self, name: &str) -> Option<&mut Plugin> {
        self.plugins.iter_mut().find(|p| p.name == name)
    }

    /// List all plugins
    pub fn list_plugins(&self) -> &[Plugin] {
        &self.plugins
    }

    /// Update plugin version
    pub fn update_plugin(&mut self, name: &str, version: &str) -> Result<()> {
        let plugin = self.get_plugin_mut(name);
        match plugin {
            Some(p) => {
                p.version = version.to_string();
                p.updated_at = chrono::Utc::now().to_rfc3339();
                Ok(())
            }
            None => anyhow::bail!("Plugin '{}' not found", name),
        }
    }

    /// Enable a plugin
    pub fn enable_plugin(&mut self, name: &str) -> Result<()> {
        let plugin = self.get_plugin_mut(name);
        match plugin {
            Some(p) => {
                p.enabled = true;
                Ok(())
            }
            None => anyhow::bail!("Plugin '{}' not found", name),
        }
    }

    /// Disable a plugin
    pub fn disable_plugin(&mut self, name: &str) -> Result<()> {
        let plugin = self.get_plugin_mut(name);
        match plugin {
            Some(p) => {
                p.enabled = false;
                Ok(())
            }
            None => anyhow::bail!("Plugin '{}' not found", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_store_default() {
        let store = PluginStore::default();
        assert!(store.plugins.is_empty());
    }

    #[test]
    fn test_plugin_store_add() {
        let mut store = PluginStore::default();
        let plugin = Plugin {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            source: "https://example.com/test-plugin".to_string(),
            enabled: true,
            description: "Test plugin".to_string(),
            author: "Test Author".to_string(),
            installed_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        store.add_plugin(plugin).unwrap();
        assert_eq!(store.plugins.len(), 1);
        assert_eq!(store.plugins[0].name, "test-plugin");
    }

    #[test]
    fn test_plugin_store_add_duplicate() {
        let mut store = PluginStore::default();
        let plugin = Plugin {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            source: "https://example.com/test-plugin".to_string(),
            enabled: true,
            description: "Test plugin".to_string(),
            author: "Test Author".to_string(),
            installed_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        store.add_plugin(plugin.clone()).unwrap();
        let result = store.add_plugin(plugin);
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_store_remove() {
        let mut store = PluginStore::default();
        let plugin = Plugin {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            source: "https://example.com/test-plugin".to_string(),
            enabled: true,
            description: "Test plugin".to_string(),
            author: "Test Author".to_string(),
            installed_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        store.add_plugin(plugin).unwrap();
        store.remove_plugin("test-plugin").unwrap();
        assert!(store.plugins.is_empty());
    }

    #[test]
    fn test_plugin_store_enable_disable() {
        let mut store = PluginStore::default();
        let plugin = Plugin {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            source: "https://example.com/test-plugin".to_string(),
            enabled: true,
            description: "Test plugin".to_string(),
            author: "Test Author".to_string(),
            installed_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        store.add_plugin(plugin).unwrap();
        store.disable_plugin("test-plugin").unwrap();
        assert!(!store.plugins[0].enabled);
        store.enable_plugin("test-plugin").unwrap();
        assert!(store.plugins[0].enabled);
    }
}
