use crate::types::*;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub settings: GlobalSettings,
    #[serde(default)]
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSettings {
    #[serde(with = "crate::types::duration_secs")]
    pub default_check_interval: Duration,
    pub default_max_results: Option<u32>,
    pub default_notifier: NotifierKind,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            settings: GlobalSettings {
                default_check_interval: Duration::from_secs(300),
                default_max_results: Some(20),
                default_notifier: NotifierKind::Terminal,
            },
            alerts: vec![],
        }
    }
}

pub fn config_dir() -> PathBuf {
    ProjectDirs::from("", "", "snag")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn data_dir() -> PathBuf {
    ProjectDirs::from("", "", "snag")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn load_config(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config from {}", path.display()))?;

    let config: AppConfig = toml::from_str(&content)
        .with_context(|| format!("failed to parse config from {}", path.display()))?;

    Ok(config)
}

pub fn save_config(config: &AppConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }

    let content = toml::to_string_pretty(config)
        .context("failed to serialize config")?;

    std::fs::write(path, content)
        .with_context(|| format!("failed to write config to {}", path.display()))?;

    Ok(())
}
