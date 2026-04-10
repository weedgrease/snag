use crate::config::{self, AppConfig, load_config, save_config};
use crate::daemon::results::{load_results, results_path};
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
use ratatui::widgets::{Block, Borders, Tabs};
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
                    } else if key.code == KeyCode::Char('q') {
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
                                        crate::tui::tabs::alerts::AlertsAction::CreateAlert => {}
                                        crate::tui::tabs::alerts::AlertsAction::EditAlert(_idx) => {}
                                        crate::tui::tabs::alerts::AlertsAction::DeleteAlert(idx) => {
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

    fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(frame.area());

        self.render_tabs(frame, chunks[0]);

        match self.active_tab {
            TabKind::Alerts => self.alerts_tab.render(frame, chunks[1], &self.theme, &self.config),
            TabKind::Results => self.results_tab.render(frame, chunks[1], &self.theme, &self.results),
            TabKind::Settings => self.settings_tab.render(frame, chunks[1], &self.theme, &self.config),
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
}
