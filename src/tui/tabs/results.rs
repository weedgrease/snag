use crate::tui::theme::Theme;
use crate::types::AlertResult;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

pub struct ResultsTab {
    pub selected: usize,
    pub list_state: ListState,
}

struct FlatListing {
    pub alert_name: String,
    pub result_idx: usize,
    pub listing_idx: usize,
}

impl ResultsTab {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            selected: 0,
            list_state,
        }
    }

    fn flatten(results: &[AlertResult]) -> Vec<FlatListing> {
        let mut flat = vec![];
        for (ri, result) in results.iter().enumerate().rev() {
            for (li, _listing) in result.listings.iter().enumerate() {
                flat.push(FlatListing {
                    alert_name: result.alert_name.clone(),
                    result_idx: ri,
                    listing_idx: li,
                });
            }
        }
        flat
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        results: &mut Vec<AlertResult>,
    ) -> Option<ResultsAction> {
        let flat = Self::flatten(results);
        let count = flat.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if count > 0 && self.selected > 0 {
                    self.selected -= 1;
                    self.list_state.select(Some(self.selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if count > 0 && self.selected < count - 1 {
                    self.selected += 1;
                    self.list_state.select(Some(self.selected));
                }
            }
            KeyCode::Char('o') => {
                if let Some(entry) = flat.get(self.selected) {
                    let url = results[entry.result_idx].listings[entry.listing_idx]
                        .url
                        .clone();
                    results[entry.result_idx].seen = true;
                    return Some(ResultsAction::OpenUrl(url));
                }
            }
            KeyCode::Char('m') => {
                if let Some(entry) = flat.get(self.selected) {
                    results[entry.result_idx].seen = true;
                }
            }
            KeyCode::Char('c') => {
                results.clear();
                self.selected = 0;
                self.list_state.select(Some(0));
                return Some(ResultsAction::ResultsChanged);
            }
            _ => {}
        }

        None
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        results: &[AlertResult],
    ) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        let flat = Self::flatten(results);
        self.render_list(frame, chunks[0], theme, results, &flat);
        self.render_detail(frame, chunks[1], theme, results, &flat);
    }

    fn render_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        results: &[AlertResult],
        flat: &[FlatListing],
    ) {
        let items: Vec<ListItem> = flat
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let listing = &results[entry.result_idx].listings[entry.listing_idx];
                let seen = results[entry.result_idx].seen;

                let indicator = if seen { "  " } else { "● " };
                let indicator_color = if seen { theme.fg_dim } else { theme.unread };

                let title = if listing.title.len() > 25 {
                    format!("{}…", &listing.title[..24])
                } else {
                    listing.title.clone()
                };

                let style = if i == self.selected {
                    Style::default().bg(theme.selected_bg).fg(theme.fg)
                } else {
                    Style::default().fg(theme.fg)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(indicator, Style::default().fg(indicator_color)),
                    Span::styled(title, style),
                ]))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(Span::styled(
                    " Results ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );

        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_detail(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        results: &[AlertResult],
        flat: &[FlatListing],
    ) {
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

        let entry = match flat.get(self.selected) {
            Some(e) => e,
            None => {
                let empty = Paragraph::new("No results yet.")
                    .style(Style::default().fg(theme.fg_dim));
                frame.render_widget(empty, inner);
                return;
            }
        };

        let listing = &results[entry.result_idx].listings[entry.listing_idx];

        let mut lines = vec![];
        lines.push(Line::from(Span::styled(
            &listing.title,
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        let price_str = listing.price.map(|p| format!("{}{:.2}", listing.currency, p));
        let marketplace_str = listing.marketplace.to_string();
        let cond_str = listing.condition.as_ref().map(|c| c.to_string());
        let posted_str = listing.posted_at.as_ref().map(|p| p.format("%Y-%m-%d %H:%M").to_string());
        let found_str = listing.found_at.format("%Y-%m-%d %H:%M").to_string();

        if let Some(ref s) = price_str {
            lines.push(detail_line("Price", s, theme));
        }

        lines.push(detail_line("Marketplace", &marketplace_str, theme));

        if let Some(ref loc) = listing.location {
            lines.push(detail_line("Location", loc, theme));
        }

        if let Some(ref s) = cond_str {
            lines.push(detail_line("Condition", s, theme));
        }

        if let Some(ref s) = posted_str {
            lines.push(detail_line("Posted", s, theme));
        }

        lines.push(detail_line("Found", &found_str, theme));
        lines.push(Line::from(""));
        lines.push(detail_line("Alert", &entry.alert_name, theme));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "[o] open in browser",
            Style::default().fg(theme.accent),
        )));

        let detail = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(detail, inner);
    }
}

fn detail_line<'a>(label: &'a str, value: &'a str, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{:<12}", label),
            Style::default().fg(theme.fg_dim),
        ),
        Span::styled(value, Style::default().fg(theme.fg)),
    ])
}

pub enum ResultsAction {
    OpenUrl(String),
    ResultsChanged,
}
