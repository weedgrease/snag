# Editable Settings + Auto-Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make settings editable inline and add auto-update from GitHub releases.

**Architecture:** Adds `check_for_updates` to config, makes the Settings tab an inline editor, creates a new `update` module for GitHub release checking and binary self-replacement, adds an `Update` CLI subcommand, and wires a background update check into the TUI event loop.

**Tech Stack:** Rust, reqwest (existing), semver (new), GitHub Releases API

---

## File Structure

```
src/
├── config.rs              — add check_for_updates field + default_true helper
├── update.rs              — NEW: check_for_update(), perform_update(), GitHub API types
├── lib.rs                 — add pub mod update
├── main.rs                — add Update subcommand
├── tui/
│   ├── app.rs             — add update_info field, background check, poll, ConfigChanged handling
│   └── tabs/
│       └── settings.rs    — inline editing, update banner
├── Cargo.toml             — add semver dependency
tests/
├── config_test.rs         — update for new field
```

---

### Task 1: Add check_for_updates to Config

**Files:**
- Modify: `src/config.rs`
- Modify: `tests/config_test.rs`

- [ ] **Step 1: Update GlobalSettings in config.rs**

In `src/config.rs`, add the `default_true` function and the new field:

Add this function before the `AppConfig` struct:

```rust
fn default_true() -> bool {
    true
}
```

Add this field to `GlobalSettings` after `default_notifier`:

```rust
#[serde(default = "default_true")]
pub check_for_updates: bool,
```

Update the `Default` impl for `AppConfig` to include `check_for_updates: true` in `GlobalSettings`.

- [ ] **Step 2: Update config tests**

In `tests/config_test.rs`, update the `save_and_load_config_round_trips` test to include the new field when constructing `GlobalSettings`:

```rust
let config = AppConfig {
    settings: GlobalSettings {
        default_check_interval: Duration::from_secs(300),
        default_max_results: Some(20),
        default_notifier: NotifierKind::Terminal,
        check_for_updates: true,
    },
    alerts: vec![
        // ... existing alert unchanged
    ],
};
```

Add a new test for backward compatibility:

```rust
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
```

- [ ] **Step 3: Run tests**

Run: `cargo test --test config_test`
Expected: all tests PASS

- [ ] **Step 4: Commit**

```bash
git add src/config.rs tests/config_test.rs
git commit -m "feat: add check_for_updates config field with backward-compatible default"
```

---

### Task 2: Editable Settings Tab

**Files:**
- Modify: `src/tui/tabs/settings.rs`
- Modify: `src/tui/app.rs`

- [ ] **Step 1: Rewrite settings.rs with inline editing**

Replace `src/tui/tabs/settings.rs` entirely:

