# In-Process Scheduler with Channel-Based Updates

Extract the scheduling logic into a shared `Scheduler` module. The first TUI instance (or `snag daemon`) acquires a PID lock and runs the scheduler, getting instant updates via MPSC channels. Additional TUI instances fall back to mtime-checked file reads — only re-parsing when files actually change.

## Architecture

The `Scheduler` runs in two modes:
1. **In-process (TUI):** sends events over MPSC channel, TUI receives via `try_recv()` in the event loop. Also writes files so other instances can read them.
2. **Standalone (daemon):** same scheduler, writes files and runs notifiers.

A PID-based lock (`daemon.pid`) ensures only one scheduler runs at a time across all processes. This follows the agent-of-empires pattern where each instance is independent and syncs via files on disk.

## New Module: `src/scheduler.rs`

Extracted from `src/daemon/mod.rs`. Contains:

### SchedulerEvent

```rust
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
```

### check_alert (moved from daemon)

The existing `check_alert` function moves here. It no longer writes to `results.json` directly — instead it returns the data needed for the caller to decide what to do with it.

Signature changes: instead of taking `results_path` and writing files internally, it takes `existing_ids: &HashSet<String>` (the set of already-seen listing IDs) and returns:

```rust
pub async fn check_alert(
    alert: &Alert,
    existing_ids: &HashSet<String>,
    default_location: Option<&str>,
) -> Result<(CheckStatus, Vec<Listing>)>
```

Returns the check status and any new (deduplicated) listings. The caller handles persistence and notification.

### Scheduler

```rust
pub struct Scheduler {
    event_tx: mpsc::Sender<SchedulerEvent>,
    config_rx: watch::Receiver<AppConfig>,
    last_check_times: HashMap<Uuid, Instant>,
    existing_ids: HashSet<String>,
}
```

- `event_tx` — sends events to the TUI (or a file-writing wrapper for daemon mode)
- `config_rx` — receives config updates from the TUI (uses `tokio::sync::watch` so the scheduler always sees the latest config without explicit push)
- `last_check_times` — tracks when each alert was last checked
- `existing_ids` — tracks seen listing IDs to deduplicate across checks

### Scheduler::run()

Async loop that:
1. Reads latest config from `config_rx`
2. For each enabled alert whose interval has elapsed, calls `check_alert`
3. Sends `SchedulerEvent::CheckComplete` or `SchedulerEvent::CheckError` over `event_tx`
4. Updates `last_check_times` and `existing_ids`
5. Sleeps 1 second between cycles
6. Exits when `event_tx` is dropped (receiver gone)

## PID Lock

### Lock Acquisition

Uses the existing `daemon.pid` file with `fs2` file locking (already a dependency):

```rust
pub fn try_acquire_scheduler_lock() -> Option<File>
```

Opens `daemon.pid` with an exclusive lock (`try_lock_exclusive`). If successful, writes PID and returns the `File` handle (lock held while handle is alive). If lock fails, returns `None`.

Lives in `src/scheduler.rs` alongside the scheduler.

### Who Checks

| Process | Lock acquired | Behavior |
|---|---|---|
| First TUI | Yes | Spawns in-process scheduler, MPSC channel, writes files |
| Second TUI | No | Mtime-checked file reads (only parses when changed) |
| `snag daemon` | Yes | Runs standalone scheduler with file output |
| `snag daemon` (lock held) | No | Prints "scheduler already running", exits |
| `snag check` | N/A | One-shot, no lock needed |

## TUI Changes

### App struct

```rust
pub struct App {
    // ... existing fields ...
    scheduler_rx: Option<mpsc::Receiver<SchedulerEvent>>,
    config_tx: Option<watch::Sender<AppConfig>>,
    _scheduler_lock: Option<File>,
    last_results_mtime: Option<std::time::SystemTime>,
    last_status_mtime: Option<std::time::SystemTime>,
}
```

- `scheduler_rx` — `Some` if this TUI owns the scheduler, `None` if fallback mode
- `config_tx` — sends config updates to the scheduler when user edits settings or alerts
- `_scheduler_lock` — holds the file lock for the lifetime of the App
- `last_results_mtime` / `last_status_mtime` — tracks file modification times for fallback mode, avoids re-parsing unchanged files

### App::new()

1. Try to acquire the scheduler lock
2. If acquired: create MPSC channel + watch channel, spawn scheduler task, store receiver
3. If not acquired: set `scheduler_rx = None`, enable mtime-checked file fallback
4. Load initial results and statuses from files (needed regardless of mode for startup state)

### Event loop — scheduler owner path

```rust
if let Some(ref mut rx) = self.scheduler_rx {
    while let Ok(event) = rx.try_recv() {
        match event {
            SchedulerEvent::CheckComplete { status, result } => {
                upsert_status(&mut self.statuses, status);
                if let Some(alert_result) = result {
                    self.results.push(alert_result);
                }
                let _ = save_results(&self.results, &self.results_path);
                let _ = save_status(&self.statuses, &self.status_path);
            }
            SchedulerEvent::CheckError { alert_id, error } => {
                upsert_status(&mut self.statuses, CheckStatus {
                    alert_id,
                    checked_at: Utc::now(),
                    new_results: 0,
                    error: Some(error),
                });
                let _ = save_status(&self.statuses, &self.status_path);
            }
        }
    }
}
```

### Event loop — fallback path (non-owner instances)

```rust
if self.scheduler_rx.is_none() && last_file_check.elapsed() >= file_check_interval {
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
        if let Ok(new_statuses) = load_status(&self.status_path) {
            self.statuses = new_statuses;
        }
        self.last_status_mtime = status_mtime;
    }

    last_file_check = Instant::now();
}
```

Checks mtime every 2 seconds. Only parses when the file has actually changed. A single `fs::metadata()` syscall is negligible.

### Config push

When the user saves config changes (alerts or settings), push to the scheduler if this instance owns it:

```rust
if let Some(ref tx) = self.config_tx {
    let _ = tx.send(self.config.clone());
}
```

### Remove auto_start_daemon

The TUI no longer spawns `snag daemon` as a child process. The in-process scheduler replaces it.

## Daemon Changes

`snag daemon` reuses `Scheduler` but wraps it differently:

1. Try to acquire scheduler lock — if held, print "scheduler already running (PID X)" and exit
2. Load config from file
3. Create MPSC channel, create watch channel
4. Spawn a config file watcher (polls config.toml mtime, pushes to watch channel on change)
5. Spawn scheduler, consume events on the main thread:

```rust
while let Some(event) = rx.recv().await {
    match event {
        SchedulerEvent::CheckComplete { status, result } => {
            // Write to results.json and status.json
            // Run notifiers for new results
        }
        SchedulerEvent::CheckError { ... } => {
            // Write to status.json
        }
    }
}
```

## Files Changed

| File | Change |
|---|---|
| `src/scheduler.rs` | NEW — Scheduler, SchedulerEvent, check_alert, PID lock |
| `src/lib.rs` | Add `pub mod scheduler` |
| `src/daemon/mod.rs` | Rewrite to use Scheduler, remove check_alert (moved) |
| `src/tui/app.rs` | Add scheduler_rx/config_tx, channel event handling, mtime fallback |
| `src/tui/mod.rs` | Remove auto_start_daemon |

## What Stays the Same

- `results.json` and `status.json` still written for persistence and cross-instance reads
- `snag check` unchanged — one-shot, calls `check_alert` directly
- `check_alert` logic unchanged — just moved to `scheduler.rs`
- All existing tests remain valid
