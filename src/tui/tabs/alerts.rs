use crate::config::AppConfig;
use crate::tui::theme::Theme;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

pub struct AlertsTab {
    pub selected: usize,
    pub list_state: ratatui::widgets::ListState,
}

impl AlertsTab {
    pub fn new() -> Self {
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(0));
        Self { selected: 0, list_state }
    }

    pub fn handle_key(&mut self, _key: KeyEvent, _config: &mut AppConfig) -> Option<AlertsAction> {
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig) {
        let _ = (frame, area, theme, config);
    }
}

pub enum AlertsAction {
    ConfigChanged,
    CreateAlert,
    EditAlert(usize),
    DeleteAlert(usize),
}