```rust
use crate::config::AppConfig;
use crate::tui::theme::Theme;
use crate::types::NotifierKind;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::path::PathBuf;
use std::time::Duration;

const FIELD_CHECK_INTERVAL: usize = 0;
const FIELD_MAX_RESULTS: usize = 1;
const FIELD_NOTIFICATION: usize = 2;
const FIELD_CHECK_UPDATES: usize = 3;
const FIELD_COUNT: usize = 4;

pub struct SettingsTab {
    pub selected: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub update_banner: Option<String>,
}

impl Default for SettingsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsTab {
    pub fn new() -> Self {
        Self {
            selected: 0,
            editing: false,
            edit_buffer: String::new(),
            update_banner: None,
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        config: &mut AppConfig,
    ) -> Option<SettingsAction> {
        if self.editing {
            match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                }
                KeyCode::Enter => {
                    self.apply_edit(config);
                    self.editing = false;
                    return Some(SettingsAction::ConfigChanged);
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                }
                _ => {}
            }
            return None;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected < FIELD_COUNT - 1 {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => {
                match self.selected {
                    FIELD_CHECK_UPDATES => {
                        config.settings.check_for_updates = !config.settings.check_for_updates;
                        return Some(SettingsAction::ConfigChanged);
                    }
                    FIELD_NOTIFICATION => {
                        config.settings.default_notifier = match config.settings.default_notifier {
                            NotifierKind::Terminal => NotifierKind::Terminal,
                        };
                        return Some(SettingsAction::ConfigChanged);
                    }
                    _ => {
                        self.editing = true;
                        self.edit_buffer = self.current_field_value(config);
                    }
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selected == FIELD_NOTIFICATION {
                    config.settings.default_notifier = match config.settings.default_notifier {
                        NotifierKind::Terminal => NotifierKind::Terminal,
                    };
                    return Some(SettingsAction::ConfigChanged);
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.selected == FIELD_NOTIFICATION {
                    config.settings.default_notifier = match config.settings.default_notifier {
                        NotifierKind::Terminal => NotifierKind::Terminal,
                    };
                    return Some(SettingsAction::ConfigChanged);
                }
            }
            KeyCode::Char('r') => return Some(SettingsAction::RestartDaemon),
            KeyCode::Char('s') => return Some(SettingsAction::StopDaemon),
            _ => {}
        }
        None
    }

    fn current_field_value(&self, config: &AppConfig) -> String {
        match self.selected {
            FIELD_CHECK_INTERVAL => config.settings.default_check_interval.as_secs().to_string(),
            FIELD_MAX_RESULTS => config
                .settings
                .default_max_results
                .map(|m| m.to_string())
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn apply_edit(&self, config: &mut AppConfig) {
        match self.selected {
            FIELD_CHECK_INTERVAL => {
                if let Ok(secs) = self.edit_buffer.trim().parse::<u64>() {
                    if secs > 0 {
                        config.settings.default_check_interval = Duration::from_secs(secs);
                    }
                }
            }
            FIELD_MAX_RESULTS => {
                let trimmed = self.edit_buffer.trim();
                if trimmed.is_empty() {
                    config.settings.default_max_results = None;
                } else if let Ok(max) = trimmed.parse::<u32>() {
                    config.settings.default_max_results = Some(max);
                }
            }
            _ => {}
        }
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

        let has_banner = self.update_banner.is_some();
        let banner_height = if has_banner { 3 } else { 0 };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(4 + FIELD_COUNT as u16 + 2),
                Constraint::Length(banner_height),
                Constraint::Min(0),
            ])
            .split(inner);

        self.render_daemon_section(frame, chunks[0], theme);
        self.render_defaults_section(frame, chunks[1], theme, config);
        if let Some(ref banner) = self.update_banner {
            self.render_update_banner(frame, chunks[2], theme, banner);
        }
    }

    fn render_daemon_section(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let pid_path = crate::config::data_dir().join("daemon.pid");
        let (status, pid) = read_daemon_status(&pid_path);

        let status_color = if status == "Running" {
            theme.enabled
        } else {
            theme.disabled
        };

        let lines = vec![
            Line::from(Span::styled(
                "Daemon",
                Style::default()
                    .fg(theme.fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Status    ", Style::default().fg(theme.fg_dim)),
                Span::styled(&status, Style::default().fg(status_color)),
                Span::styled(
                    pid.map(|p| format!(" (PID {})", p)).unwrap_or_default(),
                    Style::default().fg(theme.fg_dim),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  [r] restart  [s] stop",
                Style::default().fg(theme.accent),
            )),
        ];

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
        let interval_val = config.settings.default_check_interval.as_secs().to_string();
        let max_val = config
            .settings
            .default_max_results
            .map(|m| m.to_string())
            .unwrap_or_else(|| "unlimited".into());
        let notifier_val = config.settings.default_notifier.to_string();
        let updates_val = if config.settings.check_for_updates {
            "Enabled"
        } else {
            "Disabled"
        };

        let fields = [
            ("Check interval (s)", interval_val),
            ("Max results", max_val),
            ("Notification", notifier_val),
            ("Check for updates", updates_val.to_string()),
        ];

        let mut lines = vec![
            Line::from(Span::styled(
                "Defaults",
                Style::default()
                    .fg(theme.fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        for (i, (label, value)) in fields.iter().enumerate() {
            let is_selected = i == self.selected;
            let is_editing = is_selected && self.editing;

            let cursor = if is_selected { "▸ " } else { "  " };

            let label_style = if is_selected {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.fg_dim)
            };

            let display_value = if is_editing {
                self.edit_buffer.clone()
            } else {
                value.clone()
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

            lines.push(Line::from(vec![
                Span::styled(cursor, Style::default().fg(theme.accent)),
                Span::styled(format!("{:<20}", label), label_style),
                Span::styled(display_value, value_style),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  [Enter] edit/toggle  [↑↓] navigate",
            Style::default().fg(theme.fg_dim),
        )));

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

    fn render_update_banner(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        banner: &str,
    ) {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {}", banner),
                Style::default().fg(theme.unread),
            )),
        ];

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
    ConfigChanged,
}
```

