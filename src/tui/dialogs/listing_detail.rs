use super::DialogResult;
use crate::tui::theme::Theme;
use crate::types::Listing;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table, Wrap,
};
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
    detail_rx: Option<tokio::sync::oneshot::Receiver<(Option<String>, Option<String>)>>,
    picker: Option<ratatui_image::picker::Picker>,
    desc_scroll: u16,
}

impl ListingDetailDialog {
    pub fn new(listing: Listing, alert_name: String) -> Self {
        let mut picker = ratatui_image::picker::Picker::from_query_stdio()
            .unwrap_or_else(|_| ratatui_image::picker::Picker::halfblocks());

        if picker.protocol_type() == ratatui_image::picker::ProtocolType::Halfblocks {
            picker.set_protocol_type(ratatui_image::picker::ProtocolType::Sixel);
            log::debug!(target: "snag::image", "Auto-detect got Halfblocks, trying Sixel");
        } else {
            log::debug!(target: "snag::image", "Image protocol: {:?}", picker.protocol_type());
        }

        let picker = Some(picker);

        let marketplace = listing.marketplace;
        let listing_id = listing.id.clone();
        let is_ebay = marketplace == crate::types::MarketplaceKind::Ebay;

        // For eBay: fetch item details first (high-res image + description), then load image
        // For others: load image directly from the search result URL
        let (img_tx, img_rx) = tokio::sync::oneshot::channel();
        let (detail_tx, detail_rx) = tokio::sync::oneshot::channel();

        let search_image_url = listing.image_url.clone();

        tokio::spawn(async move {
            if is_ebay {
                let details = crate::marketplace::providers::ebay::fetch_item_details(&listing_id)
                    .await
                    .ok()
                    .unwrap_or((None, None));

                let (desc, hires_url) = details;
                let _ = detail_tx.send((desc, hires_url.clone()));

                let url_to_fetch = hires_url.or(search_image_url);
                let img = if let Some(url) = url_to_fetch {
                    fetch_image(&url).await.ok()
                } else {
                    None
                };
                let _ = img_tx.send(img);
            } else {
                let _ = detail_tx.send((None, None));
                let img = if let Some(url) = search_image_url {
                    fetch_image(&url).await.ok()
                } else {
                    None
                };
                let _ = img_tx.send(img);
            }
        });

        Self {
            listing,
            alert_name,
            image_state: None,
            image_loading: true,
            image_rx: Some(img_rx),
            description: None,
            description_loading: is_ebay,
            detail_rx: Some(detail_rx),
            picker,
            desc_scroll: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DialogResult<ListingDetailAction> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => DialogResult::Cancel,
            KeyCode::Char('o') => {
                DialogResult::Submit(ListingDetailAction::OpenUrl(self.listing.url.clone()))
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.desc_scroll = self.desc_scroll.saturating_add(1);
                DialogResult::Continue
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.desc_scroll = self.desc_scroll.saturating_sub(1);
                DialogResult::Continue
            }
            _ => DialogResult::Continue,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Poll image
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

        // Poll details (description + hires image URL)
        if let Some(ref mut rx) = self.detail_rx {
            if let Ok((desc, _img_url)) = rx.try_recv() {
                self.description = desc;
                self.description_loading = false;
                self.detail_rx = None;
            }
        }

        let has_image = self.image_loading || self.image_state.is_some();
        let fetched_desc = self
            .description
            .as_ref()
            .or(self.listing.description.as_ref());
        let has_desc = self.description_loading || fetched_desc.is_some();

        // Count detail rows for tight layout
        let mut detail_rows: u16 = 2; // marketplace + found (always present)
        if self.listing.price.is_some() {
            detail_rows += 1;
        }
        if self.listing.location.is_some() {
            detail_rows += 1;
        }
        if self.listing.condition.is_some() {
            detail_rows += 1;
        }
        if self.listing.posted_at.is_some() {
            detail_rows += 1;
        }
        detail_rows += 1; // alert row

        let image_rows: u16 = if has_image { 14 } else { 0 };
        let desc_rows: u16 = if has_desc { 8 } else { 0 };
        let fixed_rows: u16 = 2 + detail_rows + desc_rows + 2 + 1 + image_rows;

        let dialog_width = area.width.saturating_sub(6).min(100);
        let dialog_height = (fixed_rows + 2).min(area.height.saturating_sub(2));

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

        let mut constraints = vec![];
        if has_image {
            constraints.push(Constraint::Length(image_rows));
        }
        constraints.push(Constraint::Length(2)); // title
        constraints.push(Constraint::Length(detail_rows)); // details
        if has_desc {
            constraints.push(Constraint::Length(desc_rows));
        }
        constraints.push(Constraint::Length(2)); // URL
        constraints.push(Constraint::Length(1)); // hint

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        let mut idx = 0;

        // Image
        if has_image {
            if let Some(ref mut state) = self.image_state {
                frame.render_stateful_widget(StatefulImage::default(), chunks[idx], state);
            } else if self.image_loading {
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        "Loading image...",
                        Style::default().fg(theme.fg_dim),
                    )),
                    chunks[idx],
                );
            }
            idx += 1;
        }

        // Title
        frame.render_widget(
            Paragraph::new(Span::styled(
                &self.listing.title,
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ))
            .wrap(Wrap { trim: false }),
            chunks[idx],
        );
        idx += 1;

        // Details table (tight)
        let dim = Style::default().fg(theme.fg_dim);
        let fg = Style::default().fg(theme.fg);
        let mut rows: Vec<Row> = vec![];

        if let Some(price) = self.listing.price {
            rows.push(Row::new(vec![
                Cell::from("Price").style(dim),
                Cell::from(format!("${:.2}", price)).style(Style::default().fg(theme.price)),
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
            Cell::from(
                self.listing
                    .found_at
                    .format("%Y-%m-%d %H:%M")
                    .to_string(),
            )
            .style(fg),
        ]));
        rows.push(Row::new(vec![
            Cell::from("Alert").style(dim),
            Cell::from(self.alert_name.as_str()).style(fg),
        ]));

        frame.render_widget(
            Table::new(rows, [Constraint::Length(16), Constraint::Min(10)]),
            chunks[idx],
        );
        idx += 1;

        // Description
        if has_desc {
            let desc_area = chunks[idx];
            idx += 1;

            let desc_block = Block::default()
                .title(Span::styled(
                    " Description ",
                    Style::default()
                        .fg(theme.fg_dim)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme.border))
                .border_type(BorderType::Rounded);

            let desc_inner = desc_block.inner(desc_area);
            frame.render_widget(desc_block, desc_area);

            if self.description_loading {
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        "Loading description...",
                        Style::default().fg(theme.fg_dim),
                    )),
                    desc_inner,
                );
            } else if let Some(desc) = fetched_desc {
                let cleaned = strip_html(desc);
                frame.render_widget(
                    Paragraph::new(cleaned.as_str())
                        .style(Style::default().fg(theme.fg))
                        .wrap(Wrap { trim: false })
                        .scroll((self.desc_scroll, 0)),
                    desc_inner,
                );

                let wrap_width = desc_inner.width.max(1) as usize;
                let wrapped_lines: usize = cleaned
                    .lines()
                    .map(|line| (line.len() / wrap_width).max(1))
                    .sum();
                let mut sb_state = ScrollbarState::new(wrapped_lines)
                    .position(self.desc_scroll as usize);
                frame.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight),
                    desc_inner,
                    &mut sb_state,
                );
            }
        }

        // URL
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("URL  ", dim),
                Span::styled(&self.listing.url, Style::default().fg(theme.accent)),
            ]))
            .wrap(Wrap { trim: false }),
            chunks[idx],
        );
        idx += 1;

        // Hint
        frame.render_widget(
            Paragraph::new(Span::styled(
                "[o] open  [↑↓] scroll description  [Esc] close",
                Style::default().fg(theme.fg_dim),
            )),
            chunks[idx],
        );
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
    Ok(image::load_from_memory(&bytes)?)
}

fn strip_html(html: &str) -> String {
    html2text::from_read(html.as_bytes(), 200).unwrap_or_else(|_| html.to_string())
}

pub enum ListingDetailAction {
    OpenUrl(String),
}
