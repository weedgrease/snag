pub mod results;

use crate::config::{self, load_config};
use crate::marketplace::create_marketplace;
use crate::notifier::create_notifier;
use crate::types::AlertResult;
use anyhow::{Context, Result};
use chrono::Utc;
use results::{load_results, results_path, save_results};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tokio::signal;
use tracing::{error, info};

pub async fn run() -> Result<()> {
    let log_path = config::data_dir().join("daemon.log");
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    tracing_subscriber::fmt()
        .with_writer(file)
        .with_env_filter("snag=info")
        .init();

    let pid_path = config::data_dir().join("daemon.pid");
    std::fs::create_dir_all(pid_path.parent().unwrap())?;
    std::fs::write(&pid_path, std::process::id().to_string())?;

    info!("daemon started (pid: {})", std::process::id());

    let config_path = config::config_path();
    let results_path = results_path();

    let result = run_scheduler(&config_path, &results_path).await;

    let _ = std::fs::remove_file(&pid_path);
    info!("daemon stopped");

    result
}

async fn run_scheduler(config_path: &Path, results_path: &Path) -> Result<()> {
    let mut last_check_times: HashMap<uuid::Uuid, Instant> = HashMap::new();
    let mut last_config_modified = std::fs::metadata(config_path)
        .and_then(|m| m.modified())
        .ok();

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("received shutdown signal");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                let config_changed = std::fs::metadata(config_path)
                    .and_then(|m| m.modified())
                    .ok();

                if config_changed != last_config_modified {
                    info!("config changed, reloading");
                    last_config_modified = config_changed;
                }

                let config = match load_config(config_path) {
                    Ok(c) => c,
                    Err(e) => {
                        error!("failed to load config: {e}");
                        continue;
                    }
                };

                let now = Instant::now();

                for alert in &config.alerts {
                    if !alert.enabled {
                        continue;
                    }

                    let should_check = last_check_times
                        .get(&alert.id)
                        .map(|last| now.duration_since(*last) >= alert.check_interval)
                        .unwrap_or(true);

                    if !should_check {
                        continue;
                    }

                    if let Err(e) = check_alert(alert, results_path, None).await {
                        error!("failed to check alert '{}': {e}", alert.name);
                    }

                    last_check_times.insert(alert.id, Instant::now());
                }
            }
        }
    }

    Ok(())
}

async fn check_alert(alert: &crate::types::Alert, results_path: &Path, default_location: Option<&str>) -> Result<()> {
    let mut all_listings = vec![];

    for marketplace_kind in &alert.marketplaces {
        let marketplace = create_marketplace(*marketplace_kind);
        match marketplace.search(alert, default_location).await {
            Ok(listings) => all_listings.extend(listings),
            Err(e) => {
                error!(
                    "marketplace {} failed for alert '{}': {e}",
                    marketplace.name(),
                    alert.name
                );
            }
        }
    }

    let mut existing_results = load_results(results_path).unwrap_or_default();

    let existing_ids: std::collections::HashSet<String> = existing_results
        .iter()
        .flat_map(|r| r.listings.iter().map(|l| l.id.clone()))
        .collect();

    let new_listings: Vec<_> = all_listings
        .into_iter()
        .filter(|l| !existing_ids.contains(&l.id))
        .collect();

    let new_listings = if let Some(max) = alert.max_results {
        if new_listings.len() > max as usize {
            new_listings[..max as usize].to_vec()
        } else {
            new_listings
        }
    } else {
        new_listings
    };

    if !new_listings.is_empty() {
        for notifier_kind in &alert.notifiers {
            let notifier = create_notifier(*notifier_kind);
            if let Err(e) = notifier.notify(alert, &new_listings).await {
                error!("notifier {} failed: {e}", notifier.name());
            }
        }

        existing_results.push(AlertResult {
            alert_id: alert.id,
            alert_name: alert.name.clone(),
            listings: new_listings,
            checked_at: Utc::now(),
            seen: false,
        });

        save_results(&existing_results, results_path)?;
    }

    Ok(())
}

pub async fn check_once() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("snag=info")
        .init();

    let config_path = config::config_path();
    let results_path = results_path();

    check_once_with_paths(&config_path, &results_path).await
}

pub async fn check_once_with_paths(config_path: &Path, results_path: &Path) -> Result<()> {
    let config = load_config(config_path).context("failed to load config")?;

    for alert in &config.alerts {
        if !alert.enabled {
            continue;
        }

        check_alert(alert, results_path, None).await?;
    }

    Ok(())
}
