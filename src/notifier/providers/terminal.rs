use crate::notifier::Notifier;
use crate::types::{Alert, Listing, NotifierKind};
use anyhow::Result;
use async_trait::async_trait;

pub struct TerminalNotifier;

impl TerminalNotifier {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TerminalNotifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Notifier for TerminalNotifier {
    fn name(&self) -> &str {
        "Terminal"
    }

    fn kind(&self) -> NotifierKind {
        NotifierKind::Terminal
    }

    async fn notify(&self, alert: &Alert, listings: &[Listing]) -> Result<()> {
        log::info!(target: "snag::notifier", "Notifying: {} new listings for '{}'", listings.len(), alert.name);
        for listing in listings {
            let price_str = listing
                .price
                .map(|p| format!(" — ${:.2}", p))
                .unwrap_or_default();

            log::info!(target: "snag::notifier",
                "[{}] New match: {}{}  {}",
                alert.name,
                listing.title,
                price_str,
                listing.url,
            );
        }
        Ok(())
    }
}
