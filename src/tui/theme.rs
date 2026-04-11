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

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Rgb(40, 42, 54),          // Dracula Background  #282a36
            fg: Color::Rgb(248, 248, 242),        // Dracula Foreground  #f8f8f2
            fg_dim: Color::Rgb(98, 114, 164),     // Dracula Comment     #6272a4
            accent: Color::Rgb(189, 147, 249),     // Dracula Purple      #bd93f9
            active_tab: Color::Rgb(189, 147, 249), // Dracula Purple      #bd93f9
            inactive_tab: Color::Rgb(98, 114, 164), // Dracula Comment    #6272a4
            border: Color::Rgb(98, 114, 164),      // Dracula Comment     #6272a4
            selected_bg: Color::Rgb(68, 71, 90),   // Dracula Current Line #44475a
            enabled: Color::Rgb(80, 250, 123),     // Dracula Green       #50fa7b
            disabled: Color::Rgb(255, 85, 85),     // Dracula Red         #ff5555
            unread: Color::Rgb(241, 250, 140),     // Dracula Yellow      #f1fa8c
            status_bar_bg: Color::Rgb(68, 71, 90), // Dracula Current Line #44475a
            status_bar_fg: Color::Rgb(248, 248, 242), // Dracula Foreground #f8f8f2
        }
    }
}
