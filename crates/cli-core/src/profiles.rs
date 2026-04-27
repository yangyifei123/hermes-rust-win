// Profile management - helper functions for profile file operations

use crate::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Returns the profiles directory path: HERMES_HOME/profiles/
pub fn profiles_dir() -> PathBuf {
    Config::hermes_home().join("profiles")
}

/// Returns the path to a specific profile: HERMES_HOME/profiles/<name>.yaml
pub fn profile_path(name: &str) -> PathBuf {
    profiles_dir().join(format!("{}.yaml", name))
}

/// Creates the profiles directory if it doesn't exist
pub fn ensure_profiles_dir() -> Result<()> {
    let dir = profiles_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create profiles directory at {:?}", dir))?;
    }
    Ok(())
}

/// Lists all profile names (without .yaml extension)
pub fn list_profiles() -> Result<Vec<String>> {
    ensure_profiles_dir()?;
    let dir = profiles_dir();
    let mut profiles = Vec::new();

    let entries = fs::read_dir(&dir)
        .with_context(|| format!("failed to read profiles directory {:?}", dir))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "yaml").unwrap_or(false) {
            if let Some(stem) = path.file_stem() {
                profiles.push(stem.to_string_lossy().to_string());
            }
        }
    }

    profiles.sort();
    Ok(profiles)
}

/// Loads a profile by name, returning the Config
pub fn load_profile(name: &str) -> Result<Config> {
    let path = profile_path(name);
    if !path.exists() {
        anyhow::bail!("profile '{}' not found at {:?}", name, path);
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read profile from {:?}", path))?;
    let config: Config = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse profile from {:?}", path))?;
    Ok(config)
}

/// Saves a config as a profile with the given name
pub fn save_profile(name: &str, config: &Config) -> Result<()> {
    ensure_profiles_dir()?;
    let path = profile_path(name);
    let content = serde_yaml::to_string(config).context("failed to serialize profile")?;
    fs::write(&path, content).with_context(|| format!("failed to write profile to {:?}", path))?;
    Ok(())
}

/// Copies a profile from one name to another
pub fn clone_profile(from_name: &str, to_name: &str) -> Result<()> {
    let config = load_profile(from_name)?;
    save_profile(to_name, &config)?;
    Ok(())
}

/// Deletes a profile by name
pub fn delete_profile(name: &str) -> Result<()> {
    let path = profile_path(name);
    if !path.exists() {
        anyhow::bail!("profile '{}' not found", name);
    }
    fs::remove_file(&path).with_context(|| format!("failed to delete profile at {:?}", path))?;
    Ok(())
}

/// Renames a profile (file rename)
pub fn rename_profile(old_name: &str, new_name: &str) -> Result<()> {
    let old_path = profile_path(old_name);
    let new_path = profile_path(new_name);

    if !old_path.exists() {
        anyhow::bail!("profile '{}' not found", old_name);
    }
    if new_path.exists() {
        anyhow::bail!("profile '{}' already exists", new_name);
    }

    fs::rename(&old_path, &new_path).with_context(|| {
        format!(
            "failed to rename profile from {:?} to {:?}",
            old_path, new_path
        )
    })?;
    Ok(())
}

/// Checks if a profile exists
pub fn profile_exists(name: &str) -> bool {
    profile_path(name).exists()
}

/// Gets the active profile name from HERMES_PROFILE env var, or "default" if not set
pub fn get_active_profile() -> String {
    std::env::var("HERMES_PROFILE").unwrap_or_else(|_| "default".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiles_dir() {
        let dir = profiles_dir();
        assert!(dir.ends_with("profiles"));
    }

    #[test]
    fn test_profile_path() {
        let path = profile_path("myprofile");
        assert!(path.ends_with("myprofile.yaml"));
    }
}
