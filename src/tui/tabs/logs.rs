use crate::tui::theme::Theme;
use crate::types::{LogEntry, LogLevel};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::collections::VecDeque;

const MAX_ENTRIES: usize = 200;

pub struct LogsTab {
    pub entries: VecDeque<LogEntry>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub scheduler_active: bool,
}

impl Default for LogsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl LogsTab {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            scroll_offset: 0,
            auto_scroll: true,
            scheduler_active: false,
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        self.entries.push_back(entry);
        if self.entries.len() > MAX_ENTRIES {
            self.entries.pop_front();
            if self.scroll_offset > 0 {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
        }
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.entries.len().saturating_sub(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                    self.auto_scroll = false;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.scroll_offset < self.entries.len().saturating_sub(1) {
                    self.scroll_offset += 1;
                }
                if self.scroll_offset >= self.entries.len().saturating_sub(1) {
                    self.auto_scroll = true;
                }
            }
            KeyCode::Char('G') => {
                self.scroll_to_bottom();
                self.auto_scroll = true;
            }
            KeyCode::Char('c') => {
                self.entries.clear();
                self.scroll_offset = 0;
                self.auto_scroll = true;
            }
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(Span::styled(
                " Logs ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if !self.scheduler_active {
            let msg = Paragraph::new("Logs available when scheduler is active in this instance.")
                .style(Style::default().fg(theme.fg_dim));
            frame.render_widget(msg, inner);
            return;
        }

        if self.entries.is_empty() {
            let msg = Paragraph::new("No log entries yet.")
                .style(Style::default().fg(theme.fg_dim));
            frame.render_widget(msg, inner);
            return;
        }

        let visible = inner.height as usize;
        let start = self.scroll_offset.saturating_sub(visible.saturating_sub(1));
        let end = (start + visible).min(self.entries.len());

        let lines: Vec<Line> = self.entries
            .iter()
            .skip(start)
            .take(end - start)
            .map(|entry| {
                let time = entry.timestamp.format("%H:%M:%S").to_string();
                let (level_str, level_color) = match entry.level {
                    LogLevel::Info => ("INFO ", theme.fg),
                    LogLevel::Debug => ("DEBUG", theme.fg_dim),
                    LogLevel::Error => ("ERROR", theme.disabled),
                };

                Line::from(vec![
                    Span::styled(
                        format!("[{}] ", time),
                        Style::default().fg(theme.fg_dim),
                    ),
                    Span::styled(
                        format!("[{}] ", level_str),
                        Style::default().fg(level_color),
                    ),
                    Span::styled(&entry.message, Style::default().fg(theme.fg)),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);
    }
}
