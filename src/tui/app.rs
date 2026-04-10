use crate::config::{self, AppConfig, load_config, save_config};
use crate::daemon::results::{load_results, results_path};
use crate::tui::dialogs::alert_form::AlertFormDialog;
use crate::tui::dialogs::confirm::ConfirmDialog;
use crate::tui::dialogs::DialogResult;
use crate::tui::tabs::alerts::AlertsTab;
use crate::tui::tabs::results::ResultsTab;
use crate::tui::tabs::settings::SettingsTab;
use crate::tui::tabs::TabKind;
use crate::tui::theme::Theme;
use crate::types::AlertResult;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;
use std::time::{Duration, Instant};

pub struct App {
    pub active_tab: TabKind,
    pub config: AppConfig,
    pub config_path: std::path::PathBuf,
    pub results: Vec<AlertResult>,
    pub results_path: std::path::PathBuf,
    pub theme: Theme,
    pub alerts_tab: AlertsTab,
    pub results_tab: ResultsTab,
    pub settings_tab: SettingsTab,
    pub should_quit: bool,
    pub active_dialog: Option<ActiveDialog>,
}

pub enum ActiveDialog {
    AlertForm(AlertFormDialog),
    Confirm(ConfirmDialog, ConfirmAction),
}

pub enum ConfirmAction {
    DeleteAlert(usize),
    ClearResults,
}

impl App {
    pub fn new() -> Result<Self> {
        let config_path = config::config_path();
        let results_path = results_path();
        let config = load_config(&config_path).unwrap_or_default();
        let results = load_results(&results_path).unwrap_or_default();

        Ok(Self {
            active_tab: TabKind::Alerts,
            config,
            config_path,
            results,
            results_path,
            theme: Theme::default(),
            alerts_tab: AlertsTab::new(),
            results_tab: ResultsTab::new(),
            settings_tab: SettingsTab::new(),
            should_quit: false,
            active_dialog: None,
        })
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        let mut last_results_refresh = Instant::now();
        let results_refresh_interval = Duration::from_secs(2);

        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.should_quit = true;
                        continue;
                    }

                    // Dialog handling takes priority over all other input
                    if self.active_dialog.is_some() {
                        self.handle_dialog_key(key);
                        continue;
                    }

