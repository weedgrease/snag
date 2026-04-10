# Snag — Marketplace Alert CLI Tool

A terminal UI tool for configuring and managing alerts across online marketplaces (eBay, Facebook Marketplace, etc.) with a background daemon that polls for new listings and delivers notifications.

## Architecture

Single Rust binary (`snag`) with three modes:

- `snag` — launches the TUI for managing alerts and viewing results
- `snag daemon` — runs the background polling service
- `snag check` — one-shot check of all enabled alerts (useful for testing)

All modes share the same library code: config loading, marketplace adapters, notifiers, and core types.

### Data Flow

The TUI and daemon communicate through the filesystem:

- **Config:** `~/.config/snag/config.toml` — alerts, global settings, notifier config. Written by the TUI, read by the daemon.
- **Results:** `~/.local/share/snag/results.json` — matched listings with metadata. Written by the daemon, read by the TUI.
- **PID file:** `~/.local/share/snag/daemon.pid` — daemon process tracking.
- **Logs:** `~/.local/share/snag/daemon.log` — daemon log output.

File locking via the `fs2` crate prevents corruption from concurrent access.

## Core Traits

### Marketplace Adapter

```rust
#[async_trait]
pub trait Marketplace: Send + Sync {
    fn name(&self) -> &str;
    fn supported_filters(&self) -> &[FilterKind];
    async fn search(&self, alert: &Alert) -> Result<Vec<Listing>>;
}
```

Each marketplace declares which filters it supports via `FilterKind`. The TUI uses this to show/hide filter fields when creating alerts. Each adapter manages its own rate limiting internally, with exponential backoff on rate limit responses.

### Notifier

```rust
#[async_trait]
pub trait Notifier: Send + Sync {
    fn name(&self) -> &str;
    async fn notify(&self, alert: &Alert, listings: &[Listing]) -> Result<()>;
}
```

Terminal notifier ships as the default. The trait enables future implementations: Telegram, Discord, email, push notifications.

## Data Model

### Alert

```rust
pub struct Alert {
    pub id: Uuid,
    pub name: String,
    pub marketplaces: Vec<MarketplaceKind>,
    pub keywords: Vec<String>,
    pub exclude_keywords: Vec<String>,
    pub price_min: Option<f64>,
    pub price_max: Option<f64>,
    pub location: Option<String>,
    pub radius_miles: Option<u32>,
    pub condition: Option<Condition>,
    pub category: Option<String>,
    pub check_interval: Duration,
    pub notifiers: Vec<NotifierKind>,
    pub max_results: Option<u32>,
    pub enabled: bool,
}
```

### Listing

```rust
pub struct Listing {
    pub id: String,
    pub title: String,
    pub price: Option<f64>,
    pub currency: String,
    pub url: String,
    pub image_url: Option<String>,
    pub location: Option<String>,
    pub condition: Option<Condition>,
    pub marketplace: MarketplaceKind,
    pub posted_at: Option<DateTime<Utc>>,
    pub found_at: DateTime<Utc>,
}
```

### AlertResult

```rust
pub struct AlertResult {
    pub alert_id: Uuid,
    pub alert_name: String,
    pub listings: Vec<Listing>,
    pub checked_at: DateTime<Utc>,
    pub seen: bool,
}
```

### Enums

```rust
pub enum MarketplaceKind {
    Ebay,
    FacebookMarketplace,
}

pub enum NotifierKind {
    Terminal,
}

pub enum Condition {
    New,
    LikeNew,
    Used,
    ForParts,
}

pub enum FilterKind {
    PriceRange,
    Location,
    Condition,
    Category,
}
```

## TUI Design

Built with Ratatui + Crossterm, following the agent-of-empires pattern.

### Tab Structure

Three tabs, switchable via number keys or arrow keys:

**Alerts Tab** — split-pane layout. Left: scrollable list of alerts with enabled/disabled indicators. Right: detail panel showing the selected alert's full configuration and last check stats. Keybindings: `n` new, `e` edit, `d` delete, `space` toggle enabled, `/` search.

