pub mod results;

use crate::config::{self, load_config};
use crate::scheduler::{self, Scheduler, SchedulerEvent};
use crate::types::CheckStatus;
use anyhow::{Context, Result};
use chrono::Utc;
use log::{error, info};
use results::{
    load_results, load_status, results_path, save_results, save_status, status_path, upsert_status,
};
use std::collections::HashSet;
use std::path::Path;
use tokio::sync::{mpsc, watch};

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

    let _lock = scheduler::try_acquire_scheduler_lock()
        .context("scheduler already running — another snag instance or daemon holds the lock")?;

    info!("daemon started (pid: {})", std::process::id());

    let config_path = config::config_path();
    let results_path = results_path();
    let status_path = status_path();

    let config = load_config(&config_path)?;
    let existing_results = load_results(&results_path).unwrap_or_default();
    let existing_ids: HashSet<String> = existing_results
        .iter()
        .flat_map(|r| r.listings.iter().map(|l| l.id.clone()))
        .collect();

    let (event_tx, mut event_rx) = mpsc::channel::<SchedulerEvent>(64);
    let (config_tx, config_rx) = watch::channel(config.clone());

    let scheduler = Scheduler::new(event_tx, config_rx, existing_ids);
    tokio::spawn(scheduler.run());

    let config_path_clone = config_path.clone();
    tokio::spawn(async move {
        let mut last_modified = std::fs::metadata(&config_path_clone)
            .and_then(|m| m.modified())
            .ok();

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let current = std::fs::metadata(&config_path_clone)
                .and_then(|m| m.modified())
                .ok();
            if current != last_modified {
                if let Ok(new_config) = load_config(&config_path_clone) {
                    let _ = config_tx.send(new_config);
                    info!("config reloaded");
                }
                last_modified = current;
            }
        }
    });

    let mut all_results = existing_results;
    let mut statuses = load_status(&status_path).unwrap_or_default();

    while let Some(event) = event_rx.recv().await {
        match event {
            SchedulerEvent::CheckComplete { status, result } => {
                upsert_status(&mut statuses, status);
                if let Some(alert_result) = result {
                    all_results.push(alert_result);
                    if let Err(e) = save_results(&all_results, &results_path) {
                        error!("failed to save results: {e}");
                    }
                }
                if let Err(e) = save_status(&statuses, &status_path) {
                    error!("failed to save status: {e}");
                }
            }
            SchedulerEvent::CheckError { alert_id, error } => {
                error!("alert check failed: {error}");
                upsert_status(
                    &mut statuses,
                    CheckStatus {
                        alert_id,
                        checked_at: Utc::now(),
                        new_results: 0,
                        error: Some(error),
                    },
                );
                if let Err(e) = save_status(&statuses, &status_path) {
                    error!("failed to save status: {e}");
                }
            }
        }
    }

    info!("daemon stopped");
    Ok(())
}

pub async fn check_once() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("snag=info")
        .init();

    let config_path = config::config_path();
    let results_path = results_path();
    let status_path = status_path();

    check_once_with_paths(&config_path, &results_path, &status_path).await
}

pub async fn check_once_with_paths(
    config_path: &Path,
    results_path: &Path,
    status_path: &Path,
) -> Result<()> {
    let config = load_config(config_path).context("failed to load config")?;
    let mut all_results = load_results(results_path).unwrap_or_default();
    let mut statuses = load_status(status_path).unwrap_or_default();

    let existing_ids: HashSet<String> = all_results
        .iter()
        .flat_map(|r| r.listings.iter().map(|l| l.id.clone()))
        .collect();

    for alert in &config.alerts {
        if !alert.enabled {
            continue;
        }

        match scheduler::check_alert(
            alert,
            &existing_ids,
            config.settings.default_location.as_deref(),
        )
        .await
        {
            Ok((status, new_listings)) => {
                upsert_status(&mut statuses, status);
                if !new_listings.is_empty() {
                    all_results.push(crate::types::AlertResult {
                        alert_id: alert.id,
                        alert_name: alert.name.clone(),
                        listings: new_listings,
                        checked_at: Utc::now(),
                        seen: false,
                    });
                    if let Err(e) = save_results(&all_results, results_path) {
                        error!("failed to save results: {e}");
                    }
                }
            }
            Err(e) => {
                upsert_status(
                    &mut statuses,
                    CheckStatus {
                        alert_id: alert.id,
                        checked_at: Utc::now(),
                        new_results: 0,
                        error: Some(format!("{e}")),
                    },
                );
                return Err(e);
            }
        }
    }

    if let Err(e) = save_status(&statuses, status_path) {
        error!("failed to save status: {e}");
    }
    Ok(())
}
