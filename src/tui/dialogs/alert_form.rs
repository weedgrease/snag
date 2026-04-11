use super::DialogResult;
use crate::tui::theme::Theme;
use crate::types::{Alert, Condition, MarketplaceKind, NotifierKind};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use std::time::Duration;
use uuid::Uuid;

pub struct AlertFormDialog {
    pub fields: Vec<FormField>,
    pub selected_field: usize,
    pub editing: bool,
    pub existing_id: Option<Uuid>,
    pub default_location: Option<String>,
    pub original_enabled: bool,
    pub mp_facebook: bool,
    pub mp_ebay: bool,
    pub mp_selecting: bool,
    pub mp_cursor: usize,
}

pub struct FormField {
    pub label: String,
    pub value: String,
    pub cursor: usize,
}

impl FormField {
    fn new(label: &str, value: &str) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            cursor: value.len(),
        }
    }

    fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    fn delete_char(&mut self) {
        if self.cursor > 0 {
            let prev = self.value[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.value.remove(prev);
            self.cursor = prev;
        }
    }
}

impl Default for AlertFormDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl AlertFormDialog {
    pub fn new() -> Self {
        Self {
            fields: vec![
                FormField::new("Name", ""),
                FormField::new("Marketplaces", ""),
                FormField::new("Keywords", ""),
                FormField::new("Exclude keywords", ""),
                FormField::new("Price min", ""),
                FormField::new("Price max", ""),
                FormField::new("Location", ""),
                FormField::new("Radius (miles)", ""),
                FormField::new("Condition", ""),
                FormField::new("Category", ""),
                FormField::new("Interval (seconds)", "300"),
                FormField::new("Max results", "20"),
            ],
            selected_field: 0,
            editing: false,
            existing_id: None,
            default_location: None,
            original_enabled: true,
            mp_facebook: true,
            mp_ebay: false,
            mp_selecting: false,
            mp_cursor: 0,
        }
    }

    pub fn from_alert(alert: &Alert) -> Self {
        let has_fb = alert
            .marketplaces
            .contains(&MarketplaceKind::FacebookMarketplace);
        let has_ebay = alert.marketplaces.contains(&MarketplaceKind::Ebay);

        Self {
            fields: vec![
                FormField::new("Name", &alert.name),
                FormField::new("Marketplaces", ""),
                FormField::new("Keywords", &alert.keywords.join(", ")),
                FormField::new("Exclude keywords", &alert.exclude_keywords.join(", ")),
                FormField::new(
                    "Price min",
                    &alert.price_min.map(|p| p.to_string()).unwrap_or_default(),
                ),
                FormField::new(
                    "Price max",
                    &alert.price_max.map(|p| p.to_string()).unwrap_or_default(),
                ),
                FormField::new("Location", alert.location.as_deref().unwrap_or("")),
                FormField::new(
                    "Radius (miles)",
                    &alert
                        .radius_miles
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                ),
                FormField::new(
                    "Condition",
                    alert
                        .condition
                        .map(|c| match c {
                            Condition::New => "new",
                            Condition::LikeNew => "like new",
                            Condition::Used => "used",
                            Condition::ForParts => "for parts",
                        })
                        .unwrap_or(""),
                ),
                FormField::new("Category", alert.category.as_deref().unwrap_or("")),
                FormField::new(
                    "Interval (seconds)",
                    &alert.check_interval.as_secs().to_string(),
                ),
                FormField::new(
                    "Max results",
                    &alert.max_results.map(|m| m.to_string()).unwrap_or_default(),
                ),
            ],
            selected_field: 0,
            editing: false,
            existing_id: Some(alert.id),
            default_location: None,
            original_enabled: alert.enabled,
            mp_facebook: has_fb,
            mp_ebay: has_ebay,
            mp_selecting: false,
            mp_cursor: 0,
        }
    }

    pub fn set_default_location(&mut self, loc: Option<String>) {
        self.default_location = loc;
    }

    pub fn set_config_defaults(&mut self, config: &crate::config::AppConfig) {
        self.fields[10].value = config.settings.default_check_interval.as_secs().to_string();
        self.fields[10].cursor = self.fields[10].value.len();
        if let Some(max) = config.settings.default_max_results {
            self.fields[11].value = max.to_string();
            self.fields[11].cursor = self.fields[11].value.len();
        }
    }

    fn marketplace_summary(&self) -> String {
        let mut parts = vec![];
        if self.mp_facebook {
            parts.push("Facebook");
        }
        if self.mp_ebay {
            parts.push("eBay");
        }
        if parts.is_empty() {
            "—".to_string()
        } else {
            parts.join(", ")
        }
    }

