//! File-based persistence for alert results, check statuses, and seen listing IDs.
//! All reads and writes use advisory file locking (`fs2`) to coordinate between the TUI and any
//! external processes reading the same data directory.

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
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => {
            return Err(e).with_context(|| format!("failed to open results at {}", path.display()));
        }
    };

    file.lock_shared()
        .context("failed to acquire shared lock on results")?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .context("failed to read results")?;

    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    let results: Vec<AlertResult> =
        serde_json::from_str(&content).context("failed to parse results")?;

    Ok(results)
}

pub fn load_status(path: &Path) -> Result<Vec<crate::types::CheckStatus>> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => {
            return Err(e).with_context(|| format!("failed to open status at {}", path.display()));
        }
    };

    file.lock_shared()
        .context("failed to acquire shared lock on status")?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .context("failed to read status")?;

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
        .truncate(false)
        .open(path)
        .with_context(|| format!("failed to open status for writing at {}", path.display()))?;

    file.lock_exclusive()
        .context("failed to acquire exclusive lock on status")?;

    file.set_len(0).context("failed to truncate status file")?;

    let content = serde_json::to_string_pretty(status).context("failed to serialize status")?;

    file.write_all(content.as_bytes())
        .context("failed to write status")?;

    Ok(())
}

pub fn seen_path() -> std::path::PathBuf {
    crate::config::data_dir().join("seen.json")
}

pub fn load_seen(path: &Path) -> Result<std::collections::HashSet<String>> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(std::collections::HashSet::new());
        }
        Err(e) => {
            return Err(e).with_context(|| format!("failed to open seen at {}", path.display()));
        }
    };

    file.lock_shared()
        .context("failed to acquire shared lock on seen")?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .context("failed to read seen")?;

    if content.trim().is_empty() {
        return Ok(std::collections::HashSet::new());
    }

    let seen: Vec<String> = serde_json::from_str(&content).context("failed to parse seen")?;

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
        .truncate(false)
        .open(path)
        .with_context(|| format!("failed to open seen for writing at {}", path.display()))?;

    file.lock_exclusive()
        .context("failed to acquire exclusive lock on seen")?;

    file.set_len(0).context("failed to truncate seen file")?;

    let seen_vec: Vec<&String> = seen.iter().collect();
    let content = serde_json::to_string(&seen_vec).context("failed to serialize seen")?;

    file.write_all(content.as_bytes())
        .context("failed to write seen")?;

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
        .truncate(false)
        .open(path)
        .with_context(|| format!("failed to open results for writing at {}", path.display()))?;

    file.lock_exclusive()
        .context("failed to acquire exclusive lock on results")?;

    file.set_len(0).context("failed to truncate results file")?;

    let content = serde_json::to_string_pretty(results).context("failed to serialize results")?;

    file.write_all(content.as_bytes())
        .context("failed to write results")?;

    Ok(())
}
