use crate::config::{AppConfig, load_config, save_config};
use crate::daemon::results::{load_results, results_path};
use crate::tui::dialogs::DialogResult;
use crate::tui::dialogs::alert_form::AlertFormDialog;
use crate::tui::dialogs::confirm::ConfirmDialog;
use crate::tui::tabs::TabKind;
use crate::tui::tabs::alerts::AlertsTab;
use crate::tui::tabs::results::ResultsTab;
use crate::tui::tabs::settings::SettingsTab;
use crate::tui::theme::Theme;
use crate::types::AlertResult;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use std::time::{Duration, Instant};

/// Central application state for the TUI: owns the config, results, scheduler handle, and tab widgets.
pub struct App {
    pub active_tab: TabKind,
    pub config: AppConfig,
    pub config_path: std::path::PathBuf,
    pub results: Vec<AlertResult>,
    pub results_path: std::path::PathBuf,
    pub statuses: Vec<crate::types::CheckStatus>,
    pub status_path: std::path::PathBuf,
    pub theme: Theme,
    pub alerts_tab: AlertsTab,
    pub results_tab: ResultsTab,
    pub settings_tab: SettingsTab,
    pub logs_tab: crate::tui::tabs::logs::LogsTab,
    pub should_quit: bool,
    pub active_dialog: Option<ActiveDialog>,
    pub update_info: Option<crate::update::UpdateInfo>,
    pub pending_update: Option<crate::update::UpdateInfo>,
    update_rx: Option<tokio::sync::oneshot::Receiver<Option<crate::update::UpdateInfo>>>,
    scheduler_rx: Option<tokio::sync::mpsc::Receiver<crate::scheduler::SchedulerEvent>>,
    force_event_tx: Option<tokio::sync::mpsc::Sender<crate::scheduler::SchedulerEvent>>,
    config_tx: Option<tokio::sync::watch::Sender<AppConfig>>,
    _scheduler_lock: Option<std::fs::File>,
    last_results_mtime: Option<std::time::SystemTime>,
    last_status_mtime: Option<std::time::SystemTime>,
    pub seen_ids: std::collections::HashSet<String>,
    pub seen_path: std::path::PathBuf,
    last_seen_mtime: Option<std::time::SystemTime>,
}

/// The dialog currently overlaid on the TUI; only one dialog is shown at a time.
pub enum ActiveDialog {
    AlertForm(AlertFormDialog),
    Confirm(ConfirmDialog, ConfirmAction),
    ListingDetail(crate::tui::dialogs::listing_detail::ListingDetailDialog),
    EbaySetup(crate::tui::dialogs::ebay_setup::EbaySetupDialog),
}

pub enum ConfirmAction {
    DeleteAlert(usize),
    ClearResults,
    PerformUpdate,
}