- [ ] **Step 2: Update app.rs to handle ConfigChanged from settings**

In `src/tui/app.rs`, update the `TabKind::Settings` match arm in the key handler:

Replace:
```rust
TabKind::Settings => {
    self.settings_tab.handle_key(key, &mut self.config);
}
```

With:
```rust
TabKind::Settings => {
    if let Some(action) = self.settings_tab.handle_key(key, &mut self.config) {
        match action {
            crate::tui::tabs::settings::SettingsAction::ConfigChanged => {
                let _ = save_config(&self.config, &self.config_path);
            }
            crate::tui::tabs::settings::SettingsAction::StopDaemon => {
                let pid_path = config::data_dir().join("daemon.pid");
                if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
                    if let Ok(pid) = pid_str.trim().parse::<i32>() {
                        unsafe { libc::kill(pid, libc::SIGTERM); }
                    }
                }
            }
            crate::tui::tabs::settings::SettingsAction::RestartDaemon => {
                let pid_path = config::data_dir().join("daemon.pid");
                if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
                    if let Ok(pid) = pid_str.trim().parse::<i32>() {
                        unsafe { libc::kill(pid, libc::SIGTERM); }
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

Also add `libc` to `Cargo.toml` dependencies if not already present:
```toml
libc = "0.2"
```

- [ ] **Step 3: Verify it compiles and all tests pass**

Run: `cargo check && cargo test`
Expected: compiles, all tests pass

- [ ] **Step 4: Commit**

```bash
git add src/tui/tabs/settings.rs src/tui/app.rs Cargo.toml
git commit -m "feat: make settings tab editable with inline editing"
```

---

### Task 3: Update Module — Check for Updates

**Files:**
- Create: `src/update.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Add semver dependency to Cargo.toml**

Add to `[dependencies]`:
```toml
semver = "1"
```

- [ ] **Step 2: Add module declaration to lib.rs**

Add `pub mod update;` to `src/lib.rs`.

- [ ] **Step 3: Create src/update.rs**

