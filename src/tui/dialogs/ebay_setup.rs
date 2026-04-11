use super::DialogResult;
use crate::tui::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

pub struct EbaySetupDialog {
    step: SetupStep,
    client_id: String,
    client_secret: String,
    editing_field: usize,
    cursor: usize,
}

enum SetupStep {
    Intro,
    Credentials,
}

impl EbaySetupDialog {
    pub fn new() -> Self {
        Self {
            step: SetupStep::Intro,
            client_id: String::new(),
            client_secret: String::new(),
            editing_field: 0,
            cursor: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DialogResult<EbaySetupAction> {
        match self.step {
            SetupStep::Intro => match key.code {
                KeyCode::Enter => {
                    DialogResult::Submit(EbaySetupAction::OpenRegistration)
                }
                KeyCode::Char('s') => {
                    self.step = SetupStep::Credentials;
                    DialogResult::Continue
                }
                KeyCode::Esc => DialogResult::Cancel,
                _ => DialogResult::Continue,
            },
            SetupStep::Credentials => match key.code {
                KeyCode::Esc => {
                    self.step = SetupStep::Intro;
                    DialogResult::Continue
                }
                KeyCode::Tab => {
                    self.editing_field = (self.editing_field + 1) % 2;
                    self.cursor = self.active_field().len();
                    DialogResult::Continue
                }
                KeyCode::Enter => {
                    if self.editing_field == 0 {
                        self.editing_field = 1;
                        self.cursor = self.client_secret.len();
                        DialogResult::Continue
                    } else if !self.client_id.is_empty() && !self.client_secret.is_empty() {
                        DialogResult::Submit(EbaySetupAction::SaveCredentials {
                            client_id: self.client_id.clone(),
                            client_secret: self.client_secret.clone(),
                        })
                    } else {
                        DialogResult::Continue
                    }
                }
                KeyCode::Backspace => {
                    if self.cursor > 0 {
                        let cursor = self.cursor;
                        let field = self.active_field_mut();
                        let prev = field[..cursor]
                            .char_indices()
                            .last()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        field.remove(prev);
                        self.cursor = prev;
                    }
                    DialogResult::Continue
                }
                KeyCode::Char(c) => {
                    let cursor = self.cursor;
                    let field = self.active_field_mut();
                    field.insert(cursor, c);
                    self.cursor += c.len_utf8();
                    DialogResult::Continue
                }
                _ => DialogResult::Continue,
            },
        }
    }

    fn active_field(&self) -> &str {
        match self.editing_field {
            0 => &self.client_id,
            _ => &self.client_secret,
        }
    }

    fn active_field_mut(&mut self) -> &mut String {
        match self.editing_field {
            0 => &mut self.client_id,
            _ => &mut self.client_secret,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog_width = 65u16.min(area.width.saturating_sub(4));
        let dialog_height = 18u16.min(area.height.saturating_sub(4));

        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(Span::styled(
                " eBay Setup ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .border_type(BorderType::Rounded);

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        match self.step {
            SetupStep::Intro => self.render_intro(frame, inner, theme),
            SetupStep::Credentials => self.render_credentials(frame, inner, theme),
        }
    }

    fn render_intro(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "eBay requires API credentials to search listings.",
                Style::default().fg(theme.fg),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Steps:",
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "  1. Press Enter to open the eBay Developer Portal",
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                "  2. Register / sign in",
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                "  3. Create an application (Production keys)",
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                "  4. Copy your Client ID and Client Secret",
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                "  5. Press [s] to enter credentials",
                Style::default().fg(theme.fg),
            )),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "[Enter] open eBay Developer Portal  [s] enter credentials  [Esc] cancel",
                Style::default().fg(theme.fg_dim),
            )),
        ];
        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

    fn render_credentials(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let header = Paragraph::new(Span::styled(
            "Enter your eBay API credentials:",
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(header, chunks[0]);

        let id_label_style = if self.editing_field == 0 {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.fg_dim)
        };
        let id_value_style = if self.editing_field == 0 {
            Style::default().fg(theme.fg).add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.fg)
        };
        let id_cursor = if self.editing_field == 0 { "▸ " } else { "  " };
        let id_line = Line::from(vec![
            Span::styled(id_cursor, Style::default().fg(theme.accent)),
            Span::styled("Client ID:     ", id_label_style),
            Span::styled(&self.client_id, id_value_style),
        ]);
        frame.render_widget(Paragraph::new(id_line), chunks[1]);

        let secret_label_style = if self.editing_field == 1 {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.fg_dim)
        };
        let secret_value_style = if self.editing_field == 1 {
            Style::default().fg(theme.fg).add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.fg)
        };
        let secret_cursor = if self.editing_field == 1 { "▸ " } else { "  " };
        let masked = "*".repeat(self.client_secret.len());
        let secret_line = Line::from(vec![
            Span::styled(secret_cursor, Style::default().fg(theme.accent)),
            Span::styled("Client Secret: ", secret_label_style),
            Span::styled(masked, secret_value_style),
        ]);
        frame.render_widget(Paragraph::new(secret_line), chunks[2]);

        let hint = Paragraph::new(Span::styled(
            "[Tab] switch field  [Enter] save  [Esc] back",
            Style::default().fg(theme.fg_dim),
        ));
        frame.render_widget(hint, chunks[4]);
    }
}

impl Default for EbaySetupDialog {
    fn default() -> Self {
        Self::new()
    }
}

pub enum EbaySetupAction {
    OpenRegistration,
    SaveCredentials {
        client_id: String,
        client_secret: String,
    },
}
