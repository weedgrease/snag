use crate::config::AppConfig;
use crate::marketplace::create_marketplace;
use crate::notifier::create_notifier;
use crate::types::{Alert, AlertResult, CheckStatus, Listing};
use anyhow::Result;
use chrono::Utc;
use fs2::FileExt;
use log::error;
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::Instant;
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

/// Events emitted by the scheduler after each alert check.
///
/// `CheckComplete` is sent on every successful poll (result is `None` when no new listings were found).
/// `CheckError` is sent when all marketplaces for the alert returned errors.
#[derive(Debug)]
pub enum SchedulerEvent {
    CheckComplete {
        status: CheckStatus,
        result: Option<AlertResult>,
    },
    CheckError {
        alert_id: Uuid,
        error: String,
    },
}

/// Attempts to acquire an exclusive lock on the daemon PID file.
/// Returns `Some(file)` with the lock held if successful, or `None` if another instance already holds it.
pub fn try_acquire_scheduler_lock() -> Option<File> {
    let pid_path = crate::config::data_dir().join("daemon.pid");
    if let Some(parent) = pid_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&pid_path)
        .ok()?;

    if file.try_lock_exclusive().is_err() {
        return None;
    }

    file.set_len(0).ok()?;
    file.write_all(std::process::id().to_string().as_bytes())
        .ok()?;
    file.flush().ok()?;
    Some(file)
}

/// Searches all marketplaces for the alert, filters out previously seen listing IDs, and caps
/// results at `alert.max_results`. Returns an error only if every marketplace failed.
pub async fn check_alert(
    alert: &Alert,
    existing_ids: &HashSet<String>,
    default_location: Option<&str>,
) -> Result<(CheckStatus, Vec<Listing>)> {
    let mut all_listings = vec![];
    let mut last_error: Option<anyhow::Error> = None;

    for marketplace_kind in &alert.marketplaces {
        let marketplace = create_marketplace(*marketplace_kind);
        match marketplace.search(alert, default_location).await {
            Ok(listings) => {
                all_listings.extend(listings);
                last_error = None;
            }
            Err(e) => {
                error!(
                    "marketplace {} failed for alert '{}': {e}",
                    marketplace.name(),
                    alert.name
                );
                last_error = Some(e);
            }
        }
    }

    if all_listings.is_empty()
        && let Some(e) = last_error
    {
        return Err(e);
    }

    let total_fetched = all_listings.len();
    let new_listings: Vec<Listing> = all_listings
        .into_iter()
        .filter(|l| !existing_ids.contains(&l.id))
        .collect();

    log::info!(target: "snag::scheduler", "Alert '{}': fetched {}, {} new (filtered {} duplicates)",
        alert.name, total_fetched, new_listings.len(), total_fetched - new_listings.len());

    let new_listings = if let Some(max) = alert.max_results {
        if new_listings.len() > max as usize {
            new_listings[..max as usize].to_vec()
        } else {
            new_listings
        }
    } else {
        new_listings
    };

    let status = CheckStatus {
        alert_id: alert.id,
        checked_at: Utc::now(),
        new_results: new_listings.len(),
        error: None,
    };

    Ok((status, new_listings))
}

/// Background polling engine that drives alert checks on their configured intervals.
/// Constructed with [`Scheduler::new`] and consumed by [`Scheduler::run`], which loops until the
/// event channel is closed.
pub struct Scheduler {
    event_tx: mpsc::Sender<SchedulerEvent>,
    config_rx: watch::Receiver<AppConfig>,
    last_check_times: HashMap<Uuid, Instant>,
    existing_ids: HashSet<String>,
}

impl Scheduler {
    pub fn new(
        event_tx: mpsc::Sender<SchedulerEvent>,
        config_rx: watch::Receiver<AppConfig>,
        initial_existing_ids: HashSet<String>,
    ) -> Self {
        Self {
            event_tx,
            config_rx,
            last_check_times: HashMap::new(),
            existing_ids: initial_existing_ids,
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let config = self.config_rx.borrow().clone();
            let now = Instant::now();

            for alert in &config.alerts {
                if !alert.enabled {
                    continue;
                }

                let should_check = self
                    .last_check_times
                    .get(&alert.id)
                    .map(|last| now.duration_since(*last) >= alert.check_interval)
                    .unwrap_or(true);

                if !should_check {
                    continue;
                }

                let default_loc = config.settings.default_location.as_deref();

                log::info!(target: "snag::scheduler", "Checking alert: '{}'", alert.name);

                match check_alert(alert, &self.existing_ids, default_loc).await {
                    Ok((status, new_listings)) => {
                        for listing in &new_listings {
                            self.existing_ids.insert(listing.id.clone());
                        }

                        let result = if new_listings.is_empty() {
                            None
                        } else {
                            for notifier_kind in &alert.notifiers {
                                let notifier = create_notifier(*notifier_kind);
                                if let Err(e) = notifier.notify(alert, &new_listings).await {
                                    error!("notifier {} failed: {e}", notifier.name());
                                }
                            }

                            Some(AlertResult {
                                alert_id: alert.id,
                                alert_name: alert.name.clone(),
                                listings: new_listings,
                                checked_at: Utc::now(),
                                seen: false,
                            })
                        };

                        if self
                            .event_tx
                            .send(SchedulerEvent::CheckComplete { status, result })
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(e) => {
                        error!("failed to check alert '{}': {e}", alert.name);
                        if self
                            .event_tx
                            .send(SchedulerEvent::CheckError {
                                alert_id: alert.id,
                                error: format!("{e}"),
                            })
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                }

                self.last_check_times.insert(alert.id, Instant::now());
            }
        }
    }
}
