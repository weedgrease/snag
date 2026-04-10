use crate::config::AppConfig;
use crate::tui::theme::Theme;
use crate::types::{LogLevel, NotifierKind};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::time::Duration;

const FIELD_CHECK_INTERVAL: usize = 0;
const FIELD_MAX_RESULTS: usize = 1;
const FIELD_NOTIFICATION: usize = 2;
const FIELD_CHECK_UPDATES: usize = 3;
const FIELD_DEFAULT_LOCATION: usize = 4;
const FIELD_LOG_LEVEL: usize = 5;
const FIELD_COUNT: usize = 6;

pub struct SettingsTab {
    pub selected: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub update_banner: Option<String>,
}

impl Default for SettingsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsTab {
    pub fn new() -> Self {
        Self {
            selected: 0,
            editing: false,
            edit_buffer: String::new(),
            update_banner: None,
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        config: &mut AppConfig,
    ) -> Option<SettingsAction> {
        if self.editing {
            match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                }
                KeyCode::Enter => {
                    self.apply_edit(config);
                    self.editing = false;
                    return Some(SettingsAction::ConfigChanged);
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                }
                _ => {}
            }
            return None;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected < FIELD_COUNT - 1 {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => {
                match self.selected {
                    FIELD_CHECK_UPDATES => {
                        config.settings.check_for_updates = !config.settings.check_for_updates;
                        return Some(SettingsAction::ConfigChanged);
                    }
                    FIELD_NOTIFICATION => {
                        config.settings.default_notifier = match config.settings.default_notifier {
                            NotifierKind::Terminal => NotifierKind::Terminal,
                        };
                        return Some(SettingsAction::ConfigChanged);
                    }
                    FIELD_LOG_LEVEL => {
                        config.settings.log_level = match config.settings.log_level {
                            LogLevel::Info => LogLevel::Debug,
                            LogLevel::Debug => LogLevel::Error,
                            LogLevel::Error => LogLevel::Info,
                        };
                        return Some(SettingsAction::ConfigChanged);
                    }
                    _ => {
                        self.editing = true;
                        self.edit_buffer = self.current_field_value(config);
                    }
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selected == FIELD_NOTIFICATION {
                    config.settings.default_notifier = match config.settings.default_notifier {
                        NotifierKind::Terminal => NotifierKind::Terminal,
                    };
                    return Some(SettingsAction::ConfigChanged);
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.selected == FIELD_NOTIFICATION {
                    config.settings.default_notifier = match config.settings.default_notifier {
                        NotifierKind::Terminal => NotifierKind::Terminal,
                    };
                    return Some(SettingsAction::ConfigChanged);
                }
            }
            _ => {}
        }
        None
    }

    fn current_field_value(&self, config: &AppConfig) -> String {
        match self.selected {
            FIELD_CHECK_INTERVAL => config.settings.default_check_interval.as_secs().to_string(),
            FIELD_MAX_RESULTS => config
                .settings
                .default_max_results
                .map(|m| m.to_string())
                .unwrap_or_default(),
            FIELD_DEFAULT_LOCATION => config
                .settings
                .default_location
                .clone()
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn apply_edit(&self, config: &mut AppConfig) {
        match self.selected {
            FIELD_CHECK_INTERVAL => {
                if let Ok(secs) = self.edit_buffer.trim().parse::<u64>()
                    && secs > 0 {
                        config.settings.default_check_interval = Duration::from_secs(secs);
                    }
            }
            FIELD_MAX_RESULTS => {
                let trimmed = self.edit_buffer.trim();
                if trimmed.is_empty() {
                    config.settings.default_max_results = None;
                } else if let Ok(max) = trimmed.parse::<u32>() {
                    config.settings.default_max_results = Some(max);
                }
            }
            FIELD_DEFAULT_LOCATION => {
                let trimmed = self.edit_buffer.trim().to_string();
                if trimmed.is_empty() {
                    config.settings.default_location = None;
                } else {
                    config.settings.default_location = Some(trimmed);
                }
            }
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig) {
        let block = Block::default()
            .title(Span::styled(
                " Settings ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let has_banner = self.update_banner.is_some();
        let banner_height = if has_banner { 3 } else { 0 };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4 + FIELD_COUNT as u16 + 2),
                Constraint::Length(banner_height),
                Constraint::Min(0),
            ])
            .split(inner);

        self.render_defaults_section(frame, chunks[0], theme, config);
        if let Some(ref banner) = self.update_banner {
            self.render_update_banner(frame, chunks[1], theme, banner);
        }
    }

    fn render_defaults_section(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        config: &AppConfig,
    ) {
        let interval_val = config.settings.default_check_interval.as_secs().to_string();
        let max_val = config
            .settings
            .default_max_results
            .map(|m| m.to_string())
            .unwrap_or_else(|| "unlimited".into());
        let notifier_val = config.settings.default_notifier.to_string();
        let updates_val = if config.settings.check_for_updates {
            "Enabled"
        } else {
            "Disabled"
        };
        let location_val = config
            .settings
            .default_location
            .clone()
            .unwrap_or_else(|| "not set".into());
        let log_level_val = config.settings.log_level.to_string();

        let fields = [
            ("Check interval (s)", interval_val),
            ("Max results", max_val),
            ("Notification", notifier_val),
            ("Check for updates", updates_val.to_string()),
            ("Default location", location_val),
            ("Log level", log_level_val),
        ];

        let mut lines = vec![
            Line::from(Span::styled(
                "Defaults",
                Style::default()
                    .fg(theme.fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        for (i, (label, value)) in fields.iter().enumerate() {
            let is_selected = i == self.selected;
            let is_editing = is_selected && self.editing;

            let cursor = if is_selected { "▸ " } else { "  " };

            let label_style = if is_selected {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.fg_dim)
            };

            let display_value = if is_editing {
                self.edit_buffer.clone()
            } else {
                value.clone()
            };

            let value_style = if is_editing {
                Style::default()
                    .fg(theme.fg)
                    .add_modifier(Modifier::UNDERLINED)
            } else if is_selected {
                Style::default().fg(theme.fg)
            } else {
                Style::default().fg(theme.fg_dim)
            };

            lines.push(Line::from(vec![
                Span::styled(cursor, Style::default().fg(theme.accent)),
                Span::styled(format!("{:<20}", label), label_style),
                Span::styled(display_value, value_style),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  [Enter] edit/toggle  [↑↓] navigate",
            Style::default().fg(theme.fg_dim),
        )));

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

    fn render_update_banner(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        banner: &str,
    ) {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {}", banner),
                Style::default().fg(theme.unread),
            )),
        ];

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }
}

pub enum SettingsAction {
    ConfigChanged,
}
