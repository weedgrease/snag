# Snag Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI tool with a TUI for configuring marketplace alerts and a background daemon that polls for new listings.

**Architecture:** Single binary with three subcommands (TUI, daemon, check). Trait-based marketplace adapters and notifiers for extensibility. TUI and daemon communicate through shared TOML config and JSON results files on disk.

**Tech Stack:** Rust, Ratatui, Crossterm, Tokio, Clap, Serde, reqwest

---

## File Structure

```
snag/
├── Cargo.toml
├── src/
│   ├── main.rs                        — CLI entry point, clap subcommands
│   ├── lib.rs                         — re-exports all modules
│   ├── config.rs                      — config loading, saving, XDG paths
│   ├── types.rs                       — Alert, Listing, AlertResult, enums
│   ├── marketplace/
│   │   ├── mod.rs                     — Marketplace trait, FilterKind, registry
│   │   └── providers/
│   │       ├── mod.rs                 — provider re-exports
│   │       ├── ebay.rs                — eBay adapter (stub)
│   │       └── facebook.rs            — Facebook Marketplace adapter (stub)
│   ├── notifier/
│   │   ├── mod.rs                     — Notifier trait, registry
│   │   └── providers/
│   │       ├── mod.rs                 — provider re-exports
│   │       └── terminal.rs            — terminal notifier
│   ├── daemon/
│   │   ├── mod.rs                     — daemon entry, scheduler loop
│   │   └── results.rs                 — results file read/write with locking
│   └── tui/
│       ├── mod.rs                     — terminal setup/teardown, run()
│       ├── app.rs                     — App struct, event loop, tab routing
│       ├── theme.rs                   — color definitions
│       ├── tabs/
│       │   ├── mod.rs                 — Tab trait, TabKind enum
│       │   ├── alerts.rs              — alerts tab: list, detail, input, render
│       │   ├── results.rs             — results tab: list, detail, input, render
│       │   └── settings.rs            — settings tab: form, daemon control
│       └── dialogs/
│           ├── mod.rs                 — DialogResult enum, shared helpers
│           ├── alert_form.rs          — create/edit alert modal
│           └── confirm.rs             — yes/no confirmation modal
├── tests/
│   ├── config_test.rs
│   ├── types_test.rs
│   ├── results_test.rs
│   └── daemon_test.rs
```

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `.gitignore`

- [ ] **Step 1: Initialize the Rust project**

Run: `cargo init --name snag /home/kevin/repositories/snag`

- [ ] **Step 2: Replace Cargo.toml with full dependencies**

Replace the generated `Cargo.toml` with:

```toml
[package]
name = "snag"
version = "0.1.0"
edition = "2024"
description = "CLI tool for marketplace listing alerts"

[dependencies]
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
crossterm = "0.29"
directories = "6"
fs2 = "0.4"
ratatui = { version = "0.30", features = ["crossterm"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["v4", "serde"] }
anyhow = "1"
open = "5"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Write main.rs with clap subcommands**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "snag", about = "Marketplace listing alerts")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Daemon,
    Check,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => snag::tui::run().await,
        Some(Commands::Daemon) => snag::daemon::run().await,
        Some(Commands::Check) => snag::daemon::check_once().await,
    }
}
```

- [ ] **Step 4: Write lib.rs with module declarations**

```rust
pub mod config;
pub mod daemon;
pub mod marketplace;
pub mod notifier;
pub mod tui;
pub mod types;
```

- [ ] **Step 5: Create stub modules so it compiles**

Create `src/config.rs`:
```rust
pub fn placeholder() {}
```

Create `src/types.rs`:
```rust
pub fn placeholder() {}
```

Create `src/marketplace/mod.rs`:
```rust
pub mod providers;
pub fn placeholder() {}
```

Create `src/marketplace/providers/mod.rs`:
```rust
pub fn placeholder() {}
```

Create `src/notifier/mod.rs`:
```rust
pub mod providers;
pub fn placeholder() {}
```

Create `src/notifier/providers/mod.rs`:
```rust
pub fn placeholder() {}
```

Create `src/daemon/mod.rs`:
```rust
pub async fn run() -> anyhow::Result<()> {
    todo!()
}

pub async fn check_once() -> anyhow::Result<()> {
    todo!()
}
```

Create `src/daemon/results.rs`:
```rust
pub fn placeholder() {}
```

Create `src/tui/mod.rs`:
```rust
pub async fn run() -> anyhow::Result<()> {
    todo!()
}
```

Create `.gitignore`:
```
/target
.superpowers/
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors (warnings about unused code are fine)

- [ ] **Step 7: Initialize git and commit**

```bash
cd /home/kevin/repositories/snag
git init
git add Cargo.toml src/ .gitignore docs/
git commit -m "feat: scaffold snag project with clap CLI and module stubs"
```

---

### Task 2: Core Types

**Files:**
- Create: `src/types.rs`
- Create: `tests/types_test.rs`

- [ ] **Step 1: Write tests for type serialization**

Create `tests/types_test.rs`:

```rust
use snag::types::*;
use uuid::Uuid;
use chrono::Utc;
use std::time::Duration;

#[test]
fn alert_round_trips_through_toml() {
    let alert = Alert {
        id: Uuid::nil(),
        name: "Test Alert".into(),
        marketplaces: vec![MarketplaceKind::Ebay],
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
    };

    let toml_str = toml::to_string(&alert).unwrap();
    let deserialized: Alert = toml::from_str(&toml_str).unwrap();

    assert_eq!(deserialized.name, "Test Alert");
    assert_eq!(deserialized.keywords, vec!["ps5"]);
    assert_eq!(deserialized.exclude_keywords, vec!["broken"]);
    assert_eq!(deserialized.price_min, Some(100.0));
    assert_eq!(deserialized.marketplaces, vec![MarketplaceKind::Ebay]);
    assert_eq!(deserialized.condition, Some(Condition::Used));
    assert!(deserialized.enabled);
}

#[test]
fn listing_round_trips_through_json() {
    let listing = Listing {
        id: "ebay-123".into(),
        title: "PS5 Console".into(),
        price: Some(299.99),
        currency: "USD".into(),
        url: "https://ebay.com/item/123".into(),
        image_url: Some("https://ebay.com/img/123.jpg".into()),
        location: Some("Denver, CO".into()),
        condition: Some(Condition::Used),
        marketplace: MarketplaceKind::Ebay,
        posted_at: Some(Utc::now()),
        found_at: Utc::now(),
    };

    let json = serde_json::to_string(&listing).unwrap();
    let deserialized: Listing = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "ebay-123");
    assert_eq!(deserialized.title, "PS5 Console");
    assert_eq!(deserialized.price, Some(299.99));
    assert_eq!(deserialized.marketplace, MarketplaceKind::Ebay);
}

#[test]
fn alert_result_round_trips_through_json() {
    let result = AlertResult {
        alert_id: Uuid::nil(),
        alert_name: "Test".into(),
        listings: vec![],
        checked_at: Utc::now(),
        seen: false,
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: AlertResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.alert_name, "Test");
    assert!(!deserialized.seen);
}

