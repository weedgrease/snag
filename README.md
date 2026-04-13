# snag

Marketplace listing alerts in your terminal.

snag monitors Facebook Marketplace and eBay for new listings matching your search criteria and surfaces them in a keyboard-driven TUI. Run it interactively, as a background daemon, or as a one-shot cron job.

## Features

- **Multi-marketplace search** -- Facebook Marketplace and eBay from a single tool
- **Configurable alerts** -- keywords, price range, location/radius, condition, category
- **Real-time TUI** -- browse alerts, view listings, open in browser, all from the terminal
- **Background daemon** -- headless mode for continuous monitoring
- **One-shot mode** -- `snag check` for cron jobs and scripts
- **Self-update** -- `snag update` pulls the latest release from GitHub
- **Secure credential storage** -- API keys in a separate permissions-restricted file

## Installation

### Quick install (Linux / macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/weedgrease/snag/main/scripts/install.sh | bash
```

Detects your OS and architecture, downloads the latest release, and installs to `/usr/local/bin`.

### From source

```bash
git clone https://github.com/weedgrease/snag
cd snag && cargo install --path .
```

### Uninstall

```bash
snag uninstall
```

## Quick Start

1. Launch the TUI: `snag`
2. Press `n` to create a new alert
3. Fill in keywords, marketplace, price range, location
4. Press `s` to save -- snag begins checking automatically
5. New listings appear in the Alerts detail pane and the Results tab
6. Press `o` to open a listing in your browser

## Configuration

Config file: `~/.config/snag/config.toml`

Data files: `~/.local/share/snag/` (results, status, seen listings, daemon PID, logs)

Edit settings directly in the TUI via the Settings tab, or edit `config.toml` manually.

## Marketplace Providers

### Facebook Marketplace

No setup required. Uses an internal API endpoint. Works out of the box.

**Note**: Facebook's internal API can change without notice. If searches stop working, check for a snag update.

### eBay

Requires API credentials from the [eBay Developer Program](https://developer.ebay.com/).

1. Go to Settings tab in the TUI
2. Select "eBay" under Marketplaces and press Enter
3. Follow the setup wizard to register and enter your API keys

Credentials are stored in `~/.config/snag/credentials.toml` (file permissions restricted to owner-only).

## CLI Commands

| Command        | Description                                           |
|----------------|-------------------------------------------------------|
| `snag`         | Launch the interactive TUI                            |
| `snag daemon`  | Run as a headless background daemon                   |
| `snag check`   | Run a single check cycle and exit                     |
| `snag update`  | Check for and install the latest release from GitHub  |

## Key Bindings

### Global

| Key          | Action              |
|--------------|---------------------|
| `Tab`        | Next tab            |
| `Shift-Tab`  | Previous tab        |
| `1-4`        | Jump to tab         |
| `q`          | Quit                |
| `Ctrl-C`     | Force quit          |
| `u`          | Update (when available) |

### Alerts Tab

| Key     | Action                      |
|---------|-----------------------------|
| `j`/`k` | Navigate alert list         |
| `n`     | New alert                   |
| `e`     | Edit selected alert         |
| `d`     | Delete selected alert       |
| `Space` | Toggle enabled/disabled     |
| `f`     | Force check now             |
| `l`/`Enter` | Focus listings pane     |
| `Esc`   | Return to alert list        |
| `m`     | Filter by marketplace (listings pane) |
| `s`     | Sort listings (listings pane) |
| `c`     | Clear alert results (listings pane) |

### Results Tab

| Key     | Action                      |
|---------|-----------------------------|
| `j`/`k` | Navigate results            |
| `o`     | Open listing in browser     |
| `m`     | Mark as read                |
| `f`     | Filter by marketplace       |
| `s`     | Sort listings               |
| `Enter` | View listing details        |
| `c`     | Clear all results           |

### Settings Tab

| Key     | Action                      |
|---------|-----------------------------|
| `j`/`k` | Navigate settings           |
| `Enter` | Edit field / setup marketplace |
| `Esc`   | Cancel edit                 |

### Logs Tab

| Key        | Action              |
|------------|---------------------|
| `Up`/`Down` | Scroll             |
| `Left`/`Right` | Filter log level |
| `Enter`    | Focus log target    |
| `Esc`      | Clear focus         |

## Development

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run tests
cargo clippy -- -D warnings  # Lint (must pass clean)
cargo fmt --check        # Format check
cargo run                # Launch TUI
cargo run -- daemon      # Headless daemon
cargo run -- check       # One-shot check
```

## Architecture

```
src/
  main.rs          CLI entry point (clap subcommands)
  lib.rs           Module declarations
  types.rs         Core domain types (Alert, Listing, AlertResult, CheckStatus)
  config.rs        TOML config management
  credentials.rs   File-based credential storage
  scheduler.rs     Shared scheduling logic with MPSC channels
  update.rs        Self-update from GitHub releases
  daemon/          Headless daemon and file-based persistence
  marketplace/     Marketplace trait + providers (Facebook, eBay)
  notifier/        Notifier trait + providers (Terminal)
  tui/             Terminal UI (ratatui + crossterm)
```

See [CLAUDE.md](CLAUDE.md) for detailed architecture notes, design decisions, and contribution guides.

## License

[MIT](LICENSE)
