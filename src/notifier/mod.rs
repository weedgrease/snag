pub mod providers;

use crate::types::{Alert, Listing, LogEntry, NotifierKind};
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

#[async_trait]
pub trait Notifier: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> NotifierKind;
    async fn notify(
        &self,
        alert: &Alert,
        listings: &[Listing],
        log_tx: &mpsc::Sender<LogEntry>,
    ) -> Result<()>;
}

pub fn create_notifier(kind: NotifierKind) -> Box<dyn Notifier> {
    match kind {
        NotifierKind::Terminal => Box::new(providers::terminal::TerminalNotifier::new()),
    }
}