                    if key.code == KeyCode::Char('q') {
                        self.should_quit = true;
                    } else if key.code == KeyCode::Tab {
                        self.active_tab = self.active_tab.next();
                    } else if key.code == KeyCode::BackTab {
                        self.active_tab = self.active_tab.prev();
                    } else if key.code == KeyCode::Char('1') {
                        self.active_tab = TabKind::Alerts;
                    } else if key.code == KeyCode::Char('2') {
                        self.active_tab = TabKind::Results;
                    } else if key.code == KeyCode::Char('3') {
                        self.active_tab = TabKind::Settings;
                    } else {
                        match self.active_tab {
                            TabKind::Alerts => {
                                if let Some(action) = self.alerts_tab.handle_key(key, &mut self.config) {
                                    match action {
                                        crate::tui::tabs::alerts::AlertsAction::ConfigChanged => {
                                            let _ = save_config(&self.config, &self.config_path);
                                        }
                                        crate::tui::tabs::alerts::AlertsAction::CreateAlert => {
                                            self.active_dialog = Some(ActiveDialog::AlertForm(AlertFormDialog::new()));
                                        }
                                        crate::tui::tabs::alerts::AlertsAction::EditAlert(idx) => {
                                            if let Some(alert) = self.config.alerts.get(idx) {
                                                let dialog = AlertFormDialog::from_alert(alert);
                                                self.active_dialog = Some(ActiveDialog::AlertForm(dialog));
                                            }
                                        }
                                        crate::tui::tabs::alerts::AlertsAction::DeleteAlert(idx) => {
                                            if idx < self.config.alerts.len() {
                                                let name = self.config.alerts[idx].name.clone();
                                                let dialog = ConfirmDialog::new(
                                                    "Delete Alert".to_string(),
                                                    format!("Delete alert \"{}\"?", name),
                                                );
                                                self.active_dialog = Some(ActiveDialog::Confirm(dialog, ConfirmAction::DeleteAlert(idx)));
                                            }
                                        }
                                    }
                                }
                            }
                            TabKind::Results => {
                                if let Some(action) = self.results_tab.handle_key(key, &mut self.results) {
                                    match action {
                                        crate::tui::tabs::results::ResultsAction::OpenUrl(url) => {
                                            let _ = open::that(&url);
                                        }
                                        crate::tui::tabs::results::ResultsAction::ResultsChanged => {
                                            let _ = crate::daemon::results::save_results(
                                                &self.results,
                                                &self.results_path,
                                            );
                                        }
                                    }
                                }
                            }
                            TabKind::Settings => {
                                self.settings_tab.handle_key(key, &mut self.config);
                            }
                        }
                    }
                }
            }

            if last_results_refresh.elapsed() >= results_refresh_interval {
                if let Ok(new_results) = load_results(&self.results_path) {
                    self.results = new_results;
                }
                last_results_refresh = Instant::now();
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    fn handle_dialog_key(&mut self, key: crossterm::event::KeyEvent) {
        let result = match &mut self.active_dialog {
            Some(ActiveDialog::AlertForm(dialog)) => {
                let r = dialog.handle_key(key);
                match r {
                    DialogResult::Cancel => Some(DialogResult::<()>::Cancel),
                    DialogResult::Continue => None,
                    DialogResult::Submit(alert) => {
                        // Add or update alert in config
                        if let Some(existing_id) = alert.id.into() {
                            if let Some(pos) = self.config.alerts.iter().position(|a| a.id == existing_id) {
                                self.config.alerts[pos] = alert;
                            } else {
                                self.config.alerts.push(alert);
                            }
                        }
                        let _ = save_config(&self.config, &self.config_path);
                        Some(DialogResult::<()>::Cancel)
                    }
                }
            }
            Some(ActiveDialog::Confirm(dialog, _)) => {
                let r = dialog.handle_key(key);
                match r {
                    DialogResult::Cancel => Some(DialogResult::<()>::Cancel),
                    DialogResult::Continue => None,
                    DialogResult::Submit(_) => Some(DialogResult::<()>::Submit(())),
                }
            }
            None => None,
        };

        match result {
            Some(DialogResult::Submit(())) => {
                // Execute the confirm action
                let dialog = self.active_dialog.take();
                if let Some(ActiveDialog::Confirm(_, action)) = dialog {
                    match action {
                        ConfirmAction::DeleteAlert(idx) => {
                            if idx < self.config.alerts.len() {
                                self.config.alerts.remove(idx);
                                let _ = save_config(&self.config, &self.config_path);
                                if self.alerts_tab.selected >= self.config.alerts.len()
                                    && self.alerts_tab.selected > 0
                                {
                                    self.alerts_tab.selected -= 1;
                                    self.alerts_tab.list_state.select(Some(self.alerts_tab.selected));
                                }
                            }
                        }
                        ConfirmAction::ClearResults => {
                            self.results.clear();
                            let _ = crate::daemon::results::save_results(
                                &self.results,
                                &self.results_path,
                            );
                        }
                    }
                }
            }
            Some(DialogResult::Cancel) => {
                self.active_dialog = None;
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        self.render_tabs(frame, chunks[0]);

        match self.active_tab {
            TabKind::Alerts => self.alerts_tab.render(frame, chunks[1], &self.theme, &self.config),
            TabKind::Results => self.results_tab.render(frame, chunks[1], &self.theme, &self.results),
            TabKind::Settings => self.settings_tab.render(frame, chunks[1], &self.theme, &self.config),
        }

        self.render_status_bar(frame, chunks[2]);

        // Draw dialogs on top of normal content
        if let Some(dialog) = &self.active_dialog {
            match dialog {
                ActiveDialog::AlertForm(d) => d.render(frame, frame.area(), &self.theme),
                ActiveDialog::Confirm(d, _) => d.render(frame, frame.area(), &self.theme),
            }
        }
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let tab_titles: Vec<Line> = TabKind::all()
            .iter()
            .map(|t| {
                let style = if *t == self.active_tab {
                    Style::default()
                        .fg(self.theme.active_tab)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(self.theme.inactive_tab)
                };
                Line::from(Span::styled(t.title(), style))
            })
            .collect();

        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(self.theme.border))
                    .title(Span::styled(
                        " snag ",
                        Style::default()
                            .fg(self.theme.accent)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .select(
                TabKind::all()
                    .iter()
                    .position(|t| *t == self.active_tab)
                    .unwrap_or(0),
            )
            .highlight_style(
                Style::default()
                    .fg(self.theme.active_tab)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(tabs, area);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let hints = match self.active_tab {
            TabKind::Alerts => "[n]ew [e]dit [d]elete [space]toggle [q]uit",
            TabKind::Results => "[o]pen [m]ark read [c]lear [q]uit",
            TabKind::Settings => "[r]estart [s]top daemon [q]uit",
        };

        let bar = Paragraph::new(Line::from(vec![
            Span::styled(
                " Tab/1-3 ",
                Style::default()
                    .fg(self.theme.status_bar_fg)
                    .bg(self.theme.accent),
            ),
            Span::styled(
                format!(" {} ", hints),
                Style::default()
                    .fg(self.theme.status_bar_fg)
                    .bg(self.theme.status_bar_bg),
            ),
        ]));

        frame.render_widget(bar, area);
    }
}
