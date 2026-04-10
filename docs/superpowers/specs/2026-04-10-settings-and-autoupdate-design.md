# Editable Settings + Auto-Update Design

Two features: making the Settings tab's defaults editable, and adding auto-update via GitHub releases.

## Feature 1: Editable Settings

The Settings tab's Defaults section becomes an inline-editable form, following the same interaction pattern as the alert form dialog.

### Interaction

Arrow keys navigate between the 4 fields. Enter starts editing the selected field. Enter or Esc stops editing. Changes save to `config.toml` immediately. The daemon picks up changes via its existing config hot-reload.

### Fields

| Field | Input type | Behavior |
|---|---|---|
| Check interval | Text, parsed as seconds | Validates as u64 on save |
| Max results | Text, parsed as u32 | Empty means unlimited (None) |
| Notification | Cycle with left/right arrows | Cycles through NotifierKind variants |
| Check for updates | Toggle with Enter | Flips boolean |

### Config Change

Add `check_for_updates: bool` to `GlobalSettings`, defaulting to `true`. Uses `#[serde(default = "default_true")]` so existing config files without this field get `true`:

```rust
pub struct GlobalSettings {
    pub default_check_interval: Duration,
    pub default_max_results: Option<u32>,
    pub default_notifier: NotifierKind,
    #[serde(default = "default_true")]
    pub check_for_updates: bool,
}

fn default_true() -> bool { true }
```

### Settings Tab State

Replace the current read-only `SettingsTab` with state for inline editing:

```rust
pub struct SettingsTab {
    pub selected: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub field_count: usize,
}
```

When `editing` is true, keystrokes go to `edit_buffer`. On Enter/Esc, the buffer value is parsed and written back to `AppConfig.settings`, then config is saved.

### SettingsAction Changes

Add a new variant:

```rust
pub enum SettingsAction {
    StartDaemon,
    StopDaemon,
    RestartDaemon,
    ConfigChanged,
}
```

The app handles `ConfigChanged` by calling `save_config`.

## Feature 2: Auto-Update via GitHub Releases

### Background Update Check

On TUI launch, when `check_for_updates` is true, spawn a tokio background task:

1. GET `https://api.github.com/repos/weedgrease/snag/releases/latest`
2. Parse the `tag_name` field (e.g., `v0.2.0`)
3. Compare against current version from `env!("CARGO_PKG_VERSION")`
4. If newer, extract the download URL for the matching platform asset
5. Store result in `App.update_info: Option<UpdateInfo>`

```rust
pub struct UpdateInfo {
    pub latest_version: String,
    pub download_url: String,
}
```

The Settings tab displays "Update available: vX.Y.Z — run `snag update`" when `update_info` is Some.

The background check uses a oneshot channel (same pattern as agent-of-empires): spawn the task, poll the receiver in the event loop, set `update_info` when the result arrives.

### `snag update` Subcommand

New CLI subcommand added to the clap parser:

```rust
enum Commands {
    Daemon,
    Check,
    Update,
}
```

Execution flow:
1. GET the latest release from `https://api.github.com/repos/weedgrease/snag/releases/latest`
2. Parse version, compare with current. If not newer, print "Already up to date" and exit.
3. Find the asset matching the current platform. Asset naming convention: `snag-{arch}-{os}` (e.g., `snag-x86_64-linux`). Match using `std::env::consts::ARCH` and `std::env::consts::OS`.
4. Download the asset to a temporary file in the same directory as the current binary.
5. Replace the binary:
   - Rename current binary to `snag.bak`
   - Rename downloaded temp file to the original binary path
   - Set executable permissions (`chmod +x`)
   - Delete `snag.bak`
6. Print success: "Updated snag from vX.Y.Z to vA.B.C"

If any step fails after the rename, attempt to restore from `.bak`.

### New Module

Create `src/update.rs` containing:
- `check_for_update()` — async function returning `Option<UpdateInfo>`, used by both the TUI background check and the `snag update` command
- `perform_update(info: &UpdateInfo)` — downloads and replaces the binary
- Platform matching logic
- GitHub API response types

### New Dependency

- `semver` — for proper version comparison

`reqwest` is already in dependencies for marketplace adapters.

## Files Changed

| File | Change |
|---|---|
| `src/config.rs` | Add `check_for_updates` field to GlobalSettings |
| `src/types.rs` | No changes |
| `src/main.rs` | Add `Update` subcommand |
| `src/update.rs` | New module — update check + binary replacement |
| `src/lib.rs` | Add `pub mod update;` |
| `src/tui/tabs/settings.rs` | Inline editing, update banner |
| `src/tui/app.rs` | Handle ConfigChanged from settings, background update check, poll for result |
| `src/tui/mod.rs` | No changes |
| `Cargo.toml` | Add `semver` dependency |
| `tests/config_test.rs` | Update test for new field |
