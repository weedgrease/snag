pub mod providers;

use crate::types::{Alert, Listing, NotifierKind};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Notifier: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> NotifierKind;
    async fn notify(&self, alert: &Alert, listings: &[Listing]) -> Result<()>;
}

pub fn create_notifier(kind: NotifierKind) -> Box<dyn Notifier> {
    match kind {
        NotifierKind::Terminal => Box::new(providers::terminal::TerminalNotifier::new()),
    }
}
