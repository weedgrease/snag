use crate::config::AppConfig;
use crate::tui::theme::Theme;
use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table};
use ratatui::Frame;

pub struct AlertsTab {
    pub selected: usize,
    pub list_state: ListState,
}

impl Default for AlertsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl AlertsTab {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            selected: 0,
            list_state,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, config: &mut AppConfig) -> Option<AlertsAction> {
        let alert_count = config.alerts.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if alert_count > 0 && self.selected > 0 {
                    self.selected -= 1;
                    self.list_state.select(Some(self.selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if alert_count > 0 && self.selected < alert_count - 1 {
                    self.selected += 1;
                    self.list_state.select(Some(self.selected));
                }
            }
            KeyCode::Char(' ') => {
                if let Some(alert) = config.alerts.get_mut(self.selected) {
                    alert.enabled = !alert.enabled;
                    return Some(AlertsAction::ConfigChanged);
                }
            }
            KeyCode::Char('n') => {
                return Some(AlertsAction::CreateAlert);
            }
            KeyCode::Char('e') => {
                if self.selected < alert_count {
                    return Some(AlertsAction::EditAlert(self.selected));
                }
            }
            KeyCode::Char('d') => {
                if self.selected < alert_count {
                    return Some(AlertsAction::DeleteAlert(self.selected));
                }
            }
            KeyCode::Char('f') => {
                if self.selected < alert_count {
                    return Some(AlertsAction::ForceCheck(self.selected));
                }
            }
            _ => {}
        }

        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig, statuses: &[crate::types::CheckStatus]) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.render_list(frame, chunks[0], theme, config);
        self.render_detail(frame, chunks[1], theme, config, statuses);
    }

    fn render_list(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig) {
        let items: Vec<ListItem> = config
            .alerts
            .iter()
            .enumerate()
            .map(|(i, alert)| {
                let indicator = if alert.enabled { "●" } else { "○" };
                let color = if alert.enabled {
                    theme.enabled
                } else {
                    theme.disabled
                };

                let style = if i == self.selected {
                    Style::default().bg(theme.selected_bg).fg(theme.fg)
                } else {
                    Style::default().fg(theme.fg)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", indicator), Style::default().fg(color)),
                    Span::styled(&alert.name, style),
                ]))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(Span::styled(
                    " Alerts ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );

        let mut state = self.list_state;
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig, statuses: &[crate::types::CheckStatus]) {
        let block = Block::default()
            .title(Span::styled(
                " Details ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let alert = match config.alerts.get(self.selected) {
            Some(a) => a,
            None => {
                let empty = Paragraph::new("No alerts configured. Press 'n' to create one.")
                    .style(Style::default().fg(theme.fg_dim));
                frame.render_widget(empty, inner);
                return;
            }
        };

        // Pre-bind all temporary strings so they live long enough.
        let marketplaces_str: Vec<String> = alert.marketplaces.iter().map(|m| m.to_string()).collect();
        let marketplaces_joined = marketplaces_str.join(", ");
        let keywords_joined = alert.keywords.join(", ");
        let exclude_joined = alert.exclude_keywords.join(", ");
        let price_str = if alert.price_min.is_some() || alert.price_max.is_some() {
            Some(format!(
                "${} — ${}",
                alert.price_min.map(|p| format!("{:.0}", p)).unwrap_or_else(|| "any".into()),
                alert.price_max.map(|p| format!("{:.0}", p)).unwrap_or_else(|| "any".into()),
            ))
        } else {
            None
        };
        let loc_str = alert.location.as_ref().map(|loc| {
            if let Some(r) = alert.radius_miles {
                format!("{}, {}mi", loc, r)
            } else {
                loc.clone()
            }
        });
        let cond_str = alert.condition.as_ref().map(|c| c.to_string());
        let interval_secs = alert.check_interval.as_secs();
        let interval_str = if interval_secs >= 3600 {
            format!("{}h", interval_secs / 3600)
        } else if interval_secs >= 60 {
            format!("{}m", interval_secs / 60)
        } else {
            format!("{}s", interval_secs)
        };
        let notifiers_strs: Vec<String> = alert.notifiers.iter().map(|n| n.to_string()).collect();
        let notifiers_joined = notifiers_strs.join(", ");
        let max_str = alert.max_results.map(|m| m.to_string());

        let status = if alert.enabled { "Enabled" } else { "Disabled" };
        let status_color = if alert.enabled { theme.enabled } else { theme.disabled };

        // Split inner into: name (2 lines), table (flexible), status section (3 lines).
        let check_status = statuses.iter().find(|s| s.alert_id == alert.id);
        let status_height = if check_status.is_some() { 3 } else { 1 };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(status_height),
            ])
            .split(inner);

        // Name header.
        let name_para = Paragraph::new(Span::styled(
            &alert.name,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(name_para, chunks[0]);

        // Detail rows as a Table.
        let dim = Style::default().fg(theme.fg_dim);
        let fg = Style::default().fg(theme.fg);

        let mut rows: Vec<Row> = vec![
            Row::new(vec![
                Cell::from("Marketplaces").style(dim),
                Cell::from(marketplaces_joined.as_str()).style(fg),
            ]),
            Row::new(vec![
                Cell::from("Keywords").style(dim),
                Cell::from(keywords_joined.as_str()).style(fg),
            ]),
        ];

        if !alert.exclude_keywords.is_empty() {
            rows.push(Row::new(vec![
                Cell::from("Exclude").style(dim),
                Cell::from(exclude_joined.as_str()).style(fg),
            ]));
        }

        if let Some(ref price) = price_str {
            rows.push(Row::new(vec![
                Cell::from("Price").style(dim),
                Cell::from(price.as_str()).style(fg),
            ]));
        }

        if let Some(ref ls) = loc_str {
            rows.push(Row::new(vec![
                Cell::from("Location").style(dim),
                Cell::from(ls.as_str()).style(fg),
            ]));
        }

        if let Some(ref cs) = cond_str {
            rows.push(Row::new(vec![
                Cell::from("Condition").style(dim),
                Cell::from(cs.as_str()).style(fg),
            ]));
        }

        if let Some(ref cat) = alert.category {
            rows.push(Row::new(vec![
                Cell::from("Category").style(dim),
                Cell::from(cat.as_str()).style(fg),
            ]));
        }

        rows.push(Row::new(vec![
            Cell::from("Interval").style(dim),
            Cell::from(interval_str.as_str()).style(fg),
        ]));
        rows.push(Row::new(vec![
            Cell::from("Notify").style(dim),
            Cell::from(notifiers_joined.as_str()).style(fg),
        ]));

        if let Some(ref ms) = max_str {
            rows.push(Row::new(vec![
                Cell::from("Max results").style(dim),
                Cell::from(ms.as_str()).style(fg),
            ]));
        }

        rows.push(Row::new(vec![
            Cell::from("Status").style(dim),
            Cell::from(status).style(Style::default().fg(status_color)),
        ]));

        let widths = [Constraint::Length(16), Constraint::Min(10)];
        let table = Table::new(rows, widths);
        frame.render_widget(table, chunks[1]);

        // Check status section.
        if let Some(check_status) = check_status {
            let ago = Utc::now().signed_duration_since(check_status.checked_at);
            let ago_str = if ago.num_hours() > 0 {
                format!("{}h ago", ago.num_hours())
            } else if ago.num_minutes() > 0 {
                format!("{}m ago", ago.num_minutes())
            } else {
                format!("{}s ago", ago.num_seconds())
            };

            let last_check_line = if let Some(ref err) = check_status.error {
                Line::from(vec![
                    Span::styled(ago_str, Style::default().fg(theme.fg)),
                    Span::styled(format!(" — error: {}", err), Style::default().fg(theme.disabled)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(ago_str, Style::default().fg(theme.fg)),
                    Span::styled(
                        format!(" — {} new results", check_status.new_results),
                        Style::default().fg(theme.accent),
                    ),
                ])
            };

            let status_rows = vec![
                Row::new(vec![
                    Cell::from("Last check").style(dim),
                    Cell::from(last_check_line),
                ]),
            ];
            let status_table = Table::new(status_rows, widths);
            frame.render_widget(status_table, chunks[2]);
        }
    }
}

pub enum AlertsAction {
    ConfigChanged,
    CreateAlert,
    EditAlert(usize),
    DeleteAlert(usize),
    ForceCheck(usize),
}
