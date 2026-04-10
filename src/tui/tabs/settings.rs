use crate::config::AppConfig;
use crate::tui::theme::Theme;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

pub struct SettingsTab {
    pub selected: usize,
}

impl SettingsTab {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn handle_key(&mut self, _key: KeyEvent, _config: &mut AppConfig) -> Option<SettingsAction> {
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig) {
        let _ = (frame, area, theme, config);
    }
}

pub enum SettingsAction {
    StartDaemon,
    StopDaemon,
    RestartDaemon,
}
