//! Keyring-backed credential storage for marketplace API keys.

use anyhow::{Context, Result};

const SERVICE_NAME: &str = "snag";

pub fn store_credential(key: &str, value: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, key).context("failed to create keyring entry")?;
    entry
        .set_password(value)
        .context("failed to store credential in keyring")?;
    Ok(())
}

pub fn get_credential(key: &str) -> Result<Option<String>> {
    let entry = keyring::Entry::new(SERVICE_NAME, key).context("failed to create keyring entry")?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!(
            "failed to read credential from keyring: {}",
            e
        )),
    }
}

pub fn delete_credential(key: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, key).context("failed to create keyring entry")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(
            "failed to delete credential from keyring: {}",
            e
        )),
    }
}

/// Returns `true` only when both `ebay_client_id` and `ebay_client_secret` are present in the keyring.
pub fn ebay_credentials_configured() -> bool {
    get_credential("ebay_client_id").ok().flatten().is_some()
        && get_credential("ebay_client_secret")
            .ok()
            .flatten()
            .is_some()
}
