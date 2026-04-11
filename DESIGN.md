# Design System

## Product Context

snag is a terminal-based marketplace listing alert tool. It monitors Facebook Marketplace and eBay for new listings matching user-defined search criteria, then surfaces them in a keyboard-driven TUI.

**Target users**: Bargain hunters, resellers, collectors, and anyone who needs timely alerts on marketplace listings without manual searching.

**Core problem**: Marketplace platforms lack robust saved-search alerting. snag fills this gap with a single binary that runs as a TUI, background daemon, or one-shot checker.

## Aesthetic Direction

**"Alert Scanner"** with a **Dracula** color palette. Clean, focused, functional. The interface prioritizes information density and scannability over decoration. Dark background with selective color to draw attention to what matters: new listings, status changes, and active controls.

No ornamental elements. Every visual signal has a purpose.

## Color Palette

[Dracula Theme](https://draculatheme.com/contribute) sourced from `src/tui/theme.rs`:

| Role           | Color                  | Hex       | Usage                                      |
|----------------|------------------------|-----------|--------------------------------------------|
| Background     | Dracula Background     | `#282a36` | Panel backgrounds                          |
| Foreground     | Dracula Foreground     | `#f8f8f2` | Primary text                               |
| Dim            | Dracula Comment        | `#6272a4` | Labels, inactive text, hints, borders      |
| Accent         | Dracula Purple         | `#bd93f9` | Active tab, focused borders, key highlights|
| Active tab     | Dracula Purple         | `#bd93f9` | Selected tab indicator                     |
| Inactive tab   | Dracula Comment        | `#6272a4` | Unselected tab labels                      |
| Border         | Dracula Comment        | `#6272a4` | Panel borders (unfocused)                  |
| Selected BG    | Dracula Current Line   | `#44475a` | Highlighted row background                 |
| Enabled        | Dracula Green          | `#50fa7b` | Enabled alerts, ready marketplace status   |
| Disabled       | Dracula Red            | `#ff5555` | Disabled alerts, error status              |
| Unread         | Dracula Yellow         | `#f1fa8c` | New/unseen listing indicator               |
| Price          | Dracula Cyan           | `#8be9fd` | Listing prices                             |
| Update         | Dracula Orange         | `#ffb86c` | Update available notification              |
| Marketplace    | Dracula Pink           | `#ff79c6` | Marketplace name indicators                |
| Status bar BG  | Dracula Current Line   | `#44475a` | Bottom status bar background               |
| Status bar FG  | Dracula Foreground     | `#f8f8f2` | Status bar text                            |

All 10 Dracula palette colors are in use.

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

**Logs tab**: Horizontal split-pane. Left target selector (30 columns) + right log viewer. Focus indicator via accent border on active pane.

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
| `●`  | Alert enabled        | Green (`#50fa7b`)  |
| `○`  | Alert disabled       | Red (`#ff5555`)    |
| `●`  | New/unseen listing   | Yellow (`#f1fa8c`) |
| `  ` | Seen listing         | (none)             |
| `▸`  | Selected setting     | Purple (`#bd93f9`) |

## Typography

Monospace terminal font (user's default). No font selection or sizing -- the terminal controls this entirely.

- **Bold**: Tab titles, panel headers, alert names, listing titles.
- **Underlined**: Actively edited settings fields.
- **Normal weight**: Detail table values, descriptions, status text.
- **Dim (Comment `#6272a4`)**: Labels, hints, inactive elements.

## Component Patterns

### Blocks
All panels use `Block` with `BorderType::Rounded`. Focused panels get `accent` (Purple) border color; unfocused panels get `border` (Comment).

### Tables
Key-value detail display uses `Table` widget with two columns: 16-char fixed label (dim) + flexible value (fg). No visible borders on the table itself.

### Lists
Scrollable lists with `List` widget. `Scrollbar` (vertical right) appears when content exceeds viewport. Selected item gets `selected_bg` background.

### Dialogs
Centered overlay on `frame.area()`. Same `Block` + `BorderType::Rounded` pattern. Captures all keyboard input while active. `Enter` submits, `Esc` cancels.

### Status Bar
Single row. First segment is inverted (fg on accent bg) showing global navigation. Remaining segments are accent-colored keys with dim descriptions, separated by `|` in border color.

### Text Truncation
Long text is truncated with `…` (Unicode ellipsis) via `truncate_str()` to prevent layout overflow. Applied to alert names and listing titles in list views.
