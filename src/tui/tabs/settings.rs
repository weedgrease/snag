use crate::config::AppConfig;
use crate::tui::theme::Theme;
use crate::types::NotifierKind;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::time::Duration;

const FIELD_CHECK_INTERVAL: usize = 0;
const FIELD_MAX_RESULTS: usize = 1;
const FIELD_NOTIFICATION: usize = 2;
const FIELD_CHECK_UPDATES: usize = 3;
const FIELD_DEFAULT_LOCATION: usize = 4;
const DEFAULTS_COUNT: usize = 5;

const _MP_FACEBOOK: usize = 0;
const MP_EBAY: usize = 1;
const MARKETPLACE_COUNT: usize = 2;

const TOTAL_ITEMS: usize = DEFAULTS_COUNT + MARKETPLACE_COUNT;

pub struct SettingsTab {
    pub selected: usize,
    pub editing: bool,
    pub edit_buffer: String,
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
        }
    }

    fn is_defaults_field(&self) -> bool {
        self.selected < DEFAULTS_COUNT
    }

    fn marketplace_index(&self) -> Option<usize> {
        if self.selected >= DEFAULTS_COUNT {
            Some(self.selected - DEFAULTS_COUNT)
        } else {
            None
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
                if self.selected < TOTAL_ITEMS - 1 {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => {
                if self.is_defaults_field() {
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
                        _ => {
                            self.editing = true;
                            self.edit_buffer = self.current_field_value(config);
                        }
                    }
                } else if let Some(mp_idx) = self.marketplace_index()
                    && mp_idx == MP_EBAY
                {
                    return Some(SettingsAction::SetupMarketplace(MarketplaceSetup::Ebay));
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
                    && secs > 0
                {
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
            .border_style(Style::default().fg(theme.border))
            .border_type(BorderType::Rounded);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let defaults_height = 4 + DEFAULTS_COUNT as u16;
        let marketplaces_height = 4 + MARKETPLACE_COUNT as u16;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(defaults_height),
                Constraint::Length(marketplaces_height),
                Constraint::Min(0),
            ])
            .split(inner);

        self.render_defaults_section(frame, chunks[0], theme, config);
        self.render_marketplaces_section(frame, chunks[1], theme);
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

        let fields = [
            ("Check interval (s)", interval_val),
            ("Max results", max_val),
            ("Notification", notifier_val),
            ("Check for updates", updates_val.to_string()),
            ("Default location", location_val),
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

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

    fn render_marketplaces_section(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let fb_status = "Ready";
        let fb_status_color = theme.enabled;

        let ebay_configured = crate::credentials::ebay_credentials_configured();
        let ebay_status = if ebay_configured { "Ready" } else { "Not configured" };
        let ebay_status_color = if ebay_configured { theme.enabled } else { theme.disabled };

        let marketplaces = [
            ("Facebook Marketplace", fb_status, fb_status_color),
            ("eBay", ebay_status, ebay_status_color),
        ];

        let mut lines = vec![
            Line::from(Span::styled(
                "Marketplaces",
                Style::default()
                    .fg(theme.fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        for (i, (name, status, status_color)) in marketplaces.iter().enumerate() {
            let global_idx = DEFAULTS_COUNT + i;
            let is_selected = global_idx == self.selected;

            let cursor = if is_selected { "▸ " } else { "  " };

            let name_style = if is_selected {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.fg)
            };

            lines.push(Line::from(vec![
                Span::styled(cursor, Style::default().fg(theme.accent)),
                Span::styled(format!("{:<24}", name), name_style),
                Span::styled(*status, Style::default().fg(*status_color)),
            ]));
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

}

pub enum SettingsAction {
    ConfigChanged,
    SetupMarketplace(MarketplaceSetup),
}

pub enum MarketplaceSetup {
    Ebay,
}
