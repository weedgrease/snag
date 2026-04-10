pub mod alerts;
pub mod results;
pub mod settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Alerts,
    Results,
    Settings,
}

impl TabKind {
    pub fn all() -> &'static [TabKind] {
        &[TabKind::Alerts, TabKind::Results, TabKind::Settings]
    }

    pub fn title(&self) -> &str {
        match self {
            TabKind::Alerts => "Alerts",
            TabKind::Results => "Results",
            TabKind::Settings => "Settings",
        }
    }

    pub fn next(&self) -> TabKind {
        match self {
            TabKind::Alerts => TabKind::Results,
            TabKind::Results => TabKind::Settings,
            TabKind::Settings => TabKind::Alerts,
        }
    }

    pub fn prev(&self) -> TabKind {
        match self {
            TabKind::Alerts => TabKind::Settings,
            TabKind::Results => TabKind::Alerts,
            TabKind::Settings => TabKind::Results,
        }
    }
}
