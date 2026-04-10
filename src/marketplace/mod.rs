pub mod providers;

use crate::types::{Alert, FilterKind, Listing, MarketplaceKind};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Marketplace: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> MarketplaceKind;
    fn supported_filters(&self) -> &[FilterKind];
    async fn search(&self, alert: &Alert) -> Result<Vec<Listing>>;
}

pub fn create_marketplace(kind: MarketplaceKind) -> Box<dyn Marketplace> {
    match kind {
        MarketplaceKind::Ebay => Box::new(providers::ebay::EbayMarketplace::new()),
        MarketplaceKind::FacebookMarketplace => {
            Box::new(providers::facebook::FacebookMarketplace::new())
        }
    }
}
