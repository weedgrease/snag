use crate::marketplace::Marketplace;
use crate::types::{Alert, FilterKind, Listing, MarketplaceKind};
use anyhow::Result;
use async_trait::async_trait;

pub struct EbayMarketplace;

impl EbayMarketplace {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Marketplace for EbayMarketplace {
    fn name(&self) -> &str {
        "eBay"
    }

    fn kind(&self) -> MarketplaceKind {
        MarketplaceKind::Ebay
    }

    fn supported_filters(&self) -> &[FilterKind] {
        &[
            FilterKind::PriceRange,
            FilterKind::Condition,
            FilterKind::Category,
            FilterKind::Location,
        ]
    }

    async fn search(&self, _alert: &Alert) -> Result<Vec<Listing>> {
        tracing::warn!("eBay search not yet implemented, returning empty results");
        Ok(vec![])
    }
}