**Results Tab** — split-pane layout. Left: scrollable list of matched listings, newest first, with unread indicators. Right: detail panel showing price, marketplace, location, condition, post time, and the originating alert. Keybindings: `o` open in browser, `m` mark read, `c` clear, `/` search, `f` filter by alert.

**Settings Tab** — form layout. Daemon status and control (start/stop/restart via PID signals). Default values for check interval, max results, notification method. Theme selection.

### Input Handling

Follows the agent-of-empires hierarchical input pattern:

1. Active dialog consumes input first
2. If no dialog, the active tab handles input
3. Global keys (tab switching, quit) handled at the app level

### Dialogs

- **Alert form** — modal dialog for creating/editing alerts. Multi-field form with the fields from the Alert struct.
- **Confirm dialog** — yes/no confirmation for destructive actions (delete alert, clear results).

### Event Loop

50ms poll timeout for responsiveness. Immediate redraw on input. Periodic refresh of results file to pick up new daemon matches. Single draw per cycle to avoid flicker.

## Daemon Design

### Startup

Loads config, registers marketplace adapters and notifiers, writes PID file. Uses `tracing` for structured logging to `daemon.log`.

### Scheduler

Tokio-based async runtime. Tracks last check time per alert. Fires searches when an alert's interval elapses. Multiple alerts targeting the same marketplace are staggered to respect rate limits.

### Search Flow

For each due alert:
1. Call `search()` on each configured marketplace adapter
2. Deduplicate results by listing ID
3. Compare against previously seen listings in `results.json`
4. Write new matches to `results.json`
5. Send new matches through configured notifiers

### Config Hot-Reload

Polls `config.toml` for changes (modification timestamp check). When the TUI saves new alert configuration, the daemon picks up changes without requiring a restart.

### Auto-Start

When the TUI launches, it checks for the PID file. If the daemon isn't running (no PID file, or stale PID pointing to a dead process), it spawns the daemon automatically.

### Rate Limiting

Each marketplace adapter manages its own rate limits internally. On rate limit responses (HTTP 429 or equivalent), the adapter backs off exponentially. Configurable per-adapter with sensible defaults.

## Error Handling

- **Adapter failures are isolated.** If one marketplace is down, other marketplaces and alerts continue checking normally. Failures are logged and retried on the next interval.
- **Config validation on load.** Unknown marketplaces or notifiers in the config produce warnings, not crashes. The invalid entries are skipped.
- **Daemon resilience.** The scheduler loop recovers from individual alert check failures. The daemon stays up.
- **Stale PID cleanup.** If a PID file exists but the process is dead, the TUI cleans it up and offers to restart the daemon.

## Project Structure

```
snag/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs
│   ├── types.rs
│   ├── marketplace/
│   │   ├── mod.rs
│   │   └── providers/
│   │       ├── mod.rs
│   │       ├── ebay.rs
│   │       └── facebook.rs
│   ├── notifier/
│   │   ├── mod.rs
│   │   └── providers/
│   │       ├── mod.rs
│   │       └── terminal.rs
│   ├── daemon/
│   │   ├── mod.rs
│   │   └── results.rs
│   └── tui/
│       ├── mod.rs
│       ├── app.rs
│       ├── tabs/
│       │   ├── alerts.rs
│       │   ├── results.rs
│       │   └── settings.rs
│       ├── dialogs/
│       │   ├── mod.rs
│       │   ├── alert_form.rs
│       │   └── confirm.rs
│       └── theme.rs
```

## Dependencies

- `ratatui` + `crossterm` — TUI framework
- `tokio` — async runtime for daemon and TUI background operations
- `clap` — CLI argument parsing
- `serde` + `toml` — config serialization
- `serde_json` — results serialization
- `chrono` — timestamps
- `uuid` — alert IDs
- `directories` — XDG base directory paths
- `fs2` — file locking
- `tracing` + `tracing-subscriber` — structured logging
- `async-trait` — async trait support
- `reqwest` — HTTP client for marketplace adapters
- `notify` (optional) — filesystem watching for config hot-reload
