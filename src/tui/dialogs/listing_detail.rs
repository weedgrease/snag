use super::DialogResult;
use crate::tui::theme::Theme;
use crate::types::Listing;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};
use ratatui_image::StatefulImage;
use ratatui_image::protocol::StatefulProtocol;

pub struct ListingDetailDialog {
    pub listing: Listing,
    pub alert_name: String,
    image_state: Option<StatefulProtocol>,
    image_loading: bool,
    image_rx: Option<tokio::sync::oneshot::Receiver<Option<image::DynamicImage>>>,
    description: Option<String>,
    description_loading: bool,
    description_rx: Option<tokio::sync::oneshot::Receiver<Option<String>>>,
    picker: Option<ratatui_image::picker::Picker>,
}

impl ListingDetailDialog {
    pub fn new(listing: Listing, alert_name: String) -> Self {
        let picker = ratatui_image::picker::Picker::from_query_stdio().ok();

        // Spawn image download
        let (img_tx, img_rx) = tokio::sync::oneshot::channel();
        let image_url = listing.image_url.clone();
        tokio::spawn(async move {
            let result = if let Some(url) = image_url {
                fetch_image(&url).await.ok()
            } else {
                None
            };
            let _ = img_tx.send(result);
        });

        // Spawn description fetch for eBay listings
        let (desc_tx, desc_rx) = tokio::sync::oneshot::channel();
        let listing_id = listing.id.clone();
        let marketplace = listing.marketplace;
        let description_loading = marketplace == crate::types::MarketplaceKind::Ebay;
        tokio::spawn(async move {
            let result = if marketplace == crate::types::MarketplaceKind::Ebay {
                crate::marketplace::providers::ebay::fetch_item_description(&listing_id)
                    .await
                    .ok()
                    .flatten()
            } else {
                None
            };
            let _ = desc_tx.send(result);
        });

        Self {
            listing,
            alert_name,
            image_state: None,
            image_loading: true,
            image_rx: Some(img_rx),
            description: None,
            description_loading,
            description_rx: Some(desc_rx),
            picker,
        }
    }

    pub fn handle_key(&self, key: KeyEvent) -> DialogResult<ListingDetailAction> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => DialogResult::Cancel,
            KeyCode::Char('o') => {
                DialogResult::Submit(ListingDetailAction::OpenUrl(self.listing.url.clone()))
            }
            _ => DialogResult::Continue,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Poll image receiver
        if let Some(ref mut rx) = self.image_rx {
            if let Ok(result) = rx.try_recv() {
                if let Some(img) = result {
                    if let Some(ref picker) = self.picker {
                        self.image_state = Some(picker.new_resize_protocol(img));
                    }
                }
                self.image_loading = false;
                self.image_rx = None;
            }
        }

        // Poll description receiver
        if let Some(ref mut rx) = self.description_rx {
            if let Ok(result) = rx.try_recv() {
                self.description = result;
                self.description_loading = false;
                self.description_rx = None;
            }
        }

        // Determine whether to show an image area
        let show_image_area = self.image_loading || self.image_state.is_some();
        let image_height: u16 = if show_image_area { 16 } else { 0 };

        // Determine whether to show a description area
        let fetched_desc = self.description.as_ref().or(self.listing.description.as_ref());
        let show_desc_area = self.description_loading || fetched_desc.is_some();
        let desc_height: u16 = if show_desc_area { 6 } else { 0 };

        let dialog_width = 70u16.min(area.width.saturating_sub(4));
        let base_height: u16 = 18;
        let dialog_height = (base_height + image_height + desc_height).min(area.height.saturating_sub(4));

        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        frame.render_widget(Clear, dialog_area);
        frame.render_widget(
            Block::default().style(Style::default().bg(theme.bg)),
            dialog_area,
        );

        let block = Block::default()
            .title(Span::styled(
                " Listing Details ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .border_type(BorderType::Rounded);

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Build layout constraints dynamically
        let mut constraints = vec![];
        if show_image_area {
            constraints.push(Constraint::Length(image_height));
        }
        constraints.push(Constraint::Length(2)); // title
        if show_desc_area {
            constraints.push(Constraint::Length(desc_height));
        }
        constraints.push(Constraint::Min(1));    // details table
        constraints.push(Constraint::Length(2)); // URL
        constraints.push(Constraint::Length(1)); // hint

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        let mut idx = 0;

        // Image area
        if show_image_area {
            let image_area = chunks[idx];
            idx += 1;
            if let Some(ref mut state) = self.image_state {
                let image_widget = StatefulImage::default();
                frame.render_stateful_widget(image_widget, image_area, state);
            } else if self.image_loading {
                let loading = Paragraph::new(Span::styled(
                    "Loading image...",
                    Style::default().fg(theme.fg_dim),
                ));
                frame.render_widget(loading, image_area);
            }
        }

        // Title
        let title = Paragraph::new(Span::styled(
            &self.listing.title,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ))
        .wrap(Wrap { trim: false });
        frame.render_widget(title, chunks[idx]);
        idx += 1;

        // Description area (right after title)
        if show_desc_area {
            let desc_area = chunks[idx];
            idx += 1;

            if self.description_loading {
                let loading = Paragraph::new(Span::styled(
                    "Loading description...",
                    Style::default().fg(theme.fg_dim),
                ));
                frame.render_widget(loading, desc_area);
            } else if let Some(desc) = fetched_desc {
                let cleaned = strip_html(desc);
                let desc_para = Paragraph::new(cleaned.as_str())
                    .style(Style::default().fg(theme.fg_dim))
                    .wrap(Wrap { trim: false });
                frame.render_widget(desc_para, desc_area);
            }
        }

        // Details table
        let dim = Style::default().fg(theme.fg_dim);
        let fg = Style::default().fg(theme.fg);

        let mut rows: Vec<Row> = vec![];

        if let Some(price) = self.listing.price {
            let price_str = format!("${:.2}", price);
            rows.push(Row::new(vec![
                Cell::from("Price").style(dim),
                Cell::from(price_str).style(Style::default().fg(theme.price)),
            ]));
        }

        rows.push(Row::new(vec![
            Cell::from("Marketplace").style(dim),
            Cell::from(self.listing.marketplace.to_string())
                .style(Style::default().fg(theme.marketplace)),
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
        frame.render_widget(table, chunks[idx]);
        idx += 1;

        // URL
        let url = Paragraph::new(Line::from(vec![
            Span::styled("URL  ", dim),
            Span::styled(&self.listing.url, Style::default().fg(theme.accent)),
        ]))
        .wrap(Wrap { trim: false });
        frame.render_widget(url, chunks[idx]);
        idx += 1;

        // Hint
        let hint = Paragraph::new(Span::styled(
            "[o] open in browser  [Esc] close",
            Style::default().fg(theme.fg_dim),
        ));
        frame.render_widget(hint, chunks[idx]);
    }
}

async fn fetch_image(url: &str) -> anyhow::Result<image::DynamicImage> {
    let bytes = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?
        .get(url)
        .send()
        .await?
        .bytes()
        .await?;
    let img = image::load_from_memory(&bytes)?;
    Ok(img)
}

fn strip_html(html: &str) -> String {
    html2text::from_read(html.as_bytes(), 200).unwrap_or_else(|_| html.to_string())
}

pub enum ListingDetailAction {
    OpenUrl(String),
}
