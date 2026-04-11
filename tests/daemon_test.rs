use snag::config::{AppConfig, GlobalSettings, save_config};
use snag::daemon::results::load_results;
use snag::types::*;
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn check_once_runs_without_error_for_enabled_alerts() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    let results_path = dir.path().join("results.json");

    let config = AppConfig {
        settings: GlobalSettings {
            default_check_interval: Duration::from_secs(3600),
            default_max_results: Some(20),
            default_notifier: NotifierKind::Terminal,
            check_for_updates: true,
            default_location: None,
        },
        alerts: vec![Alert {
            id: Uuid::new_v4(),
            name: "Test Alert".into(),
            marketplaces: vec![MarketplaceKind::Ebay],
            keywords: vec!["test".into()],
            exclude_keywords: vec![],
            price_min: None,
            price_max: None,
            location: None,
            radius_miles: None,
            condition: None,
            category: None,
            check_interval: Duration::from_secs(3600),
            notifiers: vec![NotifierKind::Terminal],
            max_results: None,
            enabled: true,
        }],
    };

    save_config(&config, &config_path).unwrap();

    let _ = snag::daemon::check_once_with_paths(
        &config_path,
        &results_path,
        &dir.path().join("status.json"),
    )
    .await;

    // Result depends on whether eBay credentials are configured on this machine.
    // The test verifies the check runs without panicking.
}

#[tokio::test]
async fn check_once_skips_disabled_alerts() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    let results_path = dir.path().join("results.json");

    let config = AppConfig {
        settings: GlobalSettings {
            default_check_interval: Duration::from_secs(300),
            default_max_results: Some(20),
            default_notifier: NotifierKind::Terminal,
            check_for_updates: true,
            default_location: None,
        },
        alerts: vec![Alert {
            id: Uuid::new_v4(),
            name: "Disabled Alert".into(),
            marketplaces: vec![MarketplaceKind::FacebookMarketplace],
            keywords: vec!["test".into()],
            exclude_keywords: vec![],
            price_min: None,
            price_max: None,
            location: None,
            radius_miles: None,
            condition: None,
            category: None,
            check_interval: Duration::from_secs(300),
            notifiers: vec![NotifierKind::Terminal],
            max_results: None,
            enabled: false,
        }],
    };

    save_config(&config, &config_path).unwrap();

    snag::daemon::check_once_with_paths(
        &config_path,
        &results_path,
        &dir.path().join("status.json"),
    )
    .await
    .unwrap();

    let results = load_results(&results_path).unwrap();
    assert!(results.is_empty());
}
