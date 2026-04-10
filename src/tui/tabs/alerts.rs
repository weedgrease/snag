use crate::config::AppConfig;
use crate::tui::theme::Theme;
use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
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

        // Pre-bind all temporary strings so they live long enough for `lines`.
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

        let mut lines = vec![
            Line::from(Span::styled(
                &alert.name,
                Style::default()
                    .fg(theme.fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            detail_line("Marketplaces", &marketplaces_joined, theme),
            detail_line("Keywords", &keywords_joined, theme),
        ];

        if !alert.exclude_keywords.is_empty() {
            lines.push(detail_line("Exclude", &exclude_joined, theme));
        }

        if let Some(ref price) = price_str {
            lines.push(detail_line("Price", price, theme));
        }

        if let Some(ref ls) = loc_str {
            lines.push(detail_line("Location", ls, theme));
        }

        if let Some(ref cs) = cond_str {
            lines.push(detail_line("Condition", cs, theme));
        }

        if let Some(ref cat) = alert.category {
            lines.push(detail_line("Category", cat, theme));
        }

        lines.push(detail_line("Interval", &interval_str, theme));
        lines.push(detail_line("Notify", &notifiers_joined, theme));

        if let Some(ref ms) = max_str {
            lines.push(detail_line("Max results", ms, theme));
        }

        let status = if alert.enabled { "Enabled" } else { "Disabled" };
        let status_color = if alert.enabled {
            theme.enabled
        } else {
            theme.disabled
        };
        lines.push(Line::from(vec![
            Span::styled("Status    ", Style::default().fg(theme.fg_dim)),
            Span::styled(status, Style::default().fg(status_color)),
        ]));

        if let Some(check_status) = statuses.iter().find(|s| s.alert_id == alert.id) {
            lines.push(Line::from(""));

            let ago = Utc::now().signed_duration_since(check_status.checked_at);
            let ago_str = if ago.num_hours() > 0 {
                format!("{}h ago", ago.num_hours())
            } else if ago.num_minutes() > 0 {
                format!("{}m ago", ago.num_minutes())
            } else {
                format!("{}s ago", ago.num_seconds())
            };

            if let Some(ref err) = check_status.error {
                lines.push(Line::from(vec![
                    Span::styled("Last check  ", Style::default().fg(theme.fg_dim)),
                    Span::styled(ago_str, Style::default().fg(theme.fg)),
                    Span::styled(format!(" — error: {}", err), Style::default().fg(theme.disabled)),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Last check  ", Style::default().fg(theme.fg_dim)),
                    Span::styled(ago_str, Style::default().fg(theme.fg)),
                    Span::styled(
                        format!(" — {} new results", check_status.new_results),
                        Style::default().fg(theme.accent),
                    ),
                ]));
            }
        }

        let detail = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(detail, inner);
    }
}

fn detail_line<'a>(label: &'a str, value: &'a str, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{:<10}", label),
            Style::default().fg(theme.fg_dim),
        ),
        Span::styled(value, Style::default().fg(theme.fg)),
    ])
}

pub enum AlertsAction {
    ConfigChanged,
    CreateAlert,
    EditAlert(usize),
    DeleteAlert(usize),
}
