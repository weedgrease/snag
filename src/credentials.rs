//! Credential storage for marketplace API keys.
//! Stores secrets in `~/.config/snag/credentials.toml` with restricted file permissions (0600).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

fn credentials_path() -> PathBuf {
    crate::config::config_dir().join("credentials.toml")
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CredentialStore {
    #[serde(flatten)]
    entries: HashMap<String, String>,
}

fn load_store() -> CredentialStore {
    let path = credentials_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default()
}

fn save_store(store: &CredentialStore) -> Result<()> {
    let path = credentials_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }

    let content = toml::to_string_pretty(store).context("failed to serialize credentials")?;
    std::fs::write(&path, &content)
        .with_context(|| format!("failed to write credentials to {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms).ok();
    }

    Ok(())
}

/// Stores a credential key-value pair.
pub fn store_credential(key: &str, value: &str) -> Result<()> {
    let mut store = load_store();
    store.entries.insert(key.to_string(), value.to_string());
    save_store(&store)?;
    log::info!(target: "snag::credentials", "stored credential '{}'", key);
    Ok(())
}

/// Retrieves a credential by key, or `None` if not set.
pub fn get_credential(key: &str) -> Result<Option<String>> {
    let store = load_store();
    Ok(store.entries.get(key).cloned())
}

/// Returns `true` only when both `ebay_client_id` and `ebay_client_secret` are present.
pub fn ebay_credentials_configured() -> bool {
    let store = load_store();
    store.entries.contains_key("ebay_client_id")
        && store.entries.contains_key("ebay_client_secret")
}
