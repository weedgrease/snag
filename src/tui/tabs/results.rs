use crate::tui::theme::Theme;
use crate::types::AlertResult;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

pub struct ResultsTab {
    pub selected: usize,
    pub list_state: ratatui::widgets::ListState,
}

impl ResultsTab {
    pub fn new() -> Self {
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(0));
        Self { selected: 0, list_state }
    }

    pub fn handle_key(&mut self, _key: KeyEvent, _results: &mut Vec<AlertResult>) -> Option<ResultsAction> {
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, results: &[AlertResult]) {
        let _ = (frame, area, theme, results);
    }
}

pub enum ResultsAction {
    OpenUrl(String),
    ResultsChanged,
}
