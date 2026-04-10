# In-Process Scheduler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract scheduling into a shared module with MPSC channels for the owning TUI instance and mtime-based file fallback for other instances.

**Architecture:** Move `check_alert` from `daemon/mod.rs` to a new `scheduler.rs` module. Add `SchedulerEvent` enum and `Scheduler` struct with a `run()` loop. TUI tries to acquire a PID file lock — if successful, spawns the scheduler in-process with MPSC channels; if not, uses mtime-checked file reads. `snag daemon` reuses the same `Scheduler`. Config changes push via `tokio::sync::watch`.

**Tech Stack:** Rust, tokio (MPSC + watch channels), fs2 (file locking, existing dep)

---

## File Structure

```
src/
├── scheduler.rs         — NEW: Scheduler, SchedulerEvent, check_alert, PID lock
├── lib.rs               — add pub mod scheduler
├── daemon/mod.rs        — rewrite to use Scheduler
├── tui/
│   ├── mod.rs           — remove auto_start_daemon
│   └── app.rs           — scheduler_rx, config_tx, mtime fallback, config push
```

---

### Task 1: Create scheduler.rs with check_alert and SchedulerEvent

**Files:**
- Create: `src/scheduler.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add module to lib.rs**

In `src/lib.rs`, add `pub mod scheduler;` after the existing module declarations.

- [ ] **Step 2: Create src/scheduler.rs with types and check_alert**

Create `src/scheduler.rs` with the event type, lock function, and `check_alert` extracted from daemon:

```rust
use crate::config::AppConfig;
use crate::marketplace::create_marketplace;
use crate::notifier::create_notifier;
use crate::types::{Alert, AlertResult, CheckStatus, Listing};
use anyhow::{Context, Result};
use chrono::Utc;
use fs2::FileExt;
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::time::Instant;
use tokio::sync::{mpsc, watch};
use tracing::error;
use uuid::Uuid;

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

pub fn try_acquire_scheduler_lock() -> Option<File> {
    let pid_path = crate::config::data_dir().join("daemon.pid");
    if let Some(parent) = pid_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&pid_path)
        .ok()?;

    if file.try_lock_exclusive().is_err() {
        return None;
    }

    let _ = std::fs::write(&pid_path, std::process::id().to_string());
    Some(file)
}

