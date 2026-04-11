use ratatui::style::Color;

/// Visual theme for the TUI. Uses the Dracula color palette.
/// See https://draculatheme.com/contribute for canonical values.
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

// Dracula palette constants
const BACKGROUND: Color = Color::Rgb(40, 42, 54); // #282a36
const CURRENT_LINE: Color = Color::Rgb(68, 71, 90); // #44475a
const FOREGROUND: Color = Color::Rgb(248, 248, 242); // #f8f8f2
const COMMENT: Color = Color::Rgb(98, 114, 164); // #6272a4
const _CYAN: Color = Color::Rgb(139, 233, 253); // #8be9fd
const GREEN: Color = Color::Rgb(80, 250, 123); // #50fa7b
const _ORANGE: Color = Color::Rgb(255, 184, 108); // #ffb86c
const _PINK: Color = Color::Rgb(255, 121, 198); // #ff79c6
const PURPLE: Color = Color::Rgb(189, 147, 249); // #bd93f9
const RED: Color = Color::Rgb(255, 85, 85); // #ff5555
const YELLOW: Color = Color::Rgb(241, 250, 140); // #f1fa8c

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: BACKGROUND,
            fg: FOREGROUND,
            fg_dim: COMMENT,
            accent: PURPLE,
            active_tab: PURPLE,
            inactive_tab: COMMENT,
            border: COMMENT,
            selected_bg: CURRENT_LINE,
            enabled: GREEN,
            disabled: RED,
            unread: YELLOW,
            status_bar_bg: CURRENT_LINE,
            status_bar_fg: FOREGROUND,
        }
    }
}
