use crate::marketplace::Marketplace;
use crate::types::{Alert, FilterKind, Listing, MarketplaceKind};
use anyhow::Result;
use async_trait::async_trait;

pub struct EbayMarketplace;

impl Default for EbayMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

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

    async fn search(&self, _alert: &Alert, _default_location: Option<&str>) -> Result<Vec<Listing>> {
        if !crate::credentials::ebay_credentials_configured() {
            anyhow::bail!("eBay not configured — open Settings to set up API credentials");
        }

        // Real implementation comes in next task
        anyhow::bail!("eBay search not yet implemented")
    }
}
