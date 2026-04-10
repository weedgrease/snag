use snag::config::{AppConfig, GlobalSettings, load_config, save_config};
use snag::types::*;
use snag::types::LogLevel;
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn save_and_load_config_round_trips() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let config = AppConfig {
        settings: GlobalSettings {
            default_check_interval: Duration::from_secs(300),
            default_max_results: Some(20),
            default_notifier: NotifierKind::Terminal,
            check_for_updates: true,
            default_location: None,
            log_level: LogLevel::Info,
        },
        alerts: vec![
            Alert {
                id: Uuid::nil(),
                name: "Test Alert".into(),
                marketplaces: vec![MarketplaceKind::FacebookMarketplace],
                keywords: vec!["ps5".into()],
                exclude_keywords: vec!["broken".into()],
                price_min: Some(100.0),
                price_max: Some(500.0),
                location: Some("Denver, CO".into()),
                radius_miles: Some(25),
                condition: Some(Condition::Used),
                category: Some("Electronics".into()),
                check_interval: Duration::from_secs(300),
                notifiers: vec![NotifierKind::Terminal],
                max_results: Some(20),
                enabled: true,
            },
        ],
    };

    save_config(&config, &config_path).unwrap();
    let loaded = load_config(&config_path).unwrap();

    assert_eq!(loaded.alerts.len(), 1);
    assert_eq!(loaded.alerts[0].name, "Test Alert");
    assert_eq!(loaded.settings.default_check_interval, Duration::from_secs(300));
}

#[test]
fn load_missing_config_returns_default() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("nonexistent.toml");

    let config = load_config(&config_path).unwrap();

    assert!(config.alerts.is_empty());
    assert_eq!(config.settings.default_check_interval, Duration::from_secs(300));
}

#[test]
fn load_config_without_check_for_updates_defaults_to_true() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    std::fs::write(
        &config_path,
        "[settings]\ndefault_check_interval = 300\ndefault_notifier = \"Terminal\"\n",
    )
    .unwrap();

    let config = load_config(&config_path).unwrap();
    assert!(config.settings.check_for_updates);
}

#[test]
fn save_config_creates_parent_directories() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("nested").join("deep").join("config.toml");

    let config = AppConfig::default();
    save_config(&config, &config_path).unwrap();

    assert!(config_path.exists());
}

#[test]
fn config_with_default_location_round_trips() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let config = AppConfig {
        settings: GlobalSettings {
            default_check_interval: Duration::from_secs(300),
            default_max_results: Some(20),
            default_notifier: NotifierKind::Terminal,
            check_for_updates: true,
            default_location: Some("Denver, CO".into()),
            log_level: LogLevel::Info,
        },
        alerts: vec![],
    };

    save_config(&config, &config_path).unwrap();
    let loaded = load_config(&config_path).unwrap();

    assert_eq!(loaded.settings.default_location, Some("Denver, CO".into()));
}
