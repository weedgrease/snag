use crate::config::AppConfig;
use crate::tui::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::path::PathBuf;

pub struct SettingsTab {
    pub selected: usize,
    pub field_count: usize,
}

impl SettingsTab {
    pub fn new() -> Self {
        Self {
            selected: 0,
            field_count: 3,
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        _config: &mut AppConfig,
    ) -> Option<SettingsAction> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected < self.field_count - 1 {
                    self.selected += 1;
                }
            }
            KeyCode::Char('r') => return Some(SettingsAction::RestartDaemon),
            KeyCode::Char('s') => return Some(SettingsAction::StopDaemon),
            _ => {}
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, config: &AppConfig) {
        let block = Block::default()
            .title(Span::styled(
                " Settings ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Length(10), Constraint::Min(0)])
            .split(inner);

        self.render_daemon_section(frame, chunks[0], theme);
        self.render_defaults_section(frame, chunks[1], theme, config);
    }

    fn render_daemon_section(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let pid_path = crate::config::data_dir().join("daemon.pid");
        let (status, pid) = read_daemon_status(&pid_path);

        let status_color = if status == "Running" {
            theme.enabled
        } else {
            theme.disabled
        };

        let mut lines = vec![];
        lines.push(Line::from(Span::styled(
            "Daemon",
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Status    ", Style::default().fg(theme.fg_dim)),
            Span::styled(&status, Style::default().fg(status_color)),
            Span::styled(
                pid.map(|p| format!(" (PID {})", p)).unwrap_or_default(),
                Style::default().fg(theme.fg_dim),
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  [r] restart  [s] stop",
            Style::default().fg(theme.accent),
        )));

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

    fn render_defaults_section(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        config: &AppConfig,
    ) {
        let interval_secs = config.settings.default_check_interval.as_secs();
        let interval_str = if interval_secs >= 3600 {
            format!("{}h", interval_secs / 3600)
        } else if interval_secs >= 60 {
            format!("{}m", interval_secs / 60)
        } else {
            format!("{}s", interval_secs)
        };

        let mut lines = vec![];
        lines.push(Line::from(Span::styled(
            "Defaults",
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Check interval  ", Style::default().fg(theme.fg_dim)),
            Span::styled(interval_str, Style::default().fg(theme.fg)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("  Max results     ", Style::default().fg(theme.fg_dim)),
            Span::styled(
                config
                    .settings
                    .default_max_results
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "unlimited".into()),
                Style::default().fg(theme.fg),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("  Notification    ", Style::default().fg(theme.fg_dim)),
            Span::styled(
                config.settings.default_notifier.to_string(),
                Style::default().fg(theme.fg),
            ),
        ]));

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }
}

fn read_daemon_status(pid_path: &PathBuf) -> (String, Option<u32>) {
    let pid_str = match std::fs::read_to_string(pid_path) {
        Ok(s) => s,
        Err(_) => return ("Stopped".into(), None),
    };

    let pid: u32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => return ("Stopped".into(), None),
    };

    let alive = std::path::Path::new(&format!("/proc/{}", pid)).exists();
    if alive {
        ("Running".into(), Some(pid))
    } else {
        ("Stopped (stale PID)".into(), Some(pid))
    }
}

pub enum SettingsAction {
    StartDaemon,
    StopDaemon,
    RestartDaemon,
}
