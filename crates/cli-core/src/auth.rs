use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Credentials storage for auth providers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthStore {
    #[serde(default)]
    pub credentials: Vec<ProviderCredentials>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCredentials {
    pub provider: String,
    pub api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl AuthStore {
    /// Load auth store from disk
    pub fn load() -> Result<Self> {
        let path = Self::auth_path();
        if !path.exists() {
            return Ok(AuthStore::default());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read auth store from {:?}", path))?;
        let store: AuthStore = serde_yaml::from_str(&content)
            .with_context(|| format!("failed to parse auth store from {:?}", path))?;
        Ok(store)
    }

    /// Save auth store to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::auth_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create auth directory {:?}", parent))?;
        }
        let content = serde_yaml::to_string(self)
            .context("failed to serialize auth store")?;
        // Set file permissions to owner read/write only (600)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::Permissions::mode(0o600);
            fs::set_permissions(&path, perms)?;
        }
        fs::write(&path, content)
            .with_context(|| format!("failed to write auth store to {:?}", path))?;
        Ok(())
    }

    /// Get auth path
    fn auth_path() -> PathBuf {
        if let Ok(home) = std::env::var("HERMES_HOME") {
            return PathBuf::from(home).join("credentials.yaml");
        }
        if let Ok(profile) = std::env::var("HERMES_PROFILE") {
            if let Some(proj_dirs) = ProjectDirs::from("ai", "hermes", &format!("hermes-{}", profile)) {
                return proj_dirs.config_dir().join("credentials.yaml");
            }
        }
        if let Some(proj_dirs) = ProjectDirs::from("ai", "hermes", "hermes-cli") {
            return proj_dirs.config_dir().join("credentials.yaml");
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join(".hermes").join("credentials.yaml");
        }
        PathBuf::from(".hermes").join("credentials.yaml")
    }

    /// Add credentials for a provider
    pub fn add(&mut self, provider: &str, api_key: &str, base_url: Option<&str>) {
        // Remove existing credentials for this provider
        self.credentials.retain(|c| c.provider != provider);

        // Add new credentials
        self.credentials.push(ProviderCredentials {
            provider: provider.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.map(|s| s.to_string()),
        });
    }

    /// List all credentials (with API keys masked)
    pub fn list(&self) -> Vec<(String, String, Option<String>)> {
        self.credentials
            .iter()
            .map(|c| {
                (
                    c.provider.clone(),
                    mask_key(&c.api_key),
                    c.base_url.clone(),
                )
            })
            .collect()
    }

    /// Get credentials for a provider
    pub fn get(&self, provider: &str) -> Option<&ProviderCredentials> {
        self.credentials.iter().find(|c| c.provider == provider)
    }

    /// Remove credentials for a provider
    pub fn remove(&mut self, provider: &str) -> bool {
        let len = self.credentials.len();
        self.credentials.retain(|c| c.provider != provider);
        self.credentials.len() < len
    }

    /// Clear all credentials
    pub fn reset(&mut self) {
        self.credentials.clear();
    }
}

/// Mask an API key for display (show first 4 and last 4 chars)
fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    let start = &key[..4];
    let end = &key[key.len() - 4..];
    format!("{}...{}", start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_key() {
        // Keys <= 8 chars are fully masked
        assert_eq!(mask_key("short"), "*****");
        assert_eq!(mask_key("12345678"), "********");
        // Keys > 8 chars show first 4 and last 4
        assert_eq!(mask_key("sk-1234567890abcdef"), "sk-1...cdef");
    }

    #[test]
    fn test_auth_store_add_get() {
        let mut store = AuthStore::default();
        store.add("openai", "sk-test123", None);
        assert!(store.get("openai").is_some());
        assert_eq!(store.get("openai").unwrap().api_key, "sk-test123");
    }

    #[test]
    fn test_auth_store_remove() {
        let mut store = AuthStore::default();
        store.add("openai", "sk-test123", None);
        assert!(store.remove("openai"));
        assert!(store.get("openai").is_none());
    }

    #[test]
    fn test_auth_store_list_masked() {
        let mut store = AuthStore::default();
        store.add("openai", "sk-1234567890abcdef", None);
        let list = store.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "openai");
        assert_eq!(list[0].1, "sk-1...cdef"); // Masked
    }
}