pub mod providers;

use crate::types::{Alert, FilterKind, Listing, LogEntry, MarketplaceKind};
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

#[async_trait]
pub trait Marketplace: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> MarketplaceKind;
    fn supported_filters(&self) -> &[FilterKind];
    async fn search(
        &self,
        alert: &Alert,
        default_location: Option<&str>,
        log_tx: &mpsc::Sender<LogEntry>,
    ) -> Result<Vec<Listing>>;
}

pub fn create_marketplace(kind: MarketplaceKind) -> Box<dyn Marketplace> {
    match kind {
        MarketplaceKind::FacebookMarketplace => {
            Box::new(providers::facebook::FacebookMarketplace::new())
        }
    }
}