#[test]
fn alert_with_minimal_fields() {
    let alert = Alert {
        id: Uuid::nil(),
        name: "Bare Alert".into(),
        marketplaces: vec![MarketplaceKind::FacebookMarketplace],
        keywords: vec!["couch".into()],
        exclude_keywords: vec![],
        price_min: None,
        price_max: None,
        location: None,
        radius_miles: None,
        condition: None,
        category: None,
        check_interval: Duration::from_secs(600),
        notifiers: vec![NotifierKind::Terminal],
        max_results: None,
        enabled: true,
    };

    let toml_str = toml::to_string(&alert).unwrap();
    let deserialized: Alert = toml::from_str(&toml_str).unwrap();

    assert_eq!(deserialized.name, "Bare Alert");
    assert_eq!(deserialized.price_min, None);
    assert_eq!(deserialized.location, None);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test types_test`
Expected: FAIL — types don't exist yet

- [ ] **Step 3: Implement types.rs**

Replace `src/types.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(with = "duration_secs")]
    pub check_interval: Duration,
    pub notifiers: Vec<NotifierKind>,
    pub max_results: Option<u32>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertResult {
    pub alert_id: Uuid,
    pub alert_name: String,
    pub listings: Vec<Listing>,
    pub checked_at: DateTime<Utc>,
    pub seen: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketplaceKind {
    Ebay,
    FacebookMarketplace,
}

impl std::fmt::Display for MarketplaceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ebay => write!(f, "eBay"),
            Self::FacebookMarketplace => write!(f, "Facebook Marketplace"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotifierKind {
    Terminal,
}

impl std::fmt::Display for NotifierKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Terminal => write!(f, "Terminal"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    New,
    LikeNew,
    Used,
    ForParts,
}

impl std::fmt::Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New => write!(f, "New"),
            Self::LikeNew => write!(f, "Like New"),
            Self::Used => write!(f, "Used"),
            Self::ForParts => write!(f, "For Parts"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterKind {
    PriceRange,
    Location,
    Condition,
    Category,
}

mod duration_secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test types_test`
Expected: all 4 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/types.rs tests/types_test.rs
git commit -m "feat: add core types with serde serialization"
```

---

### Task 3: Config Module

**Files:**
- Create: `src/config.rs`
- Create: `tests/config_test.rs`

- [ ] **Step 1: Write tests for config load/save**

Create `tests/config_test.rs`:

```rust
use snag::config::{AppConfig, GlobalSettings, load_config, save_config};
use snag::types::*;
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
        },
        alerts: vec![
            Alert {
                id: Uuid::nil(),
                name: "Test Alert".into(),
                marketplaces: vec![MarketplaceKind::Ebay],
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
fn save_config_creates_parent_directories() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("nested").join("deep").join("config.toml");

    let config = AppConfig::default();
    save_config(&config, &config_path).unwrap();

    assert!(config_path.exists());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test config_test`
Expected: FAIL — config module not implemented

- [ ] **Step 3: Implement config.rs**

Replace `src/config.rs`:

```rust
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
```

- [ ] **Step 4: Make duration_secs module public**

In `src/types.rs`, change `mod duration_secs` to `pub mod duration_secs` so `config.rs` can reference it.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test config_test`
Expected: all 3 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/config.rs src/types.rs tests/config_test.rs
git commit -m "feat: add config module with TOML load/save and XDG paths"
```

---

### Task 4: Marketplace Trait and Stub Providers

**Files:**
- Create: `src/marketplace/mod.rs`
- Create: `src/marketplace/providers/mod.rs`
- Create: `src/marketplace/providers/ebay.rs`
- Create: `src/marketplace/providers/facebook.rs`

- [ ] **Step 1: Implement the Marketplace trait and registry**

Replace `src/marketplace/mod.rs`:

```rust
pub mod providers;

use crate::types::{Alert, FilterKind, Listing, MarketplaceKind};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Marketplace: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> MarketplaceKind;
    fn supported_filters(&self) -> &[FilterKind];
    async fn search(&self, alert: &Alert) -> Result<Vec<Listing>>;
}

pub fn create_marketplace(kind: MarketplaceKind) -> Box<dyn Marketplace> {
    match kind {
        MarketplaceKind::Ebay => Box::new(providers::ebay::EbayMarketplace::new()),
        MarketplaceKind::FacebookMarketplace => {
            Box::new(providers::facebook::FacebookMarketplace::new())
        }
    }
}
```

- [ ] **Step 2: Implement stub eBay provider**

Replace `src/marketplace/providers/ebay.rs`:

```rust
use crate::marketplace::Marketplace;
use crate::types::{Alert, FilterKind, Listing, MarketplaceKind};
use anyhow::Result;
use async_trait::async_trait;

pub struct EbayMarketplace;

impl EbayMarketplace {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Marketplace for EbayMarketplace {
    fn name(&self) -> &str {
        "eBay"
    }

    fn kind(&self) -> MarketplaceKind {
        MarketplaceKind::Ebay
    }

    fn supported_filters(&self) -> &[FilterKind] {
        &[
            FilterKind::PriceRange,
            FilterKind::Condition,
            FilterKind::Category,
            FilterKind::Location,
        ]
    }

    async fn search(&self, _alert: &Alert) -> Result<Vec<Listing>> {
        tracing::warn!("eBay search not yet implemented, returning empty results");
        Ok(vec![])
    }
}
```

- [ ] **Step 3: Implement stub Facebook Marketplace provider**

Replace `src/marketplace/providers/facebook.rs`:

```rust
use crate::marketplace::Marketplace;
use crate::types::{Alert, FilterKind, Listing, MarketplaceKind};
use anyhow::Result;
use async_trait::async_trait;

pub struct FacebookMarketplace;

impl FacebookMarketplace {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Marketplace for FacebookMarketplace {
    fn name(&self) -> &str {
        "Facebook Marketplace"
    }

    fn kind(&self) -> MarketplaceKind {
        MarketplaceKind::FacebookMarketplace
    }

    fn supported_filters(&self) -> &[FilterKind] {
        &[
            FilterKind::PriceRange,
            FilterKind::Location,
            FilterKind::Category,
        ]
    }

    async fn search(&self, _alert: &Alert) -> Result<Vec<Listing>> {
        tracing::warn!("Facebook Marketplace search not yet implemented, returning empty results");
        Ok(vec![])
    }
}
```

- [ ] **Step 4: Update providers/mod.rs**

Replace `src/marketplace/providers/mod.rs`:

```rust
pub mod ebay;
pub mod facebook;
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add src/marketplace/
git commit -m "feat: add marketplace trait and stub eBay/Facebook providers"
```

---

### Task 5: Notifier Trait and Terminal Provider

**Files:**
- Create: `src/notifier/mod.rs`
- Create: `src/notifier/providers/mod.rs`
- Create: `src/notifier/providers/terminal.rs`

- [ ] **Step 1: Implement the Notifier trait and registry**

Replace `src/notifier/mod.rs`:

```rust
pub mod providers;

use crate::types::{Alert, Listing, NotifierKind};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Notifier: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> NotifierKind;
    async fn notify(&self, alert: &Alert, listings: &[Listing]) -> Result<()>;
}

pub fn create_notifier(kind: NotifierKind) -> Box<dyn Notifier> {
    match kind {
        NotifierKind::Terminal => Box::new(providers::terminal::TerminalNotifier::new()),
    }
}
```

- [ ] **Step 2: Implement terminal notifier**

Replace `src/notifier/providers/terminal.rs`:

```rust
use crate::notifier::Notifier;
use crate::types::{Alert, Listing, NotifierKind};
use anyhow::Result;
use async_trait::async_trait;

pub struct TerminalNotifier;

impl TerminalNotifier {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Notifier for TerminalNotifier {
    fn name(&self) -> &str {
        "Terminal"
    }

    fn kind(&self) -> NotifierKind {
        NotifierKind::Terminal
    }

    async fn notify(&self, alert: &Alert, listings: &[Listing]) -> Result<()> {
        for listing in listings {
            let price_str = listing
                .price
                .map(|p| format!(" — ${:.2}", p))
                .unwrap_or_default();

            tracing::info!(
                "[{}] New match: {}{}  {}",
                alert.name,
                listing.title,
                price_str,
                listing.url,
            );
        }
        Ok(())
    }
}
```

- [ ] **Step 3: Update providers/mod.rs**

Replace `src/notifier/providers/mod.rs`:

```rust
pub mod terminal;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add src/notifier/
git commit -m "feat: add notifier trait and terminal notifier provider"
```

---

### Task 6: Results File Read/Write

**Files:**
- Create: `src/daemon/results.rs`
- Create: `tests/results_test.rs`

- [ ] **Step 1: Write tests for results read/write with locking**

Create `tests/results_test.rs`:

```rust
use chrono::Utc;
use snag::daemon::results::{load_results, save_results};
use snag::types::*;
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn save_and_load_results_round_trips() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("results.json");

    let results = vec![AlertResult {
        alert_id: Uuid::nil(),
        alert_name: "Test".into(),
        listings: vec![Listing {
            id: "ebay-1".into(),
            title: "PS5".into(),
            price: Some(300.0),
            currency: "USD".into(),
            url: "https://ebay.com/1".into(),
            image_url: None,
            location: Some("Denver".into()),
            condition: Some(Condition::Used),
            marketplace: MarketplaceKind::Ebay,
            posted_at: None,
            found_at: Utc::now(),
        }],
        checked_at: Utc::now(),
        seen: false,
    }];

    save_results(&results, &path).unwrap();
    let loaded = load_results(&path).unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].alert_name, "Test");
    assert_eq!(loaded[0].listings.len(), 1);
    assert_eq!(loaded[0].listings[0].title, "PS5");
}

#[test]
fn load_missing_results_returns_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.json");

    let results = load_results(&path).unwrap();
    assert!(results.is_empty());
}

#[test]
fn save_results_creates_parent_directories() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nested").join("results.json");

    save_results(&vec![], &path).unwrap();
    assert!(path.exists());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test results_test`
Expected: FAIL — results module not implemented

- [ ] **Step 3: Implement results.rs**

Replace `src/daemon/results.rs`:

```rust
use crate::types::AlertResult;
use anyhow::{Context, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub fn results_path() -> std::path::PathBuf {
    crate::config::data_dir().join("results.json")
}

pub fn load_results(path: &Path) -> Result<Vec<AlertResult>> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let mut file = File::open(path)
        .with_context(|| format!("failed to open results at {}", path.display()))?;

    file.lock_shared()
        .context("failed to acquire shared lock on results")?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .context("failed to read results")?;

    file.unlock().context("failed to release lock on results")?;

    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    let results: Vec<AlertResult> =
        serde_json::from_str(&content).context("failed to parse results")?;

    Ok(results)
}

pub fn save_results(results: &[AlertResult], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create data directory {}", parent.display()))?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("failed to open results for writing at {}", path.display()))?;

    file.lock_exclusive()
        .context("failed to acquire exclusive lock on results")?;

    let content = serde_json::to_string_pretty(results).context("failed to serialize results")?;

    file.write_all(content.as_bytes())
        .context("failed to write results")?;

    file.unlock()
        .context("failed to release lock on results")?;

    Ok(())
}
```

- [ ] **Step 4: Update daemon/mod.rs to export results**

Replace `src/daemon/mod.rs`:

```rust
pub mod results;

pub async fn run() -> anyhow::Result<()> {
    todo!()
}

pub async fn check_once() -> anyhow::Result<()> {
    todo!()
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test results_test`
Expected: all 3 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/daemon/ tests/results_test.rs
git commit -m "feat: add results file read/write with file locking"
```

---

### Task 7: Daemon Scheduler

**Files:**
- Modify: `src/daemon/mod.rs`
- Create: `tests/daemon_test.rs`

- [ ] **Step 1: Write test for daemon check_once**

Create `tests/daemon_test.rs`:

```rust
use snag::config::{AppConfig, GlobalSettings, save_config};
use snag::daemon::results::load_results;
use snag::types::*;
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn check_once_writes_results_for_enabled_alerts() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    let results_path = dir.path().join("results.json");

    let config = AppConfig {
        settings: GlobalSettings {
            default_check_interval: Duration::from_secs(300),
            default_max_results: Some(20),
            default_notifier: NotifierKind::Terminal,
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
            check_interval: Duration::from_secs(300),
            notifiers: vec![NotifierKind::Terminal],
            max_results: None,
            enabled: true,
        }],
    };

    save_config(&config, &config_path).unwrap();

    snag::daemon::check_once_with_paths(&config_path, &results_path)
        .await
        .unwrap();

    let results = load_results(&results_path).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].alert_name, "Test Alert");
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
        },
        alerts: vec![Alert {
            id: Uuid::new_v4(),
            name: "Disabled Alert".into(),
            marketplaces: vec![MarketplaceKind::Ebay],
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

    snag::daemon::check_once_with_paths(&config_path, &results_path)
        .await
        .unwrap();

    let results = load_results(&results_path).unwrap();
    assert!(results.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test daemon_test`
Expected: FAIL — `check_once_with_paths` doesn't exist

- [ ] **Step 3: Implement daemon/mod.rs**

Replace `src/daemon/mod.rs`:

```rust
pub mod results;

use crate::config::{self, load_config, AppConfig};
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

                    if let Err(e) = check_alert(alert, results_path).await {
                        error!("failed to check alert '{}': {e}", alert.name);
                    }

                    last_check_times.insert(alert.id, Instant::now());
                }
            }
        }
    }

    Ok(())
}

