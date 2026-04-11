use crate::tui::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders};

pub struct LogsTab {
    state: tui_logger::TuiWidgetState,
    selector_focused: bool,
    target_selected: bool,
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
            target_selected: false,
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
                KeyCode::Enter => {
                    self.target_selected = !self.target_selected;
                    self.state.transition(tui_logger::TuiWidgetEvent::FocusKey);
                    if self.target_selected {
                        self.selector_focused = false;
                    }
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::PageUp => {
                    self.state
                        .transition(tui_logger::TuiWidgetEvent::PrevPageKey);
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::PageDown => {
                    self.state
                        .transition(tui_logger::TuiWidgetEvent::NextPageKey);
                }
                KeyCode::Esc => {
                    if self.target_selected {
                        self.state.transition(tui_logger::TuiWidgetEvent::FocusKey);
                        self.target_selected = false;
                    }
                    self.selector_focused = true;
                }
                _ => {}
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(40)])
            .split(area);

        let selector_border_color = if self.selector_focused {
            theme.accent
        } else {
            theme.border
        };
        let selector_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(selector_border_color))
            .border_type(BorderType::Rounded)
            .title(Span::styled(
                " Targets ",
                Style::default()
                    .fg(selector_border_color)
                    .add_modifier(Modifier::BOLD),
            ));

        let hl_style = if self.selector_focused {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else if self.target_selected {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.fg)
        };

        let selector = tui_logger::TuiLoggerTargetWidget::default()
            .style_show(Style::default().fg(theme.fg))
            .style_hide(Style::default().fg(theme.fg_dim))
            .style_off(Style::default().fg(theme.fg_dim))
            .highlight_style(hl_style)
            .block(selector_block)
            .state(&self.state);
        frame.render_widget(selector, chunks[0]);

        let log_border_color = if self.selector_focused {
            theme.border
        } else {
            theme.accent
        };
        let log_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(log_border_color))
            .border_type(BorderType::Rounded)
            .title(Span::styled(
                " Logs ",
                Style::default()
                    .fg(log_border_color)
                    .add_modifier(Modifier::BOLD),
            ));

        let logs = tui_logger::TuiLoggerWidget::default()
            .style_error(Style::default().fg(theme.disabled))
            .style_warn(Style::default().fg(theme.unread))
            .style_info(Style::default().fg(theme.fg))
            .style_debug(Style::default().fg(theme.fg_dim))
            .style_trace(Style::default().fg(theme.fg_dim))
            .output_timestamp(Some("%H:%M:%S".to_string()))
            .block(log_block)
            .state(&self.state);
        frame.render_widget(logs, chunks[1]);
    }
}