    fn cycle_condition(&mut self) {
        let current = &self.fields[8].value;
        let next = match current.trim().to_lowercase().as_str() {
            "" => "new",
            "new" => "like new",
            "like new" => "used",
            "used" => "for parts",
            "for parts" => "",
            _ => "",
        };
        self.fields[8].value = next.to_string();
        self.fields[8].cursor = self.fields[8].value.len();
    }

    pub fn to_alert(&self) -> Option<Alert> {
        let name = self.fields[0].value.trim().to_string();
        if name.is_empty() {
            return None;
        }

        let mut marketplaces = vec![];
        if self.mp_facebook {
            marketplaces.push(MarketplaceKind::FacebookMarketplace);
        }
        if self.mp_ebay {
            marketplaces.push(MarketplaceKind::Ebay);
        }

        if marketplaces.is_empty() {
            return None;
        }

        let keywords: Vec<String> = self.fields[2]
            .value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if keywords.is_empty() {
            return None;
        }

        let exclude_keywords: Vec<String> = self.fields[3]
            .value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let price_min = self.fields[4].value.trim().parse::<f64>().ok();
        let price_max = self.fields[5].value.trim().parse::<f64>().ok();

        let location = {
            let v = self.fields[6].value.trim().to_string();
            if v.is_empty() { None } else { Some(v) }
        };

        let has_facebook = marketplaces.contains(&MarketplaceKind::FacebookMarketplace);
        if has_facebook && location.is_none() && self.default_location.is_none() {
            return None;
        }

        let radius_miles = self.fields[7].value.trim().parse::<u32>().ok();

        let condition = match self.fields[8].value.trim().to_lowercase().as_str() {
            "new" => Some(Condition::New),
            "like new" => Some(Condition::LikeNew),
            "used" => Some(Condition::Used),
            "for parts" => Some(Condition::ForParts),
            _ => None,
        };

        let category = {
            let v = self.fields[9].value.trim().to_string();
            if v.is_empty() { None } else { Some(v) }
        };

        let interval_secs = self.fields[10].value.trim().parse::<u64>().unwrap_or(300);

        let max_results = self.fields[11].value.trim().parse::<u32>().ok();

        Some(Alert {
            id: self.existing_id.unwrap_or_else(Uuid::new_v4),
            name,
            marketplaces,
            keywords,
            exclude_keywords,
            price_min,
            price_max,
            location,
            radius_miles,
            condition,
            category,
            check_interval: Duration::from_secs(interval_secs),
            notifiers: vec![NotifierKind::Terminal],
            max_results,
            enabled: self.original_enabled,
        })
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DialogResult<Alert> {
        if self.mp_selecting {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.mp_cursor > 0 {
                        self.mp_cursor -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.mp_cursor < 1 {
                        self.mp_cursor += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => match self.mp_cursor {
                    0 => self.mp_facebook = !self.mp_facebook,
                    1 => self.mp_ebay = !self.mp_ebay,
                    _ => {}
                },
                KeyCode::Esc => {
                    self.mp_selecting = false;
                }
                _ => {}
            }
            return DialogResult::Continue;
        }

        if self.editing {
            match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                }
                KeyCode::Enter => {
                    self.editing = false;
                }
                KeyCode::Backspace => {
                    self.fields[self.selected_field].delete_char();
                }
                KeyCode::Char(c) => {
                    self.fields[self.selected_field].insert_char(c);
                }
                _ => {}
            }
            return DialogResult::Continue;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => DialogResult::Cancel,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_field > 0 {
                    self.selected_field -= 1;
                }
                DialogResult::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_field < self.fields.len() - 1 {
                    self.selected_field += 1;
                }
                DialogResult::Continue
            }
            KeyCode::Enter => {
                match self.selected_field {
                    1 => {
                        self.mp_selecting = true;
                        self.mp_cursor = 0;
                        DialogResult::Continue
                    }
                    8 => {
                        self.cycle_condition();
                        DialogResult::Continue
                    }
                    _ => {
                        self.editing = true;
                        let field = &mut self.fields[self.selected_field];
                        field.cursor = field.value.len();
                        DialogResult::Continue
                    }
                }
            }
            KeyCode::Char('s') => match self.to_alert() {
                Some(alert) => DialogResult::Submit(alert),
                None => DialogResult::Continue,
            },
            _ => DialogResult::Continue,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog_width = 60u16.min(area.width.saturating_sub(4));
        let mp_extra: u16 = if self.mp_selecting { 2 } else { 0 };
        let dialog_height =
            (self.fields.len() as u16 + 6 + mp_extra).min(area.height.saturating_sub(4));

        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        frame.render_widget(Clear, dialog_area);
        frame.render_widget(
            Block::default().style(ratatui::style::Style::default().bg(theme.bg)),
            dialog_area,
        );

        let title = if self.existing_id.is_some() {
            " Edit Alert "
        } else {
            " New Alert "
        };

        let block = Block::default()
            .title(Span::styled(
                title,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .border_type(BorderType::Rounded);

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        let mp_extra = if self.mp_selecting { 2 } else { 0 };
        let mut constraints: Vec<Constraint> = Vec::new();
        for (i, _) in self.fields.iter().enumerate() {
            constraints.push(Constraint::Length(1));
            if i == 1 {
                constraints.push(Constraint::Length(mp_extra));
            }
        }
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Min(0));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        let mut chunk_idx = 0;
        for (i, field) in self.fields.iter().enumerate() {
            let is_selected = i == self.selected_field;
            let is_editing = is_selected && self.editing;

            let label_style = if is_selected {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.fg_dim)
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

            let cursor = if is_selected && !self.mp_selecting { "▸ " } else { "  " };

            if i == 1 {
                let mp_summary = self.marketplace_summary();
                let line = Line::from(vec![
                    Span::styled(cursor, Style::default().fg(theme.accent)),
                    Span::styled(format!("{:<20}", field.label), label_style),
                    Span::styled(mp_summary, value_style),
                ]);
                frame.render_widget(Paragraph::new(line), chunks[chunk_idx]);
                chunk_idx += 1;

                if self.mp_selecting {
                    let fb_check = if self.mp_facebook { "✓" } else { " " };
                    let ebay_check = if self.mp_ebay { "✓" } else { " " };
                    let fb_cursor = if self.mp_cursor == 0 { "▸ " } else { "  " };
                    let ebay_cursor = if self.mp_cursor == 1 { "▸ " } else { "  " };
                    let fb_style = if self.mp_cursor == 0 {
                        Style::default().fg(theme.fg)
                    } else {
                        Style::default().fg(theme.fg_dim)
                    };
                    let ebay_style = if self.mp_cursor == 1 {
                        Style::default().fg(theme.fg)
                    } else {
                        Style::default().fg(theme.fg_dim)
                    };

                    let fb_line = Line::from(vec![
                        Span::styled(fb_cursor, Style::default().fg(theme.accent)),
                        Span::styled(format!("[{}] ", fb_check), Style::default().fg(theme.enabled)),
                        Span::styled("Facebook Marketplace", fb_style),
                    ]);
                    frame.render_widget(Paragraph::new(fb_line), chunks[chunk_idx]);
                    chunk_idx += 1;

                    let ebay_line = Line::from(vec![
                        Span::styled(ebay_cursor, Style::default().fg(theme.accent)),
                        Span::styled(format!("[{}] ", ebay_check), Style::default().fg(theme.enabled)),
                        Span::styled("eBay", ebay_style),
                    ]);
                    frame.render_widget(Paragraph::new(ebay_line), chunks[chunk_idx]);
                    chunk_idx += 1;
                }
                continue;
            }

            let is_condition = i == 8;
            let display_value = if field.value.is_empty() && !is_editing {
                "—".to_string()
            } else {
                field.value.clone()
            };

            let mut spans = vec![
                Span::styled(cursor, Style::default().fg(theme.accent)),
                Span::styled(format!("{:<20}", field.label), label_style),
            ];

            if is_condition && is_selected {
                spans.push(Span::styled("◀ ", Style::default().fg(theme.fg_dim)));
                spans.push(Span::styled(display_value, value_style));
                spans.push(Span::styled(" ▶", Style::default().fg(theme.fg_dim)));
            } else {
                spans.push(Span::styled(display_value, value_style));
            }

            let line = Line::from(spans);

            frame.render_widget(Paragraph::new(line), chunks[chunk_idx]);
            chunk_idx += 1;
        }

        let help_line = Line::from(vec![Span::styled(
            " [Enter] edit/cycle  [s] save  [Esc] cancel",
            Style::default().fg(theme.fg_dim),
        )]);
        if chunk_idx < chunks.len() {
            frame.render_widget(Paragraph::new(help_line), chunks[chunk_idx]);
        }
    }
}
