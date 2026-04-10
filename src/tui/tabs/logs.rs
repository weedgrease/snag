use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct LogsTab {
    state: tui_logger::TuiWidgetState,
}

impl Default for LogsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl LogsTab {
    pub fn new() -> Self {
        Self {
            state: tui_logger::TuiWidgetState::new()
                .set_default_display_level(log::LevelFilter::Info),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(' ') => self.state.transition(tui_logger::TuiWidgetEvent::SpaceKey),
            KeyCode::Esc => self.state.transition(tui_logger::TuiWidgetEvent::EscapeKey),
            KeyCode::PageUp => self.state.transition(tui_logger::TuiWidgetEvent::PrevPageKey),
            KeyCode::PageDown => self.state.transition(tui_logger::TuiWidgetEvent::NextPageKey),
            KeyCode::Up => self.state.transition(tui_logger::TuiWidgetEvent::UpKey),
            KeyCode::Down => self.state.transition(tui_logger::TuiWidgetEvent::DownKey),
            KeyCode::Left => self.state.transition(tui_logger::TuiWidgetEvent::LeftKey),
            KeyCode::Right => self.state.transition(tui_logger::TuiWidgetEvent::RightKey),
            KeyCode::Char('h') => self.state.transition(tui_logger::TuiWidgetEvent::HideKey),
            KeyCode::Char('f') => self.state.transition(tui_logger::TuiWidgetEvent::FocusKey),
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let widget = tui_logger::TuiLoggerSmartWidget::default()
            .style_error(ratatui::style::Style::default().fg(ratatui::style::Color::Red))
            .style_warn(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
            .style_info(ratatui::style::Style::default().fg(ratatui::style::Color::White))
            .style_debug(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
            .style_trace(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
            .output_timestamp(Some("%H:%M:%S".to_string()))
            .title_target("Target Selector")
            .state(&self.state);
        frame.render_widget(widget, area);
    }
}
