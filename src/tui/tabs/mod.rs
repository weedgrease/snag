pub mod alerts;
pub mod logs;
pub mod results;
pub mod settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Alerts,
    Results,
    Settings,
    Logs,
}

impl TabKind {
    pub fn all() -> &'static [TabKind] {
        &[TabKind::Alerts, TabKind::Results, TabKind::Settings, TabKind::Logs]
    }

    pub fn title(&self) -> &str {
        match self {
            TabKind::Alerts => "Alerts",
            TabKind::Results => "Results",
            TabKind::Settings => "Settings",
            TabKind::Logs => "Logs",
        }
    }

    pub fn next(&self) -> TabKind {
        match self {
            TabKind::Alerts => TabKind::Results,
            TabKind::Results => TabKind::Settings,
            TabKind::Settings => TabKind::Logs,
            TabKind::Logs => TabKind::Alerts,
        }
    }

    pub fn prev(&self) -> TabKind {
        match self {
            TabKind::Alerts => TabKind::Logs,
            TabKind::Results => TabKind::Alerts,
            TabKind::Settings => TabKind::Results,
            TabKind::Logs => TabKind::Settings,
        }
    }
}
