use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Pairing status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PairingStatus {
    Pending,
    Approved,
    Revoked,
}

/// A pairing entry for platform connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pairing {
    pub platform: String,
    pub user_id: String,
    pub code: Option<String>,
    pub status: PairingStatus,
    #[serde(default)]
    pub display_name: String,
    #[serde(default = "default_created_at")]
    pub created_at: String,
    #[serde(default)]
    pub approved_at: Option<String>,
}

fn default_created_at() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Storage for pairing configurations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PairingStore {
    #[serde(default)]
    pub pairings: Vec<Pairing>,
}

impl PairingStore {
    /// Load pairing store from HERMES_HOME/pairings.json
    pub fn load() -> Result<Self> {
        let path = Self::pairings_path();
        if !path.exists() {
            return Ok(PairingStore::default());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read pairings store from {:?}", path))?;
        let store: PairingStore = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse pairings store from {:?}", path))?;
        Ok(store)
    }

    /// Save pairing store to HERMES_HOME/pairings.json
    pub fn save(&self) -> Result<()> {
        let path = Self::pairings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create pairings directory {:?}", parent))?;
        }
        let content =
            serde_json::to_string_pretty(self).context("failed to serialize pairings store")?;
        fs::write(&path, content)
            .with_context(|| format!("failed to write pairings store to {:?}", path))?;
        Ok(())
    }

    /// Get pairings path
    pub fn pairings_path() -> PathBuf {
        if let Ok(home) = std::env::var("HERMES_HOME") {
            return PathBuf::from(home).join("pairings.json");
        }
        if let Ok(profile) = std::env::var("HERMES_PROFILE") {
            if let Some(proj_dirs) =
                ProjectDirs::from("ai", "hermes", &format!("hermes-{}", profile))
            {
                return proj_dirs.config_dir().join("pairings.json");
            }
        }
        if let Some(proj_dirs) = ProjectDirs::from("ai", "hermes", "hermes-cli") {
            return proj_dirs.config_dir().join("pairings.json");
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join(".hermes").join("pairings.json");
        }
        PathBuf::from(".hermes").join("pairings.json")
    }

    /// Add a pairing
    pub fn add_pairing(&mut self, pairing: Pairing) -> Result<()> {
        // Check if already exists
        if self
            .pairings
            .iter()
            .any(|p| p.platform == pairing.platform && p.user_id == pairing.user_id)
        {
            anyhow::bail!(
                "Pairing for platform '{}' and user '{}' already exists",
                pairing.platform,
                pairing.user_id
            );
        }

        self.pairings.push(pairing);
        Ok(())
    }

    /// Approve a pairing
    pub fn approve_pairing(&mut self, platform: &str, code: &str) -> Result<()> {
        let pairing = self
            .pairings
            .iter_mut()
            .find(|p| p.platform == platform && p.code.as_deref() == Some(code));

        match pairing {
            Some(p) => {
                p.status = PairingStatus::Approved;
                p.approved_at = Some(chrono::Utc::now().to_rfc3339());
                Ok(())
            }
            None => anyhow::bail!(
                "No pending pairing found for platform '{}' with code '{}'",
                platform,
                code
            ),
        }
    }

    /// Revoke a pairing
    pub fn revoke_pairing(&mut self, platform: &str, user_id: &str) -> Result<()> {
        let len = self.pairings.len();
        self.pairings.retain(|p| !(p.platform == platform && p.user_id == user_id));
        if self.pairings.len() == len {
            anyhow::bail!("Pairing for platform '{}' and user '{}' not found", platform, user_id);
        }
        Ok(())
    }

    /// Clear all pending pairings
    pub fn clear_pending(&mut self) -> Result<()> {
        let initial_count = self.pairings.len();
        self.pairings.retain(|p| p.status != PairingStatus::Pending);
        if self.pairings.len() == initial_count {
            anyhow::bail!("No pending pairings to clear");
        }
        Ok(())
    }

    /// List all pairings
    pub fn list_pairings(&self) -> &[Pairing] {
        &self.pairings
    }

    /// List pairings by status
    pub fn list_by_status(&self, status: &PairingStatus) -> Vec<&Pairing> {
        self.pairings.iter().filter(|p| &p.status == status).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairing_store_default() {
        let store = PairingStore::default();
        assert!(store.pairings.is_empty());
    }

    #[test]
    fn test_pairing_store_add() {
        let mut store = PairingStore::default();
        let pairing = Pairing {
            platform: "telegram".to_string(),
            user_id: "user123".to_string(),
            code: Some("ABC123".to_string()),
            status: PairingStatus::Pending,
            display_name: "Test User".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            approved_at: None,
        };
        store.add_pairing(pairing).unwrap();
        assert_eq!(store.pairings.len(), 1);
    }

    #[test]
    fn test_pairing_store_approve() {
        let mut store = PairingStore::default();
        let pairing = Pairing {
            platform: "telegram".to_string(),
            user_id: "user123".to_string(),
            code: Some("ABC123".to_string()),
            status: PairingStatus::Pending,
            display_name: "Test User".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            approved_at: None,
        };
        store.add_pairing(pairing).unwrap();
        store.approve_pairing("telegram", "ABC123").unwrap();
        assert_eq!(store.pairings[0].status, PairingStatus::Approved);
    }

    #[test]
    fn test_pairing_store_revoke() {
        let mut store = PairingStore::default();
        let pairing = Pairing {
            platform: "telegram".to_string(),
            user_id: "user123".to_string(),
            code: None,
            status: PairingStatus::Approved,
            display_name: "Test User".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            approved_at: Some("2026-01-01T00:00:00Z".to_string()),
        };
        store.add_pairing(pairing).unwrap();
        store.revoke_pairing("telegram", "user123").unwrap();
        assert!(store.pairings.is_empty());
    }
}
