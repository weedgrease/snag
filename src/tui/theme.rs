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
