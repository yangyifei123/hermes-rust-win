use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Webhook configuration stored in webhooks.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub name: String,
    pub url: String,
    pub events: Vec<String>,
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default = "default_deliver")]
    pub deliver: String,
    #[serde(default)]
    pub deliver_chat_id: String,
    #[serde(default)]
    pub secret: String,
    #[serde(default = "default_added_at")]
    pub added_at: String,
}

fn default_deliver() -> String {
    "log".to_string()
}

fn default_added_at() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Storage for webhook configurations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookStore {
    #[serde(default)]
    pub webhooks: Vec<Webhook>,
}

impl WebhookStore {
    /// Load webhook store from HERMES_HOME/webhooks.json
    pub fn load() -> Result<Self> {
        let path = Self::webhooks_path();
        if !path.exists() {
            return Ok(WebhookStore::default());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read webhooks store from {:?}", path))?;
        let store: WebhookStore = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse webhooks store from {:?}", path))?;
        Ok(store)
    }

    /// Save webhook store to HERMES_HOME/webhooks.json
    pub fn save(&self) -> Result<()> {
        let path = Self::webhooks_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create webhooks directory {:?}", parent))?;
        }
        let content =
            serde_json::to_string_pretty(self).context("failed to serialize webhooks store")?;
        fs::write(&path, content)
            .with_context(|| format!("failed to write webhooks store to {:?}", path))?;
        Ok(())
    }

    /// Get webhooks path
    pub fn webhooks_path() -> PathBuf {
        if let Ok(home) = std::env::var("HERMES_HOME") {
            return PathBuf::from(home).join("webhooks.json");
        }
        if let Ok(profile) = std::env::var("HERMES_PROFILE") {
            if let Some(proj_dirs) =
                ProjectDirs::from("ai", "hermes", &format!("hermes-{}", profile))
            {
                return proj_dirs.config_dir().join("webhooks.json");
            }
        }
        if let Some(proj_dirs) = ProjectDirs::from("ai", "hermes", "hermes-cli") {
            return proj_dirs.config_dir().join("webhooks.json");
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join(".hermes").join("webhooks.json");
        }
        PathBuf::from(".hermes").join("webhooks.json")
    }

    /// Add a webhook
    pub fn add_webhook(&mut self, webhook: Webhook) -> Result<()> {
        // Check if name already exists
        if self.webhooks.iter().any(|w| w.name == webhook.name) {
            anyhow::bail!(
                "Webhook '{}' already exists. Use a different name or remove it first.",
                webhook.name
            );
        }

        self.webhooks.push(webhook);
        Ok(())
    }

    /// Remove a webhook by name
    pub fn remove_webhook(&mut self, name: &str) -> Result<()> {
        let len = self.webhooks.len();
        self.webhooks.retain(|w| w.name != name);
        if self.webhooks.len() == len {
            anyhow::bail!("Webhook '{}' not found", name);
        }
        Ok(())
    }

    /// Get a webhook by name
    pub fn get_webhook(&self, name: &str) -> Option<&Webhook> {
        self.webhooks.iter().find(|w| w.name == name)
    }

    /// List all webhooks
    pub fn list_webhooks(&self) -> &[Webhook] {
        &self.webhooks
    }

    /// Update webhook enabled status
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<()> {
        let webhook = self.webhooks.iter_mut().find(|w| w.name == name);
        match webhook {
            Some(w) => {
                w.enabled = enabled;
                Ok(())
            }
            None => anyhow::bail!("Webhook '{}' not found", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_store_default() {
        let store = WebhookStore::default();
        assert!(store.webhooks.is_empty());
    }

    #[test]
    fn test_webhook_store_add() {
        let mut store = WebhookStore::default();
        let webhook = Webhook {
            name: "test".to_string(),
            url: "https://example.com/webhook".to_string(),
            events: vec!["message".to_string()],
            enabled: true,
            description: "Test webhook".to_string(),
            skills: vec![],
            deliver: "log".to_string(),
            deliver_chat_id: String::new(),
            secret: String::new(),
            added_at: "2026-01-01T00:00:00Z".to_string(),
        };
        store.add_webhook(webhook).unwrap();
        assert_eq!(store.webhooks.len(), 1);
        assert_eq!(store.webhooks[0].name, "test");
    }

    #[test]
    fn test_webhook_store_add_duplicate() {
        let mut store = WebhookStore::default();
        let webhook = Webhook {
            name: "test".to_string(),
            url: "https://example.com/webhook".to_string(),
            events: vec![],
            enabled: true,
            description: String::new(),
            skills: vec![],
            deliver: "log".to_string(),
            deliver_chat_id: String::new(),
            secret: String::new(),
            added_at: String::new(),
        };
        store.add_webhook(webhook.clone()).unwrap();
        let result = store.add_webhook(webhook);
        assert!(result.is_err());
    }

    #[test]
    fn test_webhook_store_remove() {
        let mut store = WebhookStore::default();
        let webhook = Webhook {
            name: "test".to_string(),
            url: "https://example.com/webhook".to_string(),
            events: vec![],
            enabled: true,
            description: String::new(),
            skills: vec![],
            deliver: "log".to_string(),
            deliver_chat_id: String::new(),
            secret: String::new(),
            added_at: String::new(),
        };
        store.add_webhook(webhook).unwrap();
        store.remove_webhook("test").unwrap();
        assert!(store.webhooks.is_empty());
    }
}
