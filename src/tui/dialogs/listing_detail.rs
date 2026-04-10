use super::DialogResult;
use crate::tui::theme::Theme;
use crate::types::Listing;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};
use ratatui::Frame;

pub struct ListingDetailDialog {
    pub listing: Listing,
    pub alert_name: String,
}

impl ListingDetailDialog {
    pub fn new(listing: Listing, alert_name: String) -> Self {
        Self {
            listing,
            alert_name,
        }
    }

    pub fn handle_key(&self, key: KeyEvent) -> DialogResult<ListingDetailAction> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => DialogResult::Cancel,
            KeyCode::Char('o') => DialogResult::Submit(ListingDetailAction::OpenUrl(self.listing.url.clone())),
            _ => DialogResult::Continue,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog_width = 70u16.min(area.width.saturating_sub(4));
        let dialog_height = 28u16.min(area.height.saturating_sub(4));

        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(Span::styled(
                " Listing Details ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        let has_description = self.listing.description.is_some();
        let desc_height = if has_description { 5u16 } else { 0 };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(desc_height),
                Constraint::Length(2),
                Constraint::Length(1),
            ])
            .split(inner);

        let title = Paragraph::new(Span::styled(
            &self.listing.title,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ))
        .wrap(Wrap { trim: false });
        frame.render_widget(title, chunks[0]);

        let dim = Style::default().fg(theme.fg_dim);
        let fg = Style::default().fg(theme.fg);

        let mut rows: Vec<Row> = vec![];

        if let Some(price) = self.listing.price {
            let price_str = format!("${:.2}", price);
            rows.push(Row::new(vec![
                Cell::from("Price").style(dim),
                Cell::from(price_str).style(Style::default().fg(theme.accent)),
            ]));
        }

        rows.push(Row::new(vec![
            Cell::from("Marketplace").style(dim),
            Cell::from(self.listing.marketplace.to_string()).style(fg),
        ]));

        if let Some(ref loc) = self.listing.location {
            rows.push(Row::new(vec![
                Cell::from("Location").style(dim),
                Cell::from(loc.as_str()).style(fg),
            ]));
        }

        if let Some(ref cond) = self.listing.condition {
            rows.push(Row::new(vec![
                Cell::from("Condition").style(dim),
                Cell::from(cond.to_string()).style(fg),
            ]));
        }

        if let Some(ref posted) = self.listing.posted_at {
            rows.push(Row::new(vec![
                Cell::from("Posted").style(dim),
                Cell::from(posted.format("%Y-%m-%d %H:%M").to_string()).style(fg),
            ]));
        }

        rows.push(Row::new(vec![
            Cell::from("Found").style(dim),
            Cell::from(self.listing.found_at.format("%Y-%m-%d %H:%M").to_string()).style(fg),
        ]));

        rows.push(Row::new(vec![
            Cell::from("Alert").style(dim),
            Cell::from(self.alert_name.as_str()).style(fg),
        ]));

        let widths = [Constraint::Length(16), Constraint::Min(10)];
        let table = Table::new(rows, widths);
        frame.render_widget(table, chunks[1]);

        if let Some(ref desc) = self.listing.description {
            let desc_block = Block::default()
                .title(Span::styled(" Description ", Style::default().fg(theme.fg_dim)))
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme.border));
            let desc_inner = desc_block.inner(chunks[2]);
            frame.render_widget(desc_block, chunks[2]);
            let desc_para = Paragraph::new(desc.as_str())
                .style(Style::default().fg(theme.fg))
                .wrap(Wrap { trim: false });
            frame.render_widget(desc_para, desc_inner);
        }

        let url = Paragraph::new(Line::from(vec![
            Span::styled("URL  ", dim),
            Span::styled(&self.listing.url, Style::default().fg(theme.accent)),
        ]))
        .wrap(Wrap { trim: false });
        frame.render_widget(url, chunks[3]);

        let hint = Paragraph::new(Span::styled(
            "[o] open in browser  [Esc] close",
            Style::default().fg(theme.fg_dim),
        ));
        frame.render_widget(hint, chunks[4]);
    }
}

pub enum ListingDetailAction {
    OpenUrl(String),
}
