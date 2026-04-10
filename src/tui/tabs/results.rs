use crate::tui::theme::Theme;
use crate::types::AlertResult;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Wrap};
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

impl Default for ResultsTab {
    fn default() -> Self {
        Self::new()
    }
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
        seen_ids: &mut std::collections::HashSet<String>,
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
                    let listing = &results[entry.result_idx].listings[entry.listing_idx];
                    let url = listing.url.clone();
                    seen_ids.insert(listing.id.clone());
                    return Some(ResultsAction::OpenUrl(url));
                }
            }
            KeyCode::Char('m') => {
                if let Some(entry) = flat.get(self.selected) {
                    let listing = &results[entry.result_idx].listings[entry.listing_idx];
                    seen_ids.insert(listing.id.clone());
                    return Some(ResultsAction::SeenChanged);
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
        seen_ids: &std::collections::HashSet<String>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        let flat = Self::flatten(results);
        self.render_list(frame, chunks[0], theme, results, &flat, seen_ids);
        self.render_detail(frame, chunks[1], theme, results, &flat);
    }

    fn render_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        results: &[AlertResult],
        flat: &[FlatListing],
        seen_ids: &std::collections::HashSet<String>,
    ) {
        let items: Vec<ListItem> = flat
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let listing = &results[entry.result_idx].listings[entry.listing_idx];
                let is_seen = seen_ids.contains(&listing.id);

                let indicator = if is_seen { "  " } else { "● " };
                let indicator_color = if is_seen { theme.fg_dim } else { theme.unread };

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

        let mut state = self.list_state;
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

        let price_str = listing.price.map(|p| format!("{}{:.2}", listing.currency, p));
        let marketplace_str = listing.marketplace.to_string();
        let cond_str = listing.condition.as_ref().map(|c| c.to_string());
        let posted_str = listing.posted_at.as_ref().map(|p| p.format("%Y-%m-%d %H:%M").to_string());
        let found_str = listing.found_at.format("%Y-%m-%d %H:%M").to_string();

        // Split inner into: title (2 lines), table (flexible), hint (1 line).
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(inner);

        // Listing title header.
        let title_para = Paragraph::new(Span::styled(
            &listing.title,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ))
        .wrap(Wrap { trim: false });
        frame.render_widget(title_para, chunks[0]);

        // Detail rows as a Table.
        let dim = Style::default().fg(theme.fg_dim);
        let fg = Style::default().fg(theme.fg);

        let mut rows: Vec<Row> = vec![];

        if let Some(ref s) = price_str {
            rows.push(Row::new(vec![
                Cell::from("Price").style(dim),
                Cell::from(s.as_str()).style(fg),
            ]));
        }

        rows.push(Row::new(vec![
            Cell::from("Marketplace").style(dim),
            Cell::from(marketplace_str.as_str()).style(fg),
        ]));

        if let Some(ref loc) = listing.location {
            rows.push(Row::new(vec![
                Cell::from("Location").style(dim),
                Cell::from(loc.as_str()).style(fg),
            ]));
        }

        if let Some(ref s) = cond_str {
            rows.push(Row::new(vec![
                Cell::from("Condition").style(dim),
                Cell::from(s.as_str()).style(fg),
            ]));
        }

        if let Some(ref s) = posted_str {
            rows.push(Row::new(vec![
                Cell::from("Posted").style(dim),
                Cell::from(s.as_str()).style(fg),
            ]));
        }

        rows.push(Row::new(vec![
            Cell::from("Found").style(dim),
            Cell::from(found_str.as_str()).style(fg),
        ]));

        rows.push(Row::new(vec![
            Cell::from("Alert").style(dim),
            Cell::from(entry.alert_name.as_str()).style(fg),
        ]));

        let widths = [Constraint::Length(16), Constraint::Min(10)];
        let table = Table::new(rows, widths);
        frame.render_widget(table, chunks[1]);

        // Keyboard hint.
        let hint = Paragraph::new(Span::styled(
            "[o] open in browser",
            Style::default().fg(theme.accent),
        ));
        frame.render_widget(hint, chunks[2]);
    }
}

pub enum ResultsAction {
    OpenUrl(String),
    ResultsChanged,
    SeenChanged,
}