impl App {
    pub fn new() -> Result<Self> {
        let config_path = crate::config::config_path();
        let results_path = results_path();
        let config = load_config(&config_path).unwrap_or_default();
        let results = load_results(&results_path).unwrap_or_default();
        let status_path = crate::daemon::results::status_path();
        let statuses = crate::daemon::results::load_status(&status_path).unwrap_or_default();

        let seen_path = crate::daemon::results::seen_path();
        let seen_ids = crate::daemon::results::load_seen(&seen_path).unwrap_or_default();

        let existing_ids: std::collections::HashSet<String> = results
            .iter()
            .flat_map(|r| r.listings.iter().map(|l| l.id.clone()))
            .collect();

        let (scheduler_rx, config_tx, scheduler_lock, force_event_tx) =
            if let Some(lock) = crate::scheduler::try_acquire_scheduler_lock() {
                let (event_tx, event_rx) =
                    tokio::sync::mpsc::channel::<crate::scheduler::SchedulerEvent>(64);
                let (cfg_tx, cfg_rx) = tokio::sync::watch::channel(config.clone());
                let force_tx = event_tx.clone();
                let scheduler = crate::scheduler::Scheduler::new(event_tx, cfg_rx, existing_ids);
                tokio::spawn(scheduler.run());
                (Some(event_rx), Some(cfg_tx), Some(lock), Some(force_tx))
            } else {
                (None, None, None, None)
            };

        let logs_tab = crate::tui::tabs::logs::LogsTab::new();

        let update_rx = if config.settings.check_for_updates {
            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::spawn(async move {
                let result = crate::update::check_for_update().await.ok().flatten();
                let _ = tx.send(result);
            });
            Some(rx)
        } else {
            None
        };

        Ok(Self {
            active_tab: TabKind::Alerts,
            config,
            config_path,
            results,
            results_path,
            statuses,
            status_path,
            theme: Theme::default(),
            alerts_tab: AlertsTab::new(),
            results_tab: ResultsTab::new(),
            settings_tab: SettingsTab::new(),
            logs_tab,
            should_quit: false,
            active_dialog: None,
            update_info: None,
            pending_update: None,
            update_rx,
            scheduler_rx,
            force_event_tx,
            config_tx,
            _scheduler_lock: scheduler_lock,
            last_results_mtime: None,
            last_status_mtime: None,
            seen_ids,
            seen_path,
            last_seen_mtime: None,
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

            if event::poll(Duration::from_millis(50))?
                && let Event::Key(key) = event::read()?
            {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.should_quit = true;
                    continue;
                }

                // Dialog handling takes priority over all other input
                if self.active_dialog.is_some() {
                    self.handle_dialog_key(key);
                    continue;
                }

                if key.code == KeyCode::Char('q') && !self.settings_tab.editing {
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
                } else if key.code == KeyCode::Char('4') {
                    self.active_tab = TabKind::Logs;
                } else if key.code == KeyCode::Char('u') {
                    if self.update_info.is_some() {
                        let info = self.update_info.clone().unwrap();
                        let notes = info.release_notes.clone().unwrap_or_default();
                        let preview = crate::tui::utils::truncate_str(&notes, 200);
                        let dialog = ConfirmDialog::new(
                            "Update snag".to_string(),
                            format!("Update to {}?\n\n{}", info.latest_version, preview),
                        );
                        self.active_dialog =
                            Some(ActiveDialog::Confirm(dialog, ConfirmAction::PerformUpdate));
                    }
                } else {
                    match self.active_tab {
                        TabKind::Alerts => {
                            if let Some(action) = self.alerts_tab.handle_key(key, &mut self.config)
                            {
                                match action {
                                    crate::tui::tabs::alerts::AlertsAction::ConfigChanged => {
                                        let _ = save_config(&self.config, &self.config_path);
                                        if let Some(ref tx) = self.config_tx {
                                            let _ = tx.send(self.config.clone());
                                        }
                                    }
                                    crate::tui::tabs::alerts::AlertsAction::CreateAlert => {
                                        let mut dialog = AlertFormDialog::new();
                                        dialog.set_default_location(
                                            self.config.settings.default_location.clone(),
                                        );
                                        dialog.set_config_defaults(&self.config);
                                        self.active_dialog = Some(ActiveDialog::AlertForm(dialog));
                                    }
                                    crate::tui::tabs::alerts::AlertsAction::EditAlert(idx) => {
                                        if let Some(alert) = self.config.alerts.get(idx) {
                                            let mut dialog = AlertFormDialog::from_alert(alert);
                                            dialog.set_default_location(
                                                self.config.settings.default_location.clone(),
                                            );
                                            self.active_dialog =
                                                Some(ActiveDialog::AlertForm(dialog));
                                        }
                                    }
                                    crate::tui::tabs::alerts::AlertsAction::DeleteAlert(idx) => {
                                        if idx < self.config.alerts.len() {
                                            let name = self.config.alerts[idx].name.clone();
                                            let dialog = ConfirmDialog::new(
                                                "Delete Alert".to_string(),
                                                format!("Delete alert \"{}\"?", name),
                                            );
                                            self.active_dialog = Some(ActiveDialog::Confirm(
                                                dialog,
                                                ConfirmAction::DeleteAlert(idx),
                                            ));
                                        }
                                    }
                                    crate::tui::tabs::alerts::AlertsAction::ViewListing(
                                        alert_idx,
                                        listing_idx,
                                    ) => {
                                        if let Some(alert) = self.config.alerts.get(alert_idx) {
                                            let alert_listings: Vec<&crate::types::Listing> = self
                                                .results
                                                .iter()
                                                .filter(|r| r.alert_id == alert.id)
                                                .flat_map(|r| r.listings.iter())
                                                .collect();
                                            if let Some(listing) = alert_listings.get(listing_idx) {
                                                self.seen_ids.insert(listing.id.clone());
                                                let _ = crate::daemon::results::save_seen(
                                                    &self.seen_ids,
                                                    &self.seen_path,
                                                );
                                                let dialog = crate::tui::dialogs::listing_detail::ListingDetailDialog::new(
                                                        (*listing).clone(),
                                                        alert.name.clone(),
                                                    );
                                                self.active_dialog =
                                                    Some(ActiveDialog::ListingDetail(dialog));
                                            }
                                        }
                                    }
                                    crate::tui::tabs::alerts::AlertsAction::ForceCheck(idx) => {
                                        if let Some(alert) = self.config.alerts.get(idx).cloned() {
                                            let existing_ids: std::collections::HashSet<String> =
                                                self.results
                                                    .iter()
                                                    .flat_map(|r| {
                                                        r.listings.iter().map(|l| l.id.clone())
                                                    })
                                                    .collect();
                                            let default_loc =
                                                self.config.settings.default_location.clone();
                                            let event_tx = self.force_event_tx.clone();
                                            tokio::spawn(async move {
                                                let Some(event_tx) = event_tx else { return };
                                                log::info!(target: "snag::scheduler", "Force checking alert: '{}'", alert.name);
                                                match crate::scheduler::check_alert(
                                                    &alert,
                                                    &existing_ids,
                                                    default_loc.as_deref(),
                                                )
                                                .await
                                                {
                                                    Ok((status, new_listings)) => {
                                                        let result = if new_listings.is_empty() {
                                                            None
                                                        } else {
                                                            Some(crate::types::AlertResult {
                                                                alert_id: alert.id,
                                                                alert_name: alert.name.clone(),
                                                                listings: new_listings,
                                                                checked_at: chrono::Utc::now(),
                                                                seen: false,
                                                            })
                                                        };
                                                        let _ = event_tx.send(crate::scheduler::SchedulerEvent::CheckComplete { status, result }).await;
                                                    }
                                                    Err(e) => {
                                                        let _ = event_tx.send(crate::scheduler::SchedulerEvent::CheckError {
                                                                alert_id: alert.id,
                                                                error: format!("{e}"),
                                                            }).await;
                                                    }
                                                }
                                            });
                                        }
                                    }
                                    crate::tui::tabs::alerts::AlertsAction::ClearAlertResults(
                                        idx,
                                    ) => {
                                        if let Some(alert) = self.config.alerts.get(idx) {
                                            let alert_id = alert.id;
                                            self.results.retain(|r| r.alert_id != alert_id);
                                            let _ = crate::daemon::results::save_results(
                                                &self.results,
                                                &self.results_path,
                                            );
                                            log::info!(target: "snag::app", "Cleared results for alert '{}'", alert.name);
                                            if self.results_tab.selected
                                                >= self.results.len().max(1) - 1
                                            {
                                                self.results_tab.selected = 0;
                                                self.results_tab.list_state.select(Some(0));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        TabKind::Results => {
                            if let Some(action) = self.results_tab.handle_key(
                                key,
                                &mut self.results,
                                &mut self.seen_ids,
                            ) {
                                match action {
                                    crate::tui::tabs::results::ResultsAction::OpenUrl(url) => {
                                        let _ = open::that(&url);
                                        let _ = crate::daemon::results::save_seen(
                                            &self.seen_ids,
                                            &self.seen_path,
                                        );
                                    }
                                    crate::tui::tabs::results::ResultsAction::ResultsChanged => {
                                        let _ = crate::daemon::results::save_results(
                                            &self.results,
                                            &self.results_path,
                                        );
                                    }
                                    crate::tui::tabs::results::ResultsAction::SeenChanged => {
                                        let _ = crate::daemon::results::save_seen(
                                            &self.seen_ids,
                                            &self.seen_path,
                                        );
                                    }
                                    crate::tui::tabs::results::ResultsAction::ViewListing(
                                        listing,
                                        alert_name,
                                    ) => {
                                        self.seen_ids.insert(listing.id.clone());
                                        let _ = crate::daemon::results::save_seen(
                                            &self.seen_ids,
                                            &self.seen_path,
                                        );
                                        let dialog = crate::tui::dialogs::listing_detail::ListingDetailDialog::new(
                                                *listing,
                                                alert_name,
                                            );
                                        self.active_dialog =
                                            Some(ActiveDialog::ListingDetail(dialog));
                                    }
                                }
                            }
                        }
                        TabKind::Settings => {
                            if let Some(action) =
                                self.settings_tab.handle_key(key, &mut self.config)
                            {
                                match action {
                                        crate::tui::tabs::settings::SettingsAction::ConfigChanged => {
                                            let _ = save_config(&self.config, &self.config_path);
                                            if let Some(ref tx) = self.config_tx {
                                                let _ = tx.send(self.config.clone());
                                            }
                                        }
                                        crate::tui::tabs::settings::SettingsAction::SetupMarketplace(
                                            crate::tui::tabs::settings::MarketplaceSetup::Ebay,
                                        ) => {
                                            self.active_dialog = Some(ActiveDialog::EbaySetup(
                                                crate::tui::dialogs::ebay_setup::EbaySetupDialog::new(),
                                            ));
                                        }
                                    }
                            }
                        }
                        TabKind::Logs => {
                            self.logs_tab.handle_key(key);
                        }
                    }
                }
            }

            if let Some(ref mut rx) = self.scheduler_rx {
                while let Ok(event) = rx.try_recv() {
                    match event {
                        crate::scheduler::SchedulerEvent::CheckComplete { status, result } => {
                            log::debug!(target: "snag::app", "CheckComplete for alert {}: {} new results", status.alert_id, status.new_results);
                            crate::daemon::results::upsert_status(&mut self.statuses, status);
                            if let Some(alert_result) = result {
                                self.results.push(alert_result);
                            }
                            if let Err(e) = crate::daemon::results::save_results(
                                &self.results,
                                &self.results_path,
                            ) {
                                log::error!(target: "snag::app", "failed to save results: {e}");
                            }
                            if let Err(e) = crate::daemon::results::save_status(
                                &self.statuses,
                                &self.status_path,
                            ) {
                                log::error!(target: "snag::app", "failed to save status: {e}");
                            }
                        }
                        crate::scheduler::SchedulerEvent::CheckError { alert_id, error } => {
                            log::debug!(target: "snag::app", "CheckError for alert {}: {}", alert_id, error);
                            crate::daemon::results::upsert_status(
                                &mut self.statuses,
                                crate::types::CheckStatus {
                                    alert_id,
                                    checked_at: chrono::Utc::now(),
                                    new_results: 0,
                                    error: Some(error),
                                },
                            );
                            if let Err(e) = crate::daemon::results::save_status(
                                &self.statuses,
                                &self.status_path,
                            ) {
                                log::error!(target: "snag::app", "failed to save status: {e}");
                            }
                        }
                    }
                }
            } else if last_results_refresh.elapsed() >= results_refresh_interval {
                let results_mtime = std::fs::metadata(&self.results_path)
                    .and_then(|m| m.modified())
                    .ok();
                if results_mtime != self.last_results_mtime {
                    if let Ok(new_results) = load_results(&self.results_path) {
                        self.results = new_results;
                    }
                    self.last_results_mtime = results_mtime;
                    if self.results_tab.selected >= self.results.len().max(1) - 1 {
                        self.results_tab.selected = 0;
                        self.results_tab.list_state.select(Some(0));
                    }
                }

                let status_mtime = std::fs::metadata(&self.status_path)
                    .and_then(|m| m.modified())
                    .ok();
                if status_mtime != self.last_status_mtime {
                    if let Ok(new_statuses) = crate::daemon::results::load_status(&self.status_path)
                    {
                        self.statuses = new_statuses;
                    }
                    self.last_status_mtime = status_mtime;
                }

                let seen_mtime = std::fs::metadata(&self.seen_path)
                    .and_then(|m| m.modified())
                    .ok();
                if seen_mtime != self.last_seen_mtime {
                    if let Ok(new_seen) = crate::daemon::results::load_seen(&self.seen_path) {
                        self.seen_ids = new_seen;
                    }
                    self.last_seen_mtime = seen_mtime;
                }

                last_results_refresh = Instant::now();
            }

            if let Some(ref mut rx) = self.update_rx
                && let Ok(result) = rx.try_recv()
            {
                if let Some(info) = result {
                    self.update_info = Some(info);
                }
                self.update_rx = None;
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
                        if let Some(pos) = self.config.alerts.iter().position(|a| a.id == alert.id)
                        {
                            self.config.alerts[pos] = alert;
                        } else {
                            self.config.alerts.push(alert);
                        }
                        let _ = save_config(&self.config, &self.config_path);
                        if let Some(ref tx) = self.config_tx {
                            let _ = tx.send(self.config.clone());
                        }
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
            Some(ActiveDialog::ListingDetail(dialog)) => {
                let listing_id = dialog.listing.id.clone();
                let r = dialog.handle_key(key);
                match r {
                    DialogResult::Cancel => Some(DialogResult::<()>::Cancel),
                    DialogResult::Continue => None,
                    DialogResult::Submit(action) => {
                        match action {
                            crate::tui::dialogs::listing_detail::ListingDetailAction::OpenUrl(
                                url,
                            ) => {
                                let _ = open::that(&url);
                                self.seen_ids.insert(listing_id);
                                let _ = crate::daemon::results::save_seen(
                                    &self.seen_ids,
                                    &self.seen_path,
                                );
                            }
                        }
                        Some(DialogResult::<()>::Cancel)
                    }
                }
            }
            Some(ActiveDialog::EbaySetup(dialog)) => {
                let r = dialog.handle_key(key);
                match r {
                    DialogResult::Cancel => Some(DialogResult::<()>::Cancel),
                    DialogResult::Continue => None,
                    DialogResult::Submit(action) => match action {
                        crate::tui::dialogs::ebay_setup::EbaySetupAction::OpenRegistration => {
                            let _ = open::that("https://developer.ebay.com/signin?tab=register");
                            None
                        }
                        crate::tui::dialogs::ebay_setup::EbaySetupAction::SaveCredentials {
                            client_id,
                            client_secret,
                        } => {
                            if let Err(e) =
                                crate::credentials::store_credential("ebay_client_id", &client_id)
                            {
                                log::error!(target: "snag::app", "Failed to store eBay client ID: {e}");
                            }
                            if let Err(e) = crate::credentials::store_credential(
                                "ebay_client_secret",
                                &client_secret,
                            ) {
                                log::error!(target: "snag::app", "Failed to store eBay client secret: {e}");
                            }
                            log::info!(target: "snag::app", "eBay API credentials saved to keyring");
                            Some(DialogResult::<()>::Cancel)
                        }
                    },
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
                                if let Some(ref tx) = self.config_tx {
                                    let _ = tx.send(self.config.clone());
                                }
                                if self.alerts_tab.selected >= self.config.alerts.len()
                                    && self.alerts_tab.selected > 0
                                {
                                    self.alerts_tab.selected -= 1;
                                    self.alerts_tab
                                        .list_state
                                        .select(Some(self.alerts_tab.selected));
                                }
                                self.alerts_tab.listing_focus = false;
                                self.alerts_tab.listing_selected = 0;
                            }
                        }
                        ConfirmAction::ClearResults => {
                            self.results.clear();
                            let _ = crate::daemon::results::save_results(
                                &self.results,
                                &self.results_path,
                            );
                        }
                        ConfirmAction::PerformUpdate => {
                            self.pending_update = self.update_info.clone();
                            self.should_quit = true;
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

    fn render(&mut self, frame: &mut Frame) {
        let has_update = self.update_info.is_some();

        let chunks = if has_update {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(frame.area())
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ])
                .split(frame.area())
        };

        self.render_tabs(frame, chunks[0]);

        match self.active_tab {
            TabKind::Alerts => self.alerts_tab.render(
                frame,
                chunks[1],
                &self.theme,
                &self.config,
                &self.statuses,
                &self.results,
                &self.seen_ids,
            ),
            TabKind::Results => self.results_tab.render(
                frame,
                chunks[1],
                &self.theme,
                &self.results,
                &self.seen_ids,
            ),
            TabKind::Settings => {
                self.settings_tab
                    .render(frame, chunks[1], &self.theme, &self.config)
            }
            TabKind::Logs => self.logs_tab.render(frame, chunks[1], &self.theme),
        }

        if has_update {
            self.render_update_bar(frame, chunks[2]);
            self.render_status_bar(frame, chunks[3]);
        } else {
            self.render_status_bar(frame, chunks[2]);
        }

        // Draw dialogs on top of normal content
        if let Some(dialog) = &self.active_dialog {
            match dialog {
                ActiveDialog::AlertForm(d) => d.render(frame, frame.area(), &self.theme),
                ActiveDialog::Confirm(d, _) => d.render(frame, frame.area(), &self.theme),
                ActiveDialog::ListingDetail(d) => d.render(frame, frame.area(), &self.theme),
                ActiveDialog::EbaySetup(d) => d.render(frame, frame.area(), &self.theme),
            }
        }
    }

    fn render_update_bar(&self, frame: &mut Frame, area: Rect) {
        if let Some(ref info) = self.update_info {
            let text = format!(
                " Update available: {} → {}  Press u to update",
                env!("CARGO_PKG_VERSION"),
                info.latest_version
            );
            let bar = Paragraph::new(Line::from(Span::styled(
                text,
                Style::default()
                    .fg(self.theme.unread)
                    .add_modifier(Modifier::BOLD),
            )))
            .style(Style::default().bg(self.theme.selected_bg));
            frame.render_widget(bar, area);
        }
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let mut spans = vec![
            Span::styled(
                " snag ",
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default()),
        ];

        for (i, tab) in TabKind::all().iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ", Style::default()));
            }
            if *tab == self.active_tab {
                spans.push(Span::styled("[ ", Style::default().fg(self.theme.border)));
                spans.push(Span::styled(
                    tab.title(),
                    Style::default()
                        .fg(self.theme.accent)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(" ]", Style::default().fg(self.theme.border)));
            } else {
                spans.push(Span::styled("  ", Style::default()));
                spans.push(Span::styled(
                    tab.title(),
                    Style::default().fg(self.theme.fg_dim),
                ));
                spans.push(Span::styled("  ", Style::default()));
            }
        }

        let tabs_line = Paragraph::new(Line::from(spans));
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(self.theme.border))
            .border_type(BorderType::Rounded);
        frame.render_widget(tabs_line.block(block), area);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let mut hints: Vec<(&str, &str)> = match self.active_tab {
            TabKind::Alerts => {
                if self.alerts_tab.listing_focus {
                    vec![
                        (" Enter ", "Details"),
                        (" c ", "Clear"),
                        (" Esc ", "Back"),
                        (" q ", "Quit"),
                    ]
                } else {
                    vec![
                        (" n ", "New"),
                        (" e ", "Edit"),
                        (" d ", "Delete"),
                        (" f ", "Force"),
                        (" Enter/l ", "Listings"),
                        (" ␣ ", "Toggle"),
                        (" q ", "Quit"),
                    ]
                }
            }
            TabKind::Results => vec![
                (" Enter ", "Details"),
                (" o ", "Open"),
                (" m ", "Mark read"),
                (" c ", "Clear"),
                (" q ", "Quit"),
            ],
            TabKind::Settings => vec![(" Enter ", "Edit"), (" ↑↓ ", "Navigate"), (" q ", "Quit")],
            TabKind::Logs => vec![
                (" ↑↓ ", "Scroll"),
                (" ←→ ", "Level"),
                (" Enter ", "Focus"),
                (" Esc ", "Back"),
                (" q ", "Quit"),
            ],
        };

        if self.update_info.is_some() {
            hints.push((" u ", "Update"));
        }

        let mut spans = vec![
            Span::styled(
                " Tab/1-4 ",
                Style::default()
                    .fg(self.theme.status_bar_fg)
                    .bg(self.theme.accent),
            ),
            Span::styled(" ", Style::default().bg(self.theme.status_bar_bg)),
        ];

        for (i, (key, desc)) in hints.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(
                    " │ ",
                    Style::default()
                        .fg(self.theme.border)
                        .bg(self.theme.status_bar_bg),
                ));
            }
            spans.push(Span::styled(
                *key,
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
                    .bg(self.theme.status_bar_bg),
            ));
            spans.push(Span::styled(
                *desc,
                Style::default()
                    .fg(self.theme.fg_dim)
                    .bg(self.theme.status_bar_bg),
            ));
        }

        let bar = Paragraph::new(Line::from(spans));
        frame.render_widget(bar, area);
    }
}
