use crate::marketplace::Marketplace;
use crate::types::{Alert, FilterKind, Listing, MarketplaceKind};
use anyhow::Result;
use async_trait::async_trait;

pub struct FacebookMarketplace;

impl FacebookMarketplace {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FacebookMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Marketplace for FacebookMarketplace {
    fn name(&self) -> &str {
        "Facebook Marketplace"
    }

    fn kind(&self) -> MarketplaceKind {
        MarketplaceKind::FacebookMarketplace
    }

    fn supported_filters(&self) -> &[FilterKind] {
        &[
            FilterKind::PriceRange,
            FilterKind::Location,
            FilterKind::Category,
        ]
    }

    async fn search(&self, _alert: &Alert) -> Result<Vec<Listing>> {
        tracing::warn!("Facebook Marketplace search not yet implemented, returning empty results");
        Ok(vec![])
    }
}