async fn check_alert(alert: &crate::types::Alert, results_path: &Path) -> Result<()> {
    let mut all_listings = vec![];

    for marketplace_kind in &alert.marketplaces {
        let marketplace = create_marketplace(*marketplace_kind);
        match marketplace.search(alert).await {
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

    if let Some(max) = alert.max_results {
        let new_listings = if new_listings.len() > max as usize {
            new_listings[..max as usize].to_vec()
        } else {
            new_listings.clone()
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

        return Ok(());
    }

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

        check_alert(alert, results_path).await?;
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test daemon_test`
Expected: all 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/daemon/mod.rs tests/daemon_test.rs
git commit -m "feat: add daemon scheduler with check_once and run loop"
```

---

### Task 8: TUI Shell — Terminal Setup, App Struct, Event Loop

**Files:**
- Create: `src/tui/mod.rs`
- Create: `src/tui/app.rs`
- Create: `src/tui/theme.rs`
- Create: `src/tui/tabs/mod.rs`

- [ ] **Step 1: Implement theme.rs**

Create `src/tui/theme.rs`:

```rust
use ratatui::style::Color;

pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub accent: Color,
    pub active_tab: Color,
    pub inactive_tab: Color,
    pub border: Color,
    pub selected_bg: Color,
    pub enabled: Color,
    pub disabled: Color,
    pub unread: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::White,
            fg_dim: Color::DarkGray,
            accent: Color::Cyan,
            active_tab: Color::Cyan,
            inactive_tab: Color::DarkGray,
            border: Color::DarkGray,
            selected_bg: Color::Rgb(40, 40, 60),
            enabled: Color::Green,
            disabled: Color::Red,
            unread: Color::Yellow,
            status_bar_bg: Color::Rgb(30, 30, 50),
            status_bar_fg: Color::White,
        }
    }
}
```

- [ ] **Step 2: Implement tabs/mod.rs with TabKind**

Create `src/tui/tabs/mod.rs`:

```rust
pub mod alerts;
pub mod results;
pub mod settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Alerts,
    Results,
    Settings,
}

impl TabKind {
    pub fn all() -> &'static [TabKind] {
        &[TabKind::Alerts, TabKind::Results, TabKind::Settings]
    }

    pub fn title(&self) -> &str {
        match self {
            TabKind::Alerts => "Alerts",
            TabKind::Results => "Results",
            TabKind::Settings => "Settings",
        }
    }

    pub fn next(&self) -> TabKind {
        match self {
            TabKind::Alerts => TabKind::Results,
            TabKind::Results => TabKind::Settings,
            TabKind::Settings => TabKind::Alerts,
        }
    }

    pub fn prev(&self) -> TabKind {
        match self {
            TabKind::Alerts => TabKind::Settings,
            TabKind::Results => TabKind::Alerts,
            TabKind::Settings => TabKind::Results,
        }
    }
}
```

- [ ] **Step 3: Create stub tab files**

Create `src/tui/tabs/alerts.rs`:
```rust
use crate::config::AppConfig;
use crate::tui::theme::Theme;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

pub struct AlertsTab {
    pub selected: usize,
    pub list_state: ratatui::widgets::ListState,
}

impl AlertsTab {
    pub fn new() -> Self {
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(0));
        Self { selected: 0, list_state }
    }

    pub fn handle_key(&mut self, _key: KeyEvent, _config: &mut AppConfig) -> Option<AlertsAction> {
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig) {
        let _ = (frame, area, theme, config);
    }
}

pub enum AlertsAction {
    ConfigChanged,
    CreateAlert,
    EditAlert(usize),
    DeleteAlert(usize),
}
```

Create `src/tui/tabs/results.rs`:
```rust
use crate::tui::theme::Theme;
use crate::types::AlertResult;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

pub struct ResultsTab {
    pub selected: usize,
    pub list_state: ratatui::widgets::ListState,
}

impl ResultsTab {
    pub fn new() -> Self {
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(0));
        Self { selected: 0, list_state }
    }

    pub fn handle_key(&mut self, _key: KeyEvent, _results: &mut Vec<AlertResult>) -> Option<ResultsAction> {
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, results: &[AlertResult]) {
        let _ = (frame, area, theme, results);
    }
}

pub enum ResultsAction {
    OpenUrl(String),
    ResultsChanged,
}
```

Create `src/tui/tabs/settings.rs`:
```rust
use crate::config::AppConfig;
use crate::tui::theme::Theme;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

pub struct SettingsTab {
    pub selected: usize,
}

impl SettingsTab {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn handle_key(&mut self, _key: KeyEvent, _config: &mut AppConfig) -> Option<SettingsAction> {
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig) {
        let _ = (frame, area, theme, config);
    }
}

pub enum SettingsAction {
    StartDaemon,
    StopDaemon,
    RestartDaemon,
}
```

- [ ] **Step 4: Implement app.rs with event loop**

Create `src/tui/app.rs`:

```rust
use crate::config::{self, AppConfig, load_config, save_config};
use crate::daemon::results::{load_results, results_path};
use crate::tui::tabs::alerts::AlertsTab;
use crate::tui::tabs::results::ResultsTab;
use crate::tui::tabs::settings::SettingsTab;
use crate::tui::tabs::TabKind;
use crate::tui::theme::Theme;
use crate::types::AlertResult;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Tabs};
use ratatui::Frame;
use std::time::{Duration, Instant};

pub struct App {
    pub active_tab: TabKind,
    pub config: AppConfig,
    pub config_path: std::path::PathBuf,
    pub results: Vec<AlertResult>,
    pub results_path: std::path::PathBuf,
    pub theme: Theme,
    pub alerts_tab: AlertsTab,
    pub results_tab: ResultsTab,
    pub settings_tab: SettingsTab,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let config_path = config::config_path();
        let results_path = results_path();
        let config = load_config(&config_path).unwrap_or_default();
        let results = load_results(&results_path).unwrap_or_default();

        Ok(Self {
            active_tab: TabKind::Alerts,
            config,
            config_path,
            results,
            results_path,
            theme: Theme::default(),
            alerts_tab: AlertsTab::new(),
            results_tab: ResultsTab::new(),
            settings_tab: SettingsTab::new(),
            should_quit: false,
        })
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        let mut last_results_refresh = Instant::now();
        let results_refresh_interval = Duration::from_secs(2);

        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.should_quit = true;
                    } else if key.code == KeyCode::Char('q') {
                        self.should_quit = true;
                    } else if key.code == KeyCode::Tab {
                        self.active_tab = self.active_tab.next();
                    } else if key.code == KeyCode::BackTab {
                        self.active_tab = self.active_tab.prev();
                    } else if key.code == KeyCode::Char('1') {
                        self.active_tab = TabKind::Alerts;
                    } else if key.code == KeyCode::Char('2') {
                        self.active_tab = TabKind::Results;
                    } else if key.code == KeyCode::Char('3') {
                        self.active_tab = TabKind::Settings;
                    } else {
                        match self.active_tab {
                            TabKind::Alerts => {
                                self.alerts_tab.handle_key(key, &mut self.config);
                            }
                            TabKind::Results => {
                                if let Some(action) = self.results_tab.handle_key(key, &mut self.results) {
                                    match action {
                                        crate::tui::tabs::results::ResultsAction::OpenUrl(url) => {
                                            let _ = open::that(&url);
                                        }
                                    }
                                }
                            }
                            TabKind::Settings => {
                                self.settings_tab.handle_key(key, &mut self.config);
                            }
                        }
                    }
                }
            }

            if last_results_refresh.elapsed() >= results_refresh_interval {
                if let Ok(new_results) = load_results(&self.results_path) {
                    self.results = new_results;
                }
                last_results_refresh = Instant::now();
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(frame.area());

        self.render_tabs(frame, chunks[0]);

        match self.active_tab {
            TabKind::Alerts => self.alerts_tab.render(frame, chunks[1], &self.theme, &self.config),
            TabKind::Results => self.results_tab.render(frame, chunks[1], &self.theme, &self.results),
            TabKind::Settings => self.settings_tab.render(frame, chunks[1], &self.theme, &self.config),
        }
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let tab_titles: Vec<Line> = TabKind::all()
            .iter()
            .map(|t| {
                let style = if *t == self.active_tab {
                    Style::default()
                        .fg(self.theme.active_tab)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(self.theme.inactive_tab)
                };
                Line::from(Span::styled(t.title(), style))
            })
            .collect();

        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(self.theme.border))
                    .title(Span::styled(
                        " snag ",
                        Style::default()
                            .fg(self.theme.accent)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .select(
                TabKind::all()
                    .iter()
                    .position(|t| *t == self.active_tab)
                    .unwrap_or(0),
            )
            .highlight_style(
                Style::default()
                    .fg(self.theme.active_tab)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(tabs, area);
    }
}
```

- [ ] **Step 5: Implement tui/mod.rs with terminal setup/teardown**

Replace `src/tui/mod.rs`:

```rust
pub mod app;
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

- [ ] **Step 6: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 7: Run the TUI manually to verify it starts and quits with 'q'**

Run: `cargo run`
Expected: TUI opens with tab bar showing "Alerts Results Settings". Press `q` to quit.

- [ ] **Step 8: Commit**

```bash
git add src/tui/
git commit -m "feat: add TUI shell with tab navigation and event loop"
```

---

### Task 9: Alerts Tab — List and Detail Rendering

**Files:**
- Modify: `src/tui/tabs/alerts.rs`

- [ ] **Step 1: Implement alerts tab with split pane, list, detail, and keybindings**

Replace `src/tui/tabs/alerts.rs`:

```rust
use crate::config::AppConfig;
use crate::tui::theme::Theme;
use crate::types::Alert;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

pub struct AlertsTab {
    pub selected: usize,
    pub list_state: ListState,
}

impl AlertsTab {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            selected: 0,
            list_state,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, config: &mut AppConfig) -> Option<AlertsAction> {
        let alert_count = config.alerts.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if alert_count > 0 && self.selected > 0 {
                    self.selected -= 1;
                    self.list_state.select(Some(self.selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if alert_count > 0 && self.selected < alert_count - 1 {
                    self.selected += 1;
                    self.list_state.select(Some(self.selected));
                }
            }
            KeyCode::Char(' ') => {
                if let Some(alert) = config.alerts.get_mut(self.selected) {
                    alert.enabled = !alert.enabled;
                    return Some(AlertsAction::ConfigChanged);
                }
            }
            KeyCode::Char('n') => {
                return Some(AlertsAction::CreateAlert);
            }
            KeyCode::Char('e') => {
                if self.selected < alert_count {
                    return Some(AlertsAction::EditAlert(self.selected));
                }
            }
            KeyCode::Char('d') => {
                if self.selected < alert_count {
                    return Some(AlertsAction::DeleteAlert(self.selected));
                }
            }
            _ => {}
        }

        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.render_list(frame, chunks[0], theme, config);
        self.render_detail(frame, chunks[1], theme, config);
    }

    fn render_list(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig) {
        let items: Vec<ListItem> = config
            .alerts
            .iter()
            .enumerate()
            .map(|(i, alert)| {
                let indicator = if alert.enabled { "●" } else { "○" };
                let color = if alert.enabled {
                    theme.enabled
                } else {
                    theme.disabled
                };

                let style = if i == self.selected {
                    Style::default().bg(theme.selected_bg).fg(theme.fg)
                } else {
                    Style::default().fg(theme.fg)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", indicator), Style::default().fg(color)),
                    Span::styled(&alert.name, style),
                ]))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(Span::styled(
                    " Alerts ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );

        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig) {
        let block = Block::default()
            .title(Span::styled(
                " Details ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let alert = match config.alerts.get(self.selected) {
            Some(a) => a,
            None => {
                let empty = Paragraph::new("No alerts configured. Press 'n' to create one.")
                    .style(Style::default().fg(theme.fg_dim));
                frame.render_widget(empty, inner);
                return;
            }
        };

        let mut lines = vec![];
        lines.push(Line::from(Span::styled(
            &alert.name,
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        let marketplaces: Vec<String> = alert.marketplaces.iter().map(|m| m.to_string()).collect();
        lines.push(detail_line("Marketplaces", &marketplaces.join(", "), theme));
        lines.push(detail_line("Keywords", &alert.keywords.join(", "), theme));

        if !alert.exclude_keywords.is_empty() {
            lines.push(detail_line(
                "Exclude",
                &alert.exclude_keywords.join(", "),
                theme,
            ));
        }

        if alert.price_min.is_some() || alert.price_max.is_some() {
            let price = format!(
                "${} — ${}",
                alert
                    .price_min
                    .map(|p| format!("{:.0}", p))
                    .unwrap_or_else(|| "any".into()),
                alert
                    .price_max
                    .map(|p| format!("{:.0}", p))
                    .unwrap_or_else(|| "any".into()),
            );
            lines.push(detail_line("Price", &price, theme));
        }

        if let Some(ref loc) = alert.location {
            let loc_str = if let Some(r) = alert.radius_miles {
                format!("{}, {}mi", loc, r)
            } else {
                loc.clone()
            };
            lines.push(detail_line("Location", &loc_str, theme));
        }

        if let Some(ref cond) = alert.condition {
            lines.push(detail_line("Condition", &cond.to_string(), theme));
        }

        if let Some(ref cat) = alert.category {
            lines.push(detail_line("Category", cat, theme));
        }

        let interval_secs = alert.check_interval.as_secs();
        let interval_str = if interval_secs >= 3600 {
            format!("{}h", interval_secs / 3600)
        } else if interval_secs >= 60 {
            format!("{}m", interval_secs / 60)
        } else {
            format!("{}s", interval_secs)
        };
        lines.push(detail_line("Interval", &interval_str, theme));

        let notifiers: Vec<String> = alert.notifiers.iter().map(|n| n.to_string()).collect();
        lines.push(detail_line("Notify", &notifiers.join(", "), theme));

        if let Some(max) = alert.max_results {
            lines.push(detail_line("Max results", &max.to_string(), theme));
        }

        let status = if alert.enabled { "Enabled" } else { "Disabled" };
        let status_color = if alert.enabled {
            theme.enabled
        } else {
            theme.disabled
        };
        lines.push(Line::from(vec![
            Span::styled("Status    ", Style::default().fg(theme.fg_dim)),
            Span::styled(status, Style::default().fg(status_color)),
        ]));

        let detail = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(detail, inner);
    }
}

fn detail_line<'a>(label: &'a str, value: &'a str, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{:<10}", label),
            Style::default().fg(theme.fg_dim),
        ),
        Span::styled(value, Style::default().fg(theme.fg)),
    ])
}

pub enum AlertsAction {
    ConfigChanged,
    CreateAlert,
    EditAlert(usize),
    DeleteAlert(usize),
}
```

- [ ] **Step 2: Update app.rs to handle AlertsAction**

In `src/tui/app.rs`, replace the `TabKind::Alerts` match arm in the key handling section:

```rust
TabKind::Alerts => {
    if let Some(action) = self.alerts_tab.handle_key(key, &mut self.config) {
        match action {
            crate::tui::tabs::alerts::AlertsAction::ConfigChanged => {
                let _ = save_config(&self.config, &self.config_path);
            }
            crate::tui::tabs::alerts::AlertsAction::CreateAlert => {
                // TODO: open alert form dialog
            }
            crate::tui::tabs::alerts::AlertsAction::EditAlert(_idx) => {
                // TODO: open alert form dialog with existing alert
            }
            crate::tui::tabs::alerts::AlertsAction::DeleteAlert(idx) => {
                if idx < self.config.alerts.len() {
                    self.config.alerts.remove(idx);
                    let _ = save_config(&self.config, &self.config_path);
                    if self.alerts_tab.selected >= self.config.alerts.len()
                        && self.alerts_tab.selected > 0
                    {
                        self.alerts_tab.selected -= 1;
                        self.alerts_tab
                            .list_state
                            .select(Some(self.alerts_tab.selected));
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 3: Verify it compiles and run manually**

Run: `cargo run`
Expected: Alerts tab shows list on left, detail on right. Arrow keys navigate. Press `q` to quit.

- [ ] **Step 4: Commit**

```bash
git add src/tui/tabs/alerts.rs src/tui/app.rs
git commit -m "feat: implement alerts tab with list, detail pane, and keybindings"
```

---

### Task 10: Results Tab — List and Detail Rendering

**Files:**
- Modify: `src/tui/tabs/results.rs`

- [ ] **Step 1: Implement results tab with split pane, unread indicators, and keybindings**

Replace `src/tui/tabs/results.rs`:

```rust
use crate::tui::theme::Theme;
use crate::types::AlertResult;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

pub struct ResultsTab {
    pub selected: usize,
    pub list_state: ListState,
}

struct FlatListing {
    pub alert_name: String,
    pub result_idx: usize,
    pub listing_idx: usize,
}

impl ResultsTab {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            selected: 0,
            list_state,
        }
    }

    fn flatten(results: &[AlertResult]) -> Vec<FlatListing> {
        let mut flat = vec![];
        for (ri, result) in results.iter().enumerate().rev() {
            for (li, _listing) in result.listings.iter().enumerate() {
                flat.push(FlatListing {
                    alert_name: result.alert_name.clone(),
                    result_idx: ri,
                    listing_idx: li,
                });
            }
        }
        flat
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        results: &mut Vec<AlertResult>,
    ) -> Option<ResultsAction> {
        let flat = Self::flatten(results);
        let count = flat.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if count > 0 && self.selected > 0 {
                    self.selected -= 1;
                    self.list_state.select(Some(self.selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if count > 0 && self.selected < count - 1 {
                    self.selected += 1;
                    self.list_state.select(Some(self.selected));
                }
            }
            KeyCode::Char('o') => {
                if let Some(entry) = flat.get(self.selected) {
                    let url = results[entry.result_idx].listings[entry.listing_idx]
                        .url
                        .clone();
                    results[entry.result_idx].seen = true;
                    return Some(ResultsAction::OpenUrl(url));
                }
            }
            KeyCode::Char('m') => {
                if let Some(entry) = flat.get(self.selected) {
                    results[entry.result_idx].seen = true;
                }
            }
            KeyCode::Char('c') => {
                results.clear();
                self.selected = 0;
                self.list_state.select(Some(0));
                return Some(ResultsAction::ResultsChanged);
            }
            _ => {}
        }

        None
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        results: &[AlertResult],
    ) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        let flat = Self::flatten(results);
        self.render_list(frame, chunks[0], theme, results, &flat);
        self.render_detail(frame, chunks[1], theme, results, &flat);
    }

    fn render_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        results: &[AlertResult],
        flat: &[FlatListing],
    ) {
        let items: Vec<ListItem> = flat
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let listing = &results[entry.result_idx].listings[entry.listing_idx];
                let seen = results[entry.result_idx].seen;

                let indicator = if seen { "  " } else { "● " };
                let indicator_color = if seen { theme.fg_dim } else { theme.unread };

                let title = if listing.title.len() > 25 {
                    format!("{}…", &listing.title[..24])
                } else {
                    listing.title.clone()
                };

                let style = if i == self.selected {
                    Style::default().bg(theme.selected_bg).fg(theme.fg)
                } else {
                    Style::default().fg(theme.fg)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(indicator, Style::default().fg(indicator_color)),
                    Span::styled(title, style),
                ]))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(Span::styled(
                    " Results ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );

        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_detail(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        results: &[AlertResult],
        flat: &[FlatListing],
    ) {
        let block = Block::default()
            .title(Span::styled(
                " Details ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let entry = match flat.get(self.selected) {
            Some(e) => e,
            None => {
                let empty = Paragraph::new("No results yet.")
                    .style(Style::default().fg(theme.fg_dim));
                frame.render_widget(empty, inner);
                return;
            }
        };

        let listing = &results[entry.result_idx].listings[entry.listing_idx];

        let mut lines = vec![];
        lines.push(Line::from(Span::styled(
            &listing.title,
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        if let Some(price) = listing.price {
            lines.push(detail_line(
                "Price",
                &format!("{}{:.2}", listing.currency, price),
                theme,
            ));
        }

        lines.push(detail_line("Marketplace", &listing.marketplace.to_string(), theme));

        if let Some(ref loc) = listing.location {
            lines.push(detail_line("Location", loc, theme));
        }

        if let Some(ref cond) = listing.condition {
            lines.push(detail_line("Condition", &cond.to_string(), theme));
        }

        if let Some(ref posted) = listing.posted_at {
            lines.push(detail_line("Posted", &posted.format("%Y-%m-%d %H:%M").to_string(), theme));
        }

        lines.push(detail_line("Found", &listing.found_at.format("%Y-%m-%d %H:%M").to_string(), theme));
        lines.push(Line::from(""));
        lines.push(detail_line("Alert", &entry.alert_name, theme));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "[o] open in browser",
            Style::default().fg(theme.accent),
        )));

        let detail = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(detail, inner);
    }
}

fn detail_line<'a>(label: &'a str, value: &'a str, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{:<12}", label),
            Style::default().fg(theme.fg_dim),
        ),
        Span::styled(value, Style::default().fg(theme.fg)),
    ])
}

pub enum ResultsAction {
    OpenUrl(String),
    ResultsChanged,
}
```

- [ ] **Step 2: Update app.rs to handle ResultsAction::ResultsChanged**

In `src/tui/app.rs`, update the `TabKind::Results` match arm:

```rust
TabKind::Results => {
    if let Some(action) = self.results_tab.handle_key(key, &mut self.results) {
        match action {
            crate::tui::tabs::results::ResultsAction::OpenUrl(url) => {
                let _ = open::that(&url);
            }
            crate::tui::tabs::results::ResultsAction::ResultsChanged => {
                let _ = crate::daemon::results::save_results(
                    &self.results,
                    &self.results_path,
                );
            }
        }
    }
}
```

- [ ] **Step 3: Verify it compiles and run manually**

Run: `cargo run`
Expected: Results tab shows list and detail pane. Tab/2 to switch to it.

- [ ] **Step 4: Commit**

```bash
git add src/tui/tabs/results.rs src/tui/app.rs
git commit -m "feat: implement results tab with listing list, detail, and open-in-browser"
```

---

### Task 11: Settings Tab

**Files:**
- Modify: `src/tui/tabs/settings.rs`

- [ ] **Step 1: Implement settings tab with daemon status and global settings**

Replace `src/tui/tabs/settings.rs`:

```rust
use crate::config::AppConfig;
use crate::tui::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::path::PathBuf;

pub struct SettingsTab {
    pub selected: usize,
    pub field_count: usize,
}

impl SettingsTab {
    pub fn new() -> Self {
        Self {
            selected: 0,
            field_count: 3,
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        _config: &mut AppConfig,
    ) -> Option<SettingsAction> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected < self.field_count - 1 {
                    self.selected += 1;
                }
            }
            KeyCode::Char('r') => return Some(SettingsAction::RestartDaemon),
            KeyCode::Char('s') => return Some(SettingsAction::StopDaemon),
            _ => {}
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig) {
        let block = Block::default()
            .title(Span::styled(
                " Settings ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Length(10), Constraint::Min(0)])
            .split(inner);

        self.render_daemon_section(frame, chunks[0], theme);
        self.render_defaults_section(frame, chunks[1], theme, config);
    }

    fn render_daemon_section(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let pid_path = crate::config::data_dir().join("daemon.pid");
        let (status, pid) = read_daemon_status(&pid_path);

        let status_color = if status == "Running" {
            theme.enabled
        } else {
            theme.disabled
        };

        let mut lines = vec![];
        lines.push(Line::from(Span::styled(
            "Daemon",
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Status    ", Style::default().fg(theme.fg_dim)),
            Span::styled(&status, Style::default().fg(status_color)),
            Span::styled(
                pid.map(|p| format!(" (PID {})", p)).unwrap_or_default(),
                Style::default().fg(theme.fg_dim),
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  [r] restart  [s] stop",
            Style::default().fg(theme.accent),
        )));

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

    fn render_defaults_section(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        config: &AppConfig,
    ) {
        let interval_secs = config.settings.default_check_interval.as_secs();
        let interval_str = if interval_secs >= 3600 {
            format!("{}h", interval_secs / 3600)
        } else if interval_secs >= 60 {
            format!("{}m", interval_secs / 60)
        } else {
            format!("{}s", interval_secs)
        };

        let mut lines = vec![];
        lines.push(Line::from(Span::styled(
            "Defaults",
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Check interval  ", Style::default().fg(theme.fg_dim)),
            Span::styled(interval_str, Style::default().fg(theme.fg)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("  Max results     ", Style::default().fg(theme.fg_dim)),
            Span::styled(
                config
                    .settings
                    .default_max_results
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "unlimited".into()),
                Style::default().fg(theme.fg),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("  Notification    ", Style::default().fg(theme.fg_dim)),
            Span::styled(
                config.settings.default_notifier.to_string(),
                Style::default().fg(theme.fg),
            ),
        ]));

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }
}

fn read_daemon_status(pid_path: &PathBuf) -> (String, Option<u32>) {
    let pid_str = match std::fs::read_to_string(pid_path) {
        Ok(s) => s,
        Err(_) => return ("Stopped".into(), None),
    };

    let pid: u32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => return ("Stopped".into(), None),
    };

    let alive = std::path::Path::new(&format!("/proc/{}", pid)).exists();
    if alive {
        ("Running".into(), Some(pid))
    } else {
        ("Stopped (stale PID)".into(), Some(pid))
    }
}

pub enum SettingsAction {
    StartDaemon,
    StopDaemon,
    RestartDaemon,
}
```

- [ ] **Step 2: Update app.rs to handle SettingsAction**

In `src/tui/app.rs`, replace the `TabKind::Settings` match arm:

```rust
TabKind::Settings => {
    if let Some(action) = self.settings_tab.handle_key(key, &mut self.config) {
        match action {
            crate::tui::tabs::settings::SettingsAction::StopDaemon => {
                let pid_path = config::data_dir().join("daemon.pid");
                if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
                    if let Ok(pid) = pid_str.trim().parse::<i32>() {
                        unsafe {
                            libc::kill(pid, libc::SIGTERM);
                        }
                    }
                }
            }
            crate::tui::tabs::settings::SettingsAction::RestartDaemon => {
                let pid_path = config::data_dir().join("daemon.pid");
                if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
                    if let Ok(pid) = pid_str.trim().parse::<i32>() {
                        unsafe {
                            libc::kill(pid, libc::SIGTERM);
                        }
                    }
                }
                let exe = std::env::current_exe().unwrap();
                std::process::Command::new(exe)
                    .arg("daemon")
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .ok();
            }
            crate::tui::tabs::settings::SettingsAction::StartDaemon => {
                let exe = std::env::current_exe().unwrap();
                std::process::Command::new(exe)
                    .arg("daemon")
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .ok();
            }
        }
    }
}
```

- [ ] **Step 3: Add libc dependency to Cargo.toml**

Add to `[dependencies]` in `Cargo.toml`:

```toml
libc = "0.2"
```

- [ ] **Step 4: Verify it compiles and run manually**

Run: `cargo run`
Expected: Settings tab shows daemon status and default values. Tab/3 to switch to it.

- [ ] **Step 5: Commit**

```bash
git add src/tui/tabs/settings.rs src/tui/app.rs Cargo.toml
git commit -m "feat: implement settings tab with daemon control and global defaults"
```

---

### Task 12: Confirm Dialog

**Files:**
- Create: `src/tui/dialogs/mod.rs`
- Create: `src/tui/dialogs/confirm.rs`

- [ ] **Step 1: Implement DialogResult and confirm dialog**

Create `src/tui/dialogs/mod.rs`:

```rust
pub mod alert_form;
pub mod confirm;

pub enum DialogResult<T> {
    Continue,
    Cancel,
    Submit(T),
}
```

Create `src/tui/dialogs/confirm.rs`:

```rust
use super::DialogResult;
use crate::tui::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub selected: bool,
}

impl ConfirmDialog {
    pub fn new(title: String, message: String) -> Self {
        Self {
            title,
            message,
            selected: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DialogResult<bool> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') => DialogResult::Cancel,
            KeyCode::Enter => {
                if self.selected {
                    DialogResult::Submit(true)
                } else {
                    DialogResult::Cancel
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.selected = true;
                DialogResult::Continue
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.selected = false;
                DialogResult::Continue
            }
            KeyCode::Char('y') => DialogResult::Submit(true),
            _ => DialogResult::Continue,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog_width = 50u16.min(area.width.saturating_sub(4));
        let dialog_height = 7u16.min(area.height.saturating_sub(4));

        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(Span::styled(
                format!(" {} ", self.title),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        let inner_chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(inner);

        let message = Paragraph::new(self.message.as_str())
            .style(Style::default().fg(theme.fg))
            .wrap(Wrap { trim: false });
        frame.render_widget(message, inner_chunks[0]);

        let yes_style = if self.selected {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg_dim)
        };
        let no_style = if !self.selected {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg_dim)
        };

        let buttons = Line::from(vec![
            Span::raw("  "),
            Span::styled(" Yes ", yes_style),
            Span::raw("   "),
            Span::styled(" No ", no_style),
        ]);

        frame.render_widget(Paragraph::new(buttons), inner_chunks[1]);
    }
}
```

- [ ] **Step 2: Create stub alert_form.rs**

Create `src/tui/dialogs/alert_form.rs`:

```rust
pub struct AlertFormDialog;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles

- [ ] **Step 4: Commit**

```bash
git add src/tui/dialogs/
git commit -m "feat: add confirm dialog with yes/no selection"
```

---

### Task 13: Alert Form Dialog

**Files:**
- Modify: `src/tui/dialogs/alert_form.rs`

- [ ] **Step 1: Implement the alert form dialog**

Replace `src/tui/dialogs/alert_form.rs`:

```rust
use super::DialogResult;
use crate::tui::theme::Theme;
use crate::types::*;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use std::time::Duration;
use uuid::Uuid;

pub struct AlertFormDialog {
    pub fields: Vec<FormField>,
    pub selected_field: usize,
    pub editing: bool,
    pub existing_id: Option<Uuid>,
}

pub struct FormField {
    pub label: String,
    pub value: String,
    pub cursor: usize,
}

impl FormField {
    fn new(label: &str, value: &str) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            cursor: value.len(),
        }
    }

    fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    fn delete_char(&mut self) {
        if self.cursor > 0 {
            let prev = self.value[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.value.remove(prev);
            self.cursor = prev;
        }
    }
}

impl AlertFormDialog {
    pub fn new() -> Self {
        Self {
            fields: vec![
                FormField::new("Name", ""),
                FormField::new("Marketplaces", "ebay"),
                FormField::new("Keywords", ""),
                FormField::new("Exclude keywords", ""),
                FormField::new("Price min", ""),
                FormField::new("Price max", ""),
                FormField::new("Location", ""),
                FormField::new("Radius (miles)", ""),
                FormField::new("Condition", ""),
                FormField::new("Category", ""),
                FormField::new("Interval (seconds)", "300"),
                FormField::new("Max results", "20"),
            ],
            selected_field: 0,
            editing: false,
            existing_id: None,
        }
    }

    pub fn from_alert(alert: &Alert) -> Self {
        let marketplaces: Vec<String> = alert.marketplaces.iter().map(|m| match m {
            MarketplaceKind::Ebay => "ebay".into(),
            MarketplaceKind::FacebookMarketplace => "facebook".into(),
        }).collect();

        Self {
            fields: vec![
                FormField::new("Name", &alert.name),
                FormField::new("Marketplaces", &marketplaces.join(", ")),
                FormField::new("Keywords", &alert.keywords.join(", ")),
                FormField::new("Exclude keywords", &alert.exclude_keywords.join(", ")),
                FormField::new("Price min", &alert.price_min.map(|p| p.to_string()).unwrap_or_default()),
                FormField::new("Price max", &alert.price_max.map(|p| p.to_string()).unwrap_or_default()),
                FormField::new("Location", alert.location.as_deref().unwrap_or("")),
                FormField::new("Radius (miles)", &alert.radius_miles.map(|r| r.to_string()).unwrap_or_default()),
                FormField::new("Condition", &alert.condition.map(|c| match c {
                    Condition::New => "new",
                    Condition::LikeNew => "like new",
                    Condition::Used => "used",
                    Condition::ForParts => "for parts",
                }).unwrap_or("")),
                FormField::new("Category", alert.category.as_deref().unwrap_or("")),
                FormField::new("Interval (seconds)", &alert.check_interval.as_secs().to_string()),
                FormField::new("Max results", &alert.max_results.map(|m| m.to_string()).unwrap_or_default()),
            ],
            selected_field: 0,
            editing: false,
            existing_id: Some(alert.id),
        }
    }

    pub fn to_alert(&self) -> Option<Alert> {
        let name = self.fields[0].value.trim().to_string();
        if name.is_empty() {
            return None;
        }

        let marketplaces: Vec<MarketplaceKind> = self.fields[1]
            .value
            .split(',')
            .filter_map(|s| match s.trim().to_lowercase().as_str() {
                "ebay" => Some(MarketplaceKind::Ebay),
                "facebook" | "fb" => Some(MarketplaceKind::FacebookMarketplace),
                _ => None,
            })
            .collect();

        if marketplaces.is_empty() {
            return None;
        }

        let keywords: Vec<String> = self.fields[2]
            .value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if keywords.is_empty() {
            return None;
        }

        let exclude_keywords: Vec<String> = self.fields[3]
            .value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let price_min = self.fields[4].value.trim().parse::<f64>().ok();
        let price_max = self.fields[5].value.trim().parse::<f64>().ok();

        let location = {
            let v = self.fields[6].value.trim().to_string();
            if v.is_empty() { None } else { Some(v) }
        };

        let radius_miles = self.fields[7].value.trim().parse::<u32>().ok();

        let condition = match self.fields[8].value.trim().to_lowercase().as_str() {
            "new" => Some(Condition::New),
            "like new" => Some(Condition::LikeNew),
            "used" => Some(Condition::Used),
            "for parts" => Some(Condition::ForParts),
            _ => None,
        };

        let category = {
            let v = self.fields[9].value.trim().to_string();
            if v.is_empty() { None } else { Some(v) }
        };

        let interval_secs = self.fields[10]
            .value
            .trim()
            .parse::<u64>()
            .unwrap_or(300);

        let max_results = self.fields[11].value.trim().parse::<u32>().ok();

        Some(Alert {
            id: self.existing_id.unwrap_or_else(Uuid::new_v4),
            name,
            marketplaces,
            keywords,
            exclude_keywords,
            price_min,
            price_max,
            location,
            radius_miles,
            condition,
            category,
            check_interval: Duration::from_secs(interval_secs),
            notifiers: vec![NotifierKind::Terminal],
            max_results,
            enabled: true,
        })
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DialogResult<Alert> {
        if self.editing {
            match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                }
                KeyCode::Enter => {
                    self.editing = false;
                }
                KeyCode::Backspace => {
                    self.fields[self.selected_field].delete_char();
                }
                KeyCode::Char(c) => {
                    self.fields[self.selected_field].insert_char(c);
                }
                _ => {}
            }
            return DialogResult::Continue;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => DialogResult::Cancel,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_field > 0 {
                    self.selected_field -= 1;
                }
                DialogResult::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_field < self.fields.len() - 1 {
                    self.selected_field += 1;
                }
                DialogResult::Continue
            }
            KeyCode::Enter => {
                self.editing = true;
                let field = &mut self.fields[self.selected_field];
                field.cursor = field.value.len();
                DialogResult::Continue
            }
            KeyCode::Char('s') => match self.to_alert() {
                Some(alert) => DialogResult::Submit(alert),
                None => DialogResult::Continue,
            },
            _ => DialogResult::Continue,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog_width = 60u16.min(area.width.saturating_sub(4));
        let dialog_height = (self.fields.len() as u16 + 6).min(area.height.saturating_sub(4));

        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        frame.render_widget(Clear, dialog_area);

        let title = if self.existing_id.is_some() {
            " Edit Alert "
        } else {
            " New Alert "
        };

        let block = Block::default()
            .title(Span::styled(
                title,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        let mut constraints: Vec<Constraint> = self
            .fields
            .iter()
            .map(|_| Constraint::Length(1))
            .collect();
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Min(0));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, field) in self.fields.iter().enumerate() {
            let is_selected = i == self.selected_field;
            let is_editing = is_selected && self.editing;

            let label_style = if is_selected {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.fg_dim)
            };

            let value_style = if is_editing {
                Style::default()
                    .fg(theme.fg)
                    .add_modifier(Modifier::UNDERLINED)
            } else if is_selected {
                Style::default().fg(theme.fg)
            } else {
                Style::default().fg(theme.fg_dim)
            };

            let cursor = if is_selected { "▸ " } else { "  " };
            let display_value = if field.value.is_empty() && !is_editing {
                "—".to_string()
            } else {
                field.value.clone()
            };

            let line = Line::from(vec![
                Span::styled(cursor, Style::default().fg(theme.accent)),
                Span::styled(format!("{:<20}", field.label), label_style),
                Span::styled(display_value, value_style),
            ]);

            frame.render_widget(Paragraph::new(line), chunks[i]);
        }

        let help_line = Line::from(vec![
            Span::styled(
                " [Enter] edit field  [s] save  [Esc] cancel",
                Style::default().fg(theme.fg_dim),
            ),
        ]);
        let help_idx = self.fields.len();
        if help_idx < chunks.len() {
            frame.render_widget(Paragraph::new(help_line), chunks[help_idx]);
        }
    }
}
```

- [ ] **Step 2: Wire alert form into app.rs**

Add a dialog field to the `App` struct in `src/tui/app.rs`:

Add to the struct fields:
```rust
pub active_dialog: Option<ActiveDialog>,
```

Add the enum after the App struct:
```rust
pub enum ActiveDialog {
    AlertForm(crate::tui::dialogs::alert_form::AlertFormDialog),
    Confirm(crate::tui::dialogs::confirm::ConfirmDialog, ConfirmAction),
}

pub enum ConfirmAction {
    DeleteAlert(usize),
    ClearResults,
}
```

Initialize `active_dialog: None` in `App::new()`.

Update the key handling in `run()` to check dialogs first:

```rust
// At the top of the key event handling, before tab-specific handling:
if let Some(ref mut dialog) = self.active_dialog {
    match dialog {
        ActiveDialog::AlertForm(form) => {
            match form.handle_key(key) {
                crate::tui::dialogs::DialogResult::Continue => {}
                crate::tui::dialogs::DialogResult::Cancel => {
                    self.active_dialog = None;
                }
                crate::tui::dialogs::DialogResult::Submit(alert) => {
                    if let Some(existing_idx) = self.config.alerts.iter().position(|a| a.id == alert.id) {
                        self.config.alerts[existing_idx] = alert;
                    } else {
                        self.config.alerts.push(alert);
                    }
                    let _ = save_config(&self.config, &self.config_path);
                    self.active_dialog = None;
                }
            }
        }
        ActiveDialog::Confirm(confirm, _) => {
            match confirm.handle_key(key) {
                crate::tui::dialogs::DialogResult::Continue => {}
                crate::tui::dialogs::DialogResult::Cancel => {
                    self.active_dialog = None;
                }
                crate::tui::dialogs::DialogResult::Submit(_) => {
                    let action = match std::mem::replace(&mut self.active_dialog, None) {
                        Some(ActiveDialog::Confirm(_, action)) => Some(action),
                        _ => None,
                    };
                    if let Some(action) = action {
                        match action {
                            ConfirmAction::DeleteAlert(idx) => {
                                if idx < self.config.alerts.len() {
                                    self.config.alerts.remove(idx);
                                    let _ = save_config(&self.config, &self.config_path);
                                    if self.alerts_tab.selected >= self.config.alerts.len() && self.alerts_tab.selected > 0 {
                                        self.alerts_tab.selected -= 1;
                                        self.alerts_tab.list_state.select(Some(self.alerts_tab.selected));
                                    }
                                }
                            }
                            ConfirmAction::ClearResults => {
                                self.results.clear();
                                self.results_tab.selected = 0;
                                self.results_tab.list_state.select(Some(0));
                                let _ = crate::daemon::results::save_results(&self.results, &self.results_path);
                            }
                        }
                    }
                }
            }
        }
    }
    continue; // skip normal tab handling when dialog is active
}
```

Update the `AlertsAction` handlers:
```rust
crate::tui::tabs::alerts::AlertsAction::CreateAlert => {
    self.active_dialog = Some(ActiveDialog::AlertForm(
        crate::tui::dialogs::alert_form::AlertFormDialog::new(),
    ));
}
crate::tui::tabs::alerts::AlertsAction::EditAlert(idx) => {
    if let Some(alert) = self.config.alerts.get(idx) {
        self.active_dialog = Some(ActiveDialog::AlertForm(
            crate::tui::dialogs::alert_form::AlertFormDialog::from_alert(alert),
        ));
    }
}
crate::tui::tabs::alerts::AlertsAction::DeleteAlert(idx) => {
    if idx < self.config.alerts.len() {
        let name = self.config.alerts[idx].name.clone();
        self.active_dialog = Some(ActiveDialog::Confirm(
            crate::tui::dialogs::confirm::ConfirmDialog::new(
                "Delete Alert".into(),
                format!("Delete '{}'?", name),
            ),
            ConfirmAction::DeleteAlert(idx),
        ));
    }
}
```

Update the render method to draw dialogs on top:
```rust
fn render(&self, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(frame.area());

    self.render_tabs(frame, chunks[0]);

    match self.active_tab {
        TabKind::Alerts => self.alerts_tab.render(frame, chunks[1], &self.theme, &self.config),
        TabKind::Results => self.results_tab.render(frame, chunks[1], &self.theme, &self.results),
        TabKind::Settings => self.settings_tab.render(frame, chunks[1], &self.theme, &self.config),
    }

    if let Some(ref dialog) = self.active_dialog {
        match dialog {
            ActiveDialog::AlertForm(form) => form.render(frame, frame.area(), &self.theme),
            ActiveDialog::Confirm(confirm, _) => confirm.render(frame, frame.area(), &self.theme),
        }
    }
}
```

- [ ] **Step 3: Add the dialogs module to tui/mod.rs**

Add `pub mod dialogs;` to `src/tui/mod.rs`.

- [ ] **Step 4: Verify it compiles and run manually**

Run: `cargo run`
Expected: Press `n` on Alerts tab to open the form. Navigate fields with j/k, Enter to edit, `s` to save, Esc to cancel.

- [ ] **Step 5: Commit**

```bash
git add src/tui/
git commit -m "feat: add alert form dialog and wire dialogs into app event loop"
```

---

### Task 14: Status Bar with Keybinding Hints

**Files:**
- Modify: `src/tui/app.rs`

- [ ] **Step 1: Add a status bar at the bottom of the TUI**

In `src/tui/app.rs`, update the `render` method to add a bottom bar:

Change the layout constraints:
```rust
fn render(&self, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    self.render_tabs(frame, chunks[0]);

    match self.active_tab {
        TabKind::Alerts => self.alerts_tab.render(frame, chunks[1], &self.theme, &self.config),
        TabKind::Results => self.results_tab.render(frame, chunks[1], &self.theme, &self.results),
        TabKind::Settings => self.settings_tab.render(frame, chunks[1], &self.theme, &self.config),
    }

    self.render_status_bar(frame, chunks[2]);

    if let Some(ref dialog) = self.active_dialog {
        match dialog {
            ActiveDialog::AlertForm(form) => form.render(frame, frame.area(), &self.theme),
            ActiveDialog::Confirm(confirm, _) => confirm.render(frame, frame.area(), &self.theme),
        }
    }
}
```

Add the `render_status_bar` method:
```rust
fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
    let hints = match self.active_tab {
        TabKind::Alerts => "[n]ew [e]dit [d]elete [space]toggle [q]uit",
        TabKind::Results => "[o]pen [m]ark read [c]lear [q]uit",
        TabKind::Settings => "[r]estart [s]top daemon [q]uit",
    };

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(
            " Tab/1-3 ",
            Style::default()
                .fg(self.theme.status_bar_fg)
                .bg(self.theme.accent),
        ),
        Span::styled(
            format!(" {} ", hints),
            Style::default()
                .fg(self.theme.status_bar_fg)
                .bg(self.theme.status_bar_bg),
        ),
    ]));

    frame.render_widget(bar, area);
}
```

- [ ] **Step 2: Verify it compiles and run manually**

Run: `cargo run`
Expected: Bottom bar shows context-sensitive keybinding hints for each tab.

- [ ] **Step 3: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat: add status bar with context-sensitive keybinding hints"
```

---

### Task 15: Daemon Auto-Start from TUI

**Files:**
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Add auto-start logic to TUI startup**

In `src/tui/mod.rs`, add daemon auto-start before launching the TUI:

```rust
pub mod app;
pub mod dialogs;
pub mod tabs;
pub mod theme;

use crate::config;
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

pub async fn run() -> Result<()> {
    auto_start_daemon();

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

fn auto_start_daemon() {
    let pid_path = config::data_dir().join("daemon.pid");

    let daemon_running = std::fs::read_to_string(&pid_path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .map(|pid| std::path::Path::new(&format!("/proc/{}", pid)).exists())
        .unwrap_or(false);

    if daemon_running {
        return;
    }

    let _ = std::fs::remove_file(&pid_path);

    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe)
            .arg("daemon")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add src/tui/mod.rs
git commit -m "feat: auto-start daemon when launching TUI if not already running"
```

---

### Task 16: Integration Smoke Test

**Files:**
- No new files — manual verification

- [ ] **Step 1: Build the release binary**

Run: `cargo build --release`
Expected: builds successfully

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings

- [ ] **Step 4: Fix any clippy warnings**

Address any issues found by clippy.

- [ ] **Step 5: Run the TUI end-to-end**

Run: `cargo run`
Expected:
1. TUI opens with Alerts tab
2. Tab through all three tabs
3. Press `n` to create an alert, fill in fields, press `s` to save
4. Alert appears in the list
5. Press `q` to quit

- [ ] **Step 6: Verify daemon starts and runs**

Run: `cargo run -- daemon &` then `cargo run -- check`
Expected: daemon starts, check runs without error

- [ ] **Step 7: Final commit**

```bash
git add -A
git commit -m "chore: fix clippy warnings and verify integration"
```