```rust
use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use std::path::Path;

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/weedgrease/snag/releases/latest";

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub download_url: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

fn platform_asset_name() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    format!("snag-{}-{}", arch, os)
}

fn parse_version(tag: &str) -> Option<Version> {
    let trimmed = tag.strip_prefix('v').unwrap_or(tag);
    Version::parse(trimmed).ok()
}

pub async fn check_for_update() -> Result<Option<UpdateInfo>> {
    let client = reqwest::Client::builder()
        .user_agent(format!("snag/{}", CURRENT_VERSION))
        .build()?;

    let release: GitHubRelease = client
        .get(GITHUB_RELEASES_URL)
        .send()
        .await
        .context("failed to fetch latest release")?
        .error_for_status()
        .context("GitHub API returned an error")?
        .json()
        .await
        .context("failed to parse release JSON")?;

    let current = match Version::parse(CURRENT_VERSION) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    let latest = match parse_version(&release.tag_name) {
        Some(v) => v,
        None => return Ok(None),
    };

    if latest <= current {
        return Ok(None);
    }

    let expected_name = platform_asset_name();
    let download_url = release
        .assets
        .iter()
        .find(|a| a.name == expected_name)
        .map(|a| a.browser_download_url.clone());

    let download_url = match download_url {
        Some(url) => url,
        None => return Ok(None),
    };

    Ok(Some(UpdateInfo {
        latest_version: release.tag_name,
        download_url,
    }))
}

pub async fn perform_update(info: &UpdateInfo) -> Result<()> {
    let current_exe = std::env::current_exe().context("failed to determine current executable")?;
    let exe_dir = current_exe
        .parent()
        .context("failed to determine executable directory")?;

    println!("Downloading {}...", info.latest_version);

    let client = reqwest::Client::builder()
        .user_agent(format!("snag/{}", CURRENT_VERSION))
        .build()?;

    let bytes = client
        .get(&info.download_url)
        .send()
        .await
        .context("failed to download update")?
        .error_for_status()
        .context("download returned an error")?
        .bytes()
        .await
        .context("failed to read download body")?;

    let temp_path = exe_dir.join("snag.tmp");
    let backup_path = exe_dir.join("snag.bak");

    std::fs::write(&temp_path, &bytes).context("failed to write temp file")?;

    set_executable(&temp_path)?;

    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    std::fs::rename(&current_exe, &backup_path).context("failed to backup current binary")?;

    if let Err(e) = std::fs::rename(&temp_path, &current_exe) {
        let _ = std::fs::rename(&backup_path, &current_exe);
        return Err(e).context("failed to install new binary, restored backup");
    }

    let _ = std::fs::remove_file(&backup_path);

    println!(
        "Updated snag from v{} to {}",
        CURRENT_VERSION, info.latest_version
    );

    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(path, perms).context("failed to set executable permissions")?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

pub async fn run_update() -> Result<()> {
    println!("Checking for updates...");

    let info = check_for_update()
        .await?
        .context("already up to date")?;

    perform_update(&info).await
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add src/update.rs src/lib.rs Cargo.toml
git commit -m "feat: add update module with GitHub release check and self-replace"
```

---

### Task 4: Add Update CLI Subcommand

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add Update variant to Commands enum and wire it up**

Replace `src/main.rs`:

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
    Update,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => snag::tui::run().await,
        Some(Commands::Daemon) => snag::daemon::run().await,
        Some(Commands::Check) => snag::daemon::check_once().await,
        Some(Commands::Update) => snag::update::run_update().await,
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add snag update CLI subcommand"
```

---

### Task 5: Background Update Check in TUI

**Files:**
- Modify: `src/tui/app.rs`

- [ ] **Step 1: Add update_info and update_rx fields to App**

In `src/tui/app.rs`, add to the `App` struct:

```rust
pub update_info: Option<crate::update::UpdateInfo>,
update_rx: Option<tokio::sync::oneshot::Receiver<Option<crate::update::UpdateInfo>>>,
```

- [ ] **Step 2: Spawn background update check in App::new()**

Update `App::new()` to spawn the check if enabled:

```rust
pub fn new() -> Result<Self> {
    let config_path = config::config_path();
    let results_path = results_path();
    let config = load_config(&config_path).unwrap_or_default();
    let results = load_results(&results_path).unwrap_or_default();

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
        theme: Theme::default(),
        alerts_tab: AlertsTab::new(),
        results_tab: ResultsTab::new(),
        settings_tab: SettingsTab::new(),
        should_quit: false,
        active_dialog: None,
        update_info: None,
        update_rx,
    })
}
```

- [ ] **Step 3: Poll for update result in the event loop**

In the `run()` method, add polling after the results refresh block (before the quit check):

```rust
if let Some(ref mut rx) = self.update_rx {
    if let Ok(result) = rx.try_recv() {
        if let Some(info) = result {
            self.settings_tab.update_banner =
                Some(format!("Update available: {} — run `snag update`", info.latest_version));
            self.update_info = Some(info);
        }
        self.update_rx = None;
    }
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat: add background update check on TUI launch"
```

---

### Task 6: Final Verification

**Files:** None — verification only

- [ ] **Step 1: Run all tests**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Fix any clippy warnings**

Address any issues found.

- [ ] **Step 4: Build release**

Run: `cargo build --release`
Expected: builds successfully

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore: fix clippy warnings and verify integration"
```
