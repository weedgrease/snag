# Design System

## Product Context

snag is a terminal-based marketplace listing alert tool. It monitors Facebook Marketplace and eBay for new listings matching user-defined search criteria, then surfaces them in a keyboard-driven TUI.

**Target users**: Bargain hunters, resellers, collectors, and anyone who needs timely alerts on marketplace listings without manual searching.

**Core problem**: Marketplace platforms lack robust saved-search alerting. snag fills this gap with a single binary that runs as a TUI, background daemon, or one-shot checker.

## Aesthetic Direction

**"Alert Scanner"** -- clean, focused, functional. The interface prioritizes information density and scannability over decoration. Dark terminal background with selective color to draw attention to what matters: new listings, status changes, and active controls.

No ornamental elements. Every visual signal has a purpose.

## Color Palette

Sourced from `src/tui/theme.rs`:

| Role           | Color                  | Hex / Name     | Usage                                      |
|----------------|------------------------|----------------|---------------------------------------------|
| Background     | Terminal default        | `Reset`        | Inherits user's terminal background         |
| Foreground     | White                  | `#FFFFFF`      | Primary text                                |
| Dim            | DarkGray               | `#808080`      | Labels, inactive text, hints                |
| Accent         | Cyan                   | `#00FFFF`      | Active tab, focused borders, key highlights |
| Active tab     | Cyan                   | `#00FFFF`      | Selected tab indicator                      |
| Inactive tab   | DarkGray               | `#808080`      | Unselected tab labels                       |
| Border         | DarkGray               | `#808080`      | Panel borders (unfocused)                   |
| Selected BG    | RGB(40, 40, 60)        | `#28283C`      | Highlighted row background                  |
| Enabled        | Green                  | `#00FF00`      | Enabled alerts, ready status                |
| Disabled       | Red                    | `#FF0000`      | Disabled alerts, error status               |
| Unread         | Yellow                 | `#FFFF00`      | New/unseen listing indicator                |
| Status bar BG  | RGB(30, 30, 50)        | `#1E1E32`      | Bottom status bar background                |
| Status bar FG  | White                  | `#FFFFFF`      | Status bar text                             |

## TUI Layout

### Top-level structure (vertical)

```
+------------------------------------------+
| Tab bar: snag  [ Alerts ]  Results  ...  |
+------------------------------------------+
| Content area (tab-dependent)             |
|                                          |
|                                          |
+------------------------------------------+
| Update bar (conditional)                 |
+------------------------------------------+
| Status bar: Tab/1-4  n New | e Edit | ...| 
+------------------------------------------+
```

- **Tab bar**: 3 rows. Brand label left, bracketed active tab, plain inactive tabs. Bottom border (rounded).
- **Content area**: Fills remaining vertical space. Layout varies per tab.
- **Update bar**: Only visible when an update is available. Yellow text on selected_bg.
- **Status bar**: 1 row. Key hints with `|` separators. Accent-colored keys, dim descriptions.

### Tab content layouts

**Alerts / Results tabs**: Horizontal split-pane. Left sidebar (alert/listing list) + right detail panel. Focus indicator via cyan border on active pane.

**Settings tab**: Single panel with two sections (Defaults, Marketplaces). Arrow cursor (`>`) on selected field. Inline editing with underline indicator.

**Logs tab**: Full-width log viewer via `tui-logger`.

## Interaction Patterns

- **Navigation**: `Tab`/`Shift-Tab` or `1-4` for tab switching. `j`/`k` or arrow keys for list navigation.
- **Actions**: Single-key commands (`n` new, `e` edit, `d` delete, `f` force check, `o` open).
- **Confirmation**: `Enter` to confirm, `Esc` to cancel. Consistent across all dialogs.
- **Focus transfer**: `Enter`/`l` to move focus into listings pane, `Esc` to return.
- **Toggle**: `Space` to enable/disable alerts.
- **Quit**: `q` from any non-dialog context, `Ctrl-C` always.

## Status Indicators

| Icon | Meaning             | Color   |
|------|----------------------|---------|
| `●`  | Alert enabled        | Green   |
| `○`  | Alert disabled       | Red     |
| `●`  | New/unseen listing   | Yellow  |
| `  ` | Seen listing         | (none)  |
| `▸`  | Selected setting     | Cyan    |

## Typography

Monospace terminal font (user's default). No font selection or sizing -- the terminal controls this entirely.

- **Bold**: Tab titles, panel headers, alert names, listing titles.
- **Underlined**: Actively edited settings fields.
- **Normal weight**: Detail table values, descriptions, status text.
- **Dim (DarkGray)**: Labels, hints, inactive elements.

## Component Patterns

### Blocks
All panels use `Block` with `BorderType::Rounded`. Focused panels get `accent` border color; unfocused panels get `border` (DarkGray).

### Tables
Key-value detail display uses `Table` widget with two columns: 16-char fixed label (dim) + flexible value (fg). No visible borders on the table itself.

### Lists
Scrollable lists with `List` widget. `Scrollbar` (vertical right) appears when content exceeds viewport. Selected item gets `selected_bg` background.

### Dialogs
Centered overlay on `frame.area()`. Same `Block` + `BorderType::Rounded` pattern. Captures all keyboard input while active. `Enter` submits, `Esc` cancels.

### Status Bar
Single row. First segment is inverted (fg on accent bg) showing global navigation. Remaining segments are accent-colored keys with dim descriptions, separated by `|` in border color.

### Text Truncation
Long text is truncated with `...` via `truncate_str()` to prevent layout overflow. Applied to alert names and listing titles in list views.
