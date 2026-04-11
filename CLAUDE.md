# CLAUDE.md

## Project

snag is a Rust TUI for monitoring marketplace listings (Facebook Marketplace, eBay) with configurable alerts. Single binary with three modes: `snag` (TUI), `snag daemon` (headless), `snag check` (one-shot).

## Build & Test

```bash
cargo build              # debug build
cargo build --release    # release build
cargo test               # run all tests
cargo clippy -- -D warnings  # lint (must pass clean)
cargo run                # launch TUI
cargo run -- daemon      # headless daemon
cargo run -- check       # one-shot check
cargo run -- update      # self-update from GitHub
```

## Architecture

```
main.rs          CLI entry (clap subcommands)
lib.rs           Module declarations
types.rs         Core domain types (Alert, Listing, AlertResult, CheckStatus)
config.rs        TOML config at ~/.config/snag/
credentials.rs   Keyring-based secret storage (eBay API keys)
scheduler.rs     Shared scheduling logic with MPSC channels
update.rs        Self-update from GitHub releases
daemon/
  mod.rs         Headless daemon mode (uses Scheduler)
  results.rs     File-based persistence (results.json, status.json, seen.json) with fs2 locking
marketplace/
  mod.rs         Marketplace trait
  rate_limit.rs  Per-marketplace rate limit persistence
  providers/     Facebook (GraphQL), eBay (Browse API + OAuth)
notifier/
  mod.rs         Notifier trait
  providers/     Terminal notifier
tui/
  mod.rs         Terminal setup/teardown with panic hook
  app.rs         Main App struct, event loop, dialog dispatch
  theme.rs       Color scheme
  utils.rs       Shared utilities (truncate_str)
  tabs/          Alerts, Results, Settings, Logs
  dialogs/       Alert form, Confirm, Listing detail, eBay setup
```

## Key Design Decisions

- **In-process scheduler**: The first TUI instance acquires a PID file lock and runs the scheduler internally via MPSC channels. Additional instances fall back to mtime-based file polling. `snag daemon` uses the same Scheduler but writes to files.
- **File-based IPC**: TUI instances sync via shared JSON files (results.json, status.json, seen.json) with fs2 advisory locking. Follows the agent-of-empires pattern.
- **Logging**: Uses the `log` crate facade with `tui-logger` for TUI rendering. The daemon uses `tracing-subscriber` for file-based logging (compatible via log bridge).
- **Credentials**: eBay API keys stored in OS keyring via `keyring` crate. Never written to config files.
- **Facebook API**: Uses undocumented internal GraphQL endpoint with hardcoded doc_ids. Fragile — can break without notice.
- **Rate limiting**: Persisted to `~/.local/share/snag/rate_limit_{marketplace}` as RFC 3339 timestamps. Survives restarts.

## Conventions

- Rust edition 2024 (enables let-chains)
- No inline comments inside function bodies
- `///` doc comments on public types and important functions only
- `anyhow::Result` for error handling throughout
- Clippy must pass with `-D warnings`
- Tests in `tests/` directory (integration-level)
- Commit messages follow conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `style:`

## Visual Changes

Read `DESIGN.md` before making any visual or TUI changes. The TUI uses the **Dracula** color palette (`src/tui/theme.rs`). All colors must come from the theme — never hardcode `Color::` values directly. The Logs tab accepts `&Theme` just like all other tabs.

## CI

All PRs must pass CI. Run locally before pushing:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Release Process

1. Update version in `Cargo.toml`
2. Commit: `chore: bump version to 0.X.Y`
3. Tag: `git tag v0.X.Y`
4. Push tag: `git push origin v0.X.Y`

The `release.yml` workflow builds binaries for Linux (x86_64, aarch64) and macOS (x86_64, aarch64), then creates a GitHub release with artifacts.

## Adding a New Marketplace Provider

1. Add variant to `MarketplaceKind` in `src/types.rs` (with Display impl)
2. Create `src/marketplace/providers/{name}.rs` implementing the `Marketplace` trait
3. Add `pub mod {name}` to `src/marketplace/providers/mod.rs`
4. Add match arm to `create_marketplace()` in `src/marketplace/mod.rs`
5. Update `from_alert()` and `to_alert()` in `src/tui/dialogs/alert_form.rs` for parsing
6. If credentials needed: add to `src/credentials.rs`, create setup dialog, add to Settings tab marketplaces section
7. Use `rate_limit::set_rate_limited("{name}", duration)` for rate limiting

## Adding a New Notifier

1. Add variant to `NotifierKind` in `src/types.rs`
2. Create `src/notifier/providers/{name}.rs` implementing the `Notifier` trait
3. Add to `create_notifier()` in `src/notifier/mod.rs`
4. Update Settings tab if configuration needed

## Data Files

- `~/.config/snag/config.toml` — alerts and settings
- `~/.local/share/snag/results.json` — matched listings
- `~/.local/share/snag/status.json` — per-alert check status
- `~/.local/share/snag/seen.json` — seen listing IDs
- `~/.local/share/snag/daemon.pid` — PID lock file
- `~/.local/share/snag/daemon.log` — daemon log output
- `~/.local/share/snag/rate_limit_{marketplace}` — rate limit timestamps

## Known Limitations

- Facebook GraphQL doc_ids are hardcoded and can break if Facebook changes their internal API
- eBay Browse API requires developer account registration
- No checksum verification on self-update downloads
- `app.rs` is large (~650 lines) — business logic should be extracted for testability
- Test coverage is integration-level only; unit tests for scheduler logic and API parameter building are needed
