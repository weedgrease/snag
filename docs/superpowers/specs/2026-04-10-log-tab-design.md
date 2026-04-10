# Log Tab Design

Add a Logs tab to the TUI showing real-time activity from the scheduler and marketplace providers, with configurable log levels.

## LogEntry and LogLevel

```rust
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
}

pub enum LogLevel {
    Info,
    Debug,
    Error,
}
```

## Log Channel

A dedicated `mpsc::Sender<LogEntry>` separate from the scheduler event channel. Passed to:
- Scheduler (check start/complete, config reload)
- Facebook marketplace provider (location resolution, search results, cache hits)
- Notifiers (notification sent)

The TUI holds the receiver and drains it via `try_recv()` in the event loop alongside scheduler events.

For non-owner TUI instances (fallback mode), the log channel doesn't exist. The Logs tab shows "Logs available when scheduler is active in this instance."

## Config

Add `log_level` to `GlobalSettings`, defaulting to `Info`:

```rust
#[serde(default)]
pub log_level: LogLevel,
```

When `Info`: scheduler activity, errors, high-level provider events.
When `Debug`: adds HTTP request details, response parsing, location cache hits, timing.

Editable in the Settings tab as a 6th field (cycle with Enter like the notification field).

## Tab

Fourth tab: Alerts | Results | Settings | Logs

Scrollable list, newest entries at the bottom, auto-scrolls to bottom on new entries unless the user has scrolled up.

Each line: `[HH:MM:SS] [LEVEL] message`

Level colors:
- Info: default fg
- Debug: dim
- Error: red (theme.disabled)

Keybindings: `j/k` or arrows to scroll, `G` to jump to bottom, `c` to clear.

## Buffer

200 entries max in memory. When full, oldest entry is dropped. No file persistence.

## Files Changed

| File | Change |
|---|---|
| `src/types.rs` | Add `LogEntry`, `LogLevel` |
| `src/config.rs` | Add `log_level` to GlobalSettings |
| `src/scheduler.rs` | Accept and use log sender |
| `src/marketplace/providers/facebook.rs` | Accept and use log sender |
| `src/notifier/providers/terminal.rs` | Accept and use log sender |
| `src/marketplace/mod.rs` | Update trait to pass log sender |
| `src/tui/tabs/mod.rs` | Add Logs tab variant |
| `src/tui/tabs/logs.rs` | NEW — Logs tab rendering and input |
| `src/tui/tabs/settings.rs` | Add log_level as 6th field |
| `src/tui/app.rs` | Hold log receiver, drain in event loop, pass sender to scheduler/providers |
| `src/daemon/mod.rs` | Pass log sender to scheduler |
| `tests/config_test.rs` | Update for new field |
| `tests/daemon_test.rs` | Update for new field |
