use crate::types::AlertResult;
use anyhow::{Context, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub fn results_path() -> std::path::PathBuf {
    crate::config::data_dir().join("results.json")
}

pub fn status_path() -> std::path::PathBuf {
    crate::config::data_dir().join("status.json")
}

pub fn load_results(path: &Path) -> Result<Vec<AlertResult>> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let mut file = File::open(path)
        .with_context(|| format!("failed to open results at {}", path.display()))?;

    file.lock_shared()
        .context("failed to acquire shared lock on results")?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .context("failed to read results")?;

    file.unlock().context("failed to release lock on results")?;

    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    let results: Vec<AlertResult> =
        serde_json::from_str(&content).context("failed to parse results")?;

    Ok(results)
}

pub fn load_status(path: &Path) -> Result<Vec<crate::types::CheckStatus>> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let mut file = File::open(path)
        .with_context(|| format!("failed to open status at {}", path.display()))?;

    file.lock_shared()
        .context("failed to acquire shared lock on status")?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .context("failed to read status")?;

    file.unlock().context("failed to release lock on status")?;

    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    let status: Vec<crate::types::CheckStatus> =
        serde_json::from_str(&content).context("failed to parse status")?;

    Ok(status)
}

pub fn save_status(status: &[crate::types::CheckStatus], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create data directory {}", parent.display()))?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("failed to open status for writing at {}", path.display()))?;

    file.lock_exclusive()
        .context("failed to acquire exclusive lock on status")?;

    let content = serde_json::to_string_pretty(status).context("failed to serialize status")?;

    file.write_all(content.as_bytes())
        .context("failed to write status")?;

    file.unlock()
        .context("failed to release lock on status")?;

    Ok(())
}

pub fn seen_path() -> std::path::PathBuf {
    crate::config::data_dir().join("seen.json")
}

pub fn load_seen(path: &Path) -> Result<std::collections::HashSet<String>> {
    if !path.exists() {
        return Ok(std::collections::HashSet::new());
    }

    let mut file = File::open(path)
        .with_context(|| format!("failed to open seen at {}", path.display()))?;

    file.lock_shared()
        .context("failed to acquire shared lock on seen")?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .context("failed to read seen")?;

    file.unlock().context("failed to release lock on seen")?;

    if content.trim().is_empty() {
        return Ok(std::collections::HashSet::new());
    }

    let seen: Vec<String> =
        serde_json::from_str(&content).context("failed to parse seen")?;

    Ok(seen.into_iter().collect())
}

pub fn save_seen(seen: &std::collections::HashSet<String>, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create data directory {}", parent.display()))?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("failed to open seen for writing at {}", path.display()))?;

    file.lock_exclusive()
        .context("failed to acquire exclusive lock on seen")?;

    let seen_vec: Vec<&String> = seen.iter().collect();
    let content = serde_json::to_string(&seen_vec).context("failed to serialize seen")?;

    file.write_all(content.as_bytes())
        .context("failed to write seen")?;

    file.unlock()
        .context("failed to release lock on seen")?;

    Ok(())
}

pub fn save_results(results: &[AlertResult], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create data directory {}", parent.display()))?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("failed to open results for writing at {}", path.display()))?;

    file.lock_exclusive()
        .context("failed to acquire exclusive lock on results")?;

    let content = serde_json::to_string_pretty(results).context("failed to serialize results")?;

    file.write_all(content.as_bytes())
        .context("failed to write results")?;

    file.unlock()
        .context("failed to release lock on results")?;

    Ok(())
}
