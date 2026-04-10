use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

pub struct LogsTab {
    state: tui_logger::TuiWidgetState,
    selector_focused: bool,
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
            selector_focused: true,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.selector_focused {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.transition(tui_logger::TuiWidgetEvent::UpKey);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.transition(tui_logger::TuiWidgetEvent::DownKey);
                }
                KeyCode::Left => {
                    self.state.transition(tui_logger::TuiWidgetEvent::LeftKey);
                }
                KeyCode::Right => {
                    self.state.transition(tui_logger::TuiWidgetEvent::RightKey);
                }
                KeyCode::Char('f') => {
                    self.state.transition(tui_logger::TuiWidgetEvent::FocusKey);
                }
                KeyCode::Esc => {
                    self.selector_focused = false;
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::PageUp => {
                    self.state.transition(tui_logger::TuiWidgetEvent::PrevPageKey);
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::PageDown => {
                    self.state.transition(tui_logger::TuiWidgetEvent::NextPageKey);
                }
                KeyCode::Esc | KeyCode::Enter => {
                    self.selector_focused = true;
                }
                KeyCode::Char('f') => {
                    self.state.transition(tui_logger::TuiWidgetEvent::FocusKey);
                }
                _ => {}
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(40)])
            .split(area);

        let selector_border_color = if self.selector_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let selector_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(selector_border_color))
            .title(Span::styled(
                " Targets ",
                Style::default()
                    .fg(selector_border_color)
                    .add_modifier(Modifier::BOLD),
            ));

        let hl_style = if self.selector_focused {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let selector = tui_logger::TuiLoggerTargetWidget::default()
            .style_show(Style::default().fg(Color::White))
            .style_hide(Style::default().fg(Color::DarkGray))
            .style_off(Style::default().fg(Color::DarkGray))
            .highlight_style(hl_style)
            .block(selector_block)
            .state(&self.state);
        frame.render_widget(selector, chunks[0]);

        let log_border_color = if self.selector_focused {
            Color::DarkGray
        } else {
            Color::Cyan
        };
        let log_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(log_border_color))
            .title(Span::styled(
                " Logs ",
                Style::default()
                    .fg(log_border_color)
                    .add_modifier(Modifier::BOLD),
            ));

        let logs = tui_logger::TuiLoggerWidget::default()
            .style_error(Style::default().fg(Color::Red))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_info(Style::default().fg(Color::White))
            .style_debug(Style::default().fg(Color::DarkGray))
            .style_trace(Style::default().fg(Color::DarkGray))
            .output_timestamp(Some("%H:%M:%S".to_string()))
            .block(log_block)
            .state(&self.state);
        frame.render_widget(logs, chunks[1]);
    }
}