pub fn read_lock_pid() -> Option<u32> {
    let pid_path = crate::config::data_dir().join("daemon.pid");
    std::fs::read_to_string(&pid_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

pub async fn check_alert(
    alert: &Alert,
    existing_ids: &HashSet<String>,
    default_location: Option<&str>,
) -> Result<(CheckStatus, Vec<Listing>)> {
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

    let new_listings: Vec<Listing> = all_listings
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

    let status = CheckStatus {
        alert_id: alert.id,
        checked_at: Utc::now(),
        new_results: new_listings.len(),
        error: None,
    };

    Ok((status, new_listings))
}

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
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles (scheduler.rs is defined but not yet used by anyone)

- [ ] **Step 4: Commit**

```bash
git add src/scheduler.rs src/lib.rs
git commit -m "feat: add scheduler module with check_alert, SchedulerEvent, and PID lock"
```

---

### Task 2: Rewrite daemon to use Scheduler

**Files:**
- Modify: `src/daemon/mod.rs`

- [ ] **Step 1: Replace daemon/mod.rs**

Replace the entire content of `src/daemon/mod.rs`. The daemon now:
- Tries to acquire the scheduler lock (exits if held)
- Creates MPSC + watch channels
- Spawns the Scheduler
- Consumes events on the main thread, writing files and running notifiers

```rust
pub mod results;

use crate::config::{self, load_config};
use crate::scheduler::{self, Scheduler, SchedulerEvent};
use crate::types::CheckStatus;
use anyhow::{Context, Result};
use chrono::Utc;
use results::{load_results, load_status, results_path, save_results, save_status, status_path};
use std::collections::HashSet;
use std::path::Path;
use tokio::sync::{mpsc, watch};
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
                    let _ = save_results(&all_results, &results_path);
                }
                let _ = save_status(&statuses, &status_path);
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
                let _ = save_status(&statuses, &status_path);
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

        match scheduler::check_alert(alert, &existing_ids, config.settings.default_location.as_deref()).await {
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
                    let _ = save_results(&all_results, results_path);
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

    let _ = save_status(&statuses, status_path);
    Ok(())
}

fn upsert_status(statuses: &mut Vec<CheckStatus>, status: CheckStatus) {
    if let Some(existing) = statuses.iter_mut().find(|s| s.alert_id == status.alert_id) {
        *existing = status;
    } else {
        statuses.push(status);
    }
}
```

- [ ] **Step 2: Update daemon test**

In `tests/daemon_test.rs`, update `check_once_with_paths` calls to pass the additional `status_path` parameter:

```rust
snag::daemon::check_once_with_paths(&config_path, &results_path, &dir.path().join("status.json"))
    .await
    .unwrap();
```

Do this for both tests.

- [ ] **Step 3: Verify it compiles and tests pass**

Run: `cargo check && cargo test`
Expected: compiles, all tests pass

- [ ] **Step 4: Commit**

```bash
git add src/daemon/mod.rs tests/daemon_test.rs
git commit -m "refactor: rewrite daemon to use shared Scheduler module"
```

---

### Task 3: Wire Scheduler into TUI with Mtime Fallback

**Files:**
- Modify: `src/tui/app.rs`
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Remove auto_start_daemon from tui/mod.rs**

Replace `src/tui/mod.rs`:

```rust
pub mod app;
pub mod dialogs;
pub mod tabs;
pub mod theme;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new()?;
    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
```

- [ ] **Step 2: Update App struct and App::new() in tui/app.rs**

Read `src/tui/app.rs`. Make these changes:

Add new fields to the `App` struct (after `update_rx`):

```rust
scheduler_rx: Option<tokio::sync::mpsc::Receiver<crate::scheduler::SchedulerEvent>>,
config_tx: Option<tokio::sync::watch::Sender<AppConfig>>,
_scheduler_lock: Option<std::fs::File>,
last_results_mtime: Option<std::time::SystemTime>,
last_status_mtime: Option<std::time::SystemTime>,
```

Replace `App::new()` to try acquiring the lock and spawning the scheduler:

```rust
pub fn new() -> Result<Self> {
    let config_path = config::config_path();
    let results_path = results_path();
    let config = load_config(&config_path).unwrap_or_default();
    let results = load_results(&results_path).unwrap_or_default();
    let status_path = crate::daemon::results::status_path();
    let statuses = crate::daemon::results::load_status(&status_path).unwrap_or_default();

    let existing_ids: std::collections::HashSet<String> = results
        .iter()
        .flat_map(|r| r.listings.iter().map(|l| l.id.clone()))
        .collect();

    let (scheduler_rx, config_tx, scheduler_lock) =
        if let Some(lock) = crate::scheduler::try_acquire_scheduler_lock() {
            let (event_tx, event_rx) =
                tokio::sync::mpsc::channel::<crate::scheduler::SchedulerEvent>(64);
            let (cfg_tx, cfg_rx) = tokio::sync::watch::channel(config.clone());
            let scheduler =
                crate::scheduler::Scheduler::new(event_tx, cfg_rx, existing_ids);
            tokio::spawn(scheduler.run());
            (Some(event_rx), Some(cfg_tx), Some(lock))
        } else {
            (None, None, None)
        };

    let update_rx = if config.settings.check_for_updates {
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let result = crate::update::check_for_update().await.ok().flatten();
            let _ = tx.send(result);
        });
        Some(rx)
    } else {
        None
    };

    Ok(Self {
        active_tab: TabKind::Alerts,
        config,
        config_path,
        results,
        results_path,
        statuses,
        status_path,
        theme: Theme::default(),
        alerts_tab: AlertsTab::new(),
        results_tab: ResultsTab::new(),
        settings_tab: SettingsTab::new(),
        should_quit: false,
        active_dialog: None,
        update_info: None,
        update_rx,
        scheduler_rx,
        config_tx,
        _scheduler_lock: scheduler_lock,
        last_results_mtime: None,
        last_status_mtime: None,
    })
}
```

- [ ] **Step 3: Replace the file polling block in run() with dual-path logic**

Find the current file polling block in `run()`:

```rust
if last_results_refresh.elapsed() >= results_refresh_interval {
    if let Ok(new_results) = load_results(&self.results_path) {
        self.results = new_results;
    }
    if let Ok(new_statuses) = crate::daemon::results::load_status(&self.status_path) {
        self.statuses = new_statuses;
    }
    last_results_refresh = Instant::now();
}
```

Replace it with:

```rust
if let Some(ref mut rx) = self.scheduler_rx {
    while let Ok(event) = rx.try_recv() {
        match event {
            crate::scheduler::SchedulerEvent::CheckComplete { status, result } => {
                upsert_status(&mut self.statuses, status);
                if let Some(alert_result) = result {
                    self.results.push(alert_result);
                }
                let _ = crate::daemon::results::save_results(
                    &self.results,
                    &self.results_path,
                );
                let _ = crate::daemon::results::save_status(
                    &self.statuses,
                    &self.status_path,
                );
            }
            crate::scheduler::SchedulerEvent::CheckError { alert_id, error } => {
                upsert_status(
                    &mut self.statuses,
                    crate::types::CheckStatus {
                        alert_id,
                        checked_at: chrono::Utc::now(),
                        new_results: 0,
                        error: Some(error),
                    },
                );
                let _ = crate::daemon::results::save_status(
                    &self.statuses,
                    &self.status_path,
                );
            }
        }
    }
} else if last_results_refresh.elapsed() >= results_refresh_interval {
    let results_mtime = std::fs::metadata(&self.results_path)
        .and_then(|m| m.modified())
        .ok();
    if results_mtime != self.last_results_mtime {
        if let Ok(new_results) = load_results(&self.results_path) {
            self.results = new_results;
        }
        self.last_results_mtime = results_mtime;
    }

    let status_mtime = std::fs::metadata(&self.status_path)
        .and_then(|m| m.modified())
        .ok();
    if status_mtime != self.last_status_mtime {
        if let Ok(new_statuses) =
            crate::daemon::results::load_status(&self.status_path)
        {
            self.statuses = new_statuses;
        }
        self.last_status_mtime = status_mtime;
    }

    last_results_refresh = Instant::now();
}
```

- [ ] **Step 4: Add config push after every config save**

Find every place in `app.rs` that calls `save_config(&self.config, &self.config_path)`. After each one, add:

```rust
if let Some(ref tx) = self.config_tx {
    let _ = tx.send(self.config.clone());
}
```

There are multiple call sites — in the AlertsAction::ConfigChanged handler, the SettingsAction::ConfigChanged handler, and in handle_dialog_key after alert form submit. Add the config push after each `save_config` call.

- [ ] **Step 5: Add upsert_status helper function**

Add this function at the bottom of `app.rs` (outside the impl block):

```rust
fn upsert_status(statuses: &mut Vec<crate::types::CheckStatus>, status: crate::types::CheckStatus) {
    if let Some(existing) = statuses.iter_mut().find(|s| s.alert_id == status.alert_id) {
        *existing = status;
    } else {
        statuses.push(status);
    }
}
```

- [ ] **Step 6: Remove the daemon start/stop/restart actions from settings handler**

The Settings tab still has StopDaemon/RestartDaemon/StartDaemon actions that spawn child processes. Since the TUI now runs its own scheduler, these actions should be removed or repurposed. For now, remove the `libc::kill` and process spawning code — the settings tab's `[r]` and `[s]` keybindings just become no-ops (we can revisit daemon control later):

Replace the Settings match block with:

```rust
TabKind::Settings => {
    if let Some(action) = self.settings_tab.handle_key(key, &mut self.config) {
        match action {
            crate::tui::tabs::settings::SettingsAction::ConfigChanged => {
                let _ = save_config(&self.config, &self.config_path);
                if let Some(ref tx) = self.config_tx {
                    let _ = tx.send(self.config.clone());
                }
            }
            _ => {}
        }
    }
}
```

- [ ] **Step 7: Remove the libc import**

Remove the `#[allow(unused_imports)] use libc;` from the top of app.rs since we no longer use it.

- [ ] **Step 8: Verify it compiles and tests pass**

Run: `cargo check && cargo test`
Expected: compiles, all tests pass

- [ ] **Step 9: Commit**

```bash
git add src/tui/app.rs src/tui/mod.rs
git commit -m "feat: wire in-process scheduler into TUI with mtime fallback"
```

---

### Task 4: Final Verification

**Files:** None — verification only

- [ ] **Step 1: Run all tests**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings (fix any that appear)

- [ ] **Step 3: Build release**

Run: `cargo build --release`
Expected: builds successfully

- [ ] **Step 4: Commit any fixes**

```bash
git add -A
git commit -m "chore: fix clippy warnings and verify in-process scheduler"
```
