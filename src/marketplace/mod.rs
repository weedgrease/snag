pub mod providers;
pub mod rate_limit;

use crate::types::{Alert, FilterKind, Listing, MarketplaceKind};
use anyhow::Result;
use async_trait::async_trait;

/// Common interface that each marketplace provider must implement.
#[async_trait]
pub trait Marketplace: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> MarketplaceKind;
    fn supported_filters(&self) -> &[FilterKind];
    /// Searches the marketplace using the alert's criteria.
    /// `default_location` is used as a fallback when `alert.location` is `None`.
    async fn search(&self, alert: &Alert, default_location: Option<&str>) -> Result<Vec<Listing>>;
}

pub fn create_marketplace(kind: MarketplaceKind) -> Box<dyn Marketplace> {
    match kind {
        MarketplaceKind::Ebay => Box::new(providers::ebay::EbayMarketplace::new()),
        MarketplaceKind::FacebookMarketplace => {
            Box::new(providers::facebook::FacebookMarketplace::new())
        }
    }
}
