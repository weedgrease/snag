use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// A user-defined search alert with keywords, filters, and scheduling settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub name: String,
    pub marketplaces: Vec<MarketplaceKind>,
    pub keywords: Vec<String>,
    pub exclude_keywords: Vec<String>,
    pub price_min: Option<f64>,
    pub price_max: Option<f64>,
    pub location: Option<String>,
    pub radius_miles: Option<u32>,
    pub condition: Option<Condition>,
    pub category: Option<String>,
    #[serde(with = "duration_secs")]
    pub check_interval: Duration,
    pub notifiers: Vec<NotifierKind>,
    pub max_results: Option<u32>,
    pub enabled: bool,
}

/// A single marketplace listing returned by a search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing {
    pub id: String,
    pub title: String,
    pub price: Option<f64>,
    pub currency: String,
    pub url: String,
    pub image_url: Option<String>,
    pub location: Option<String>,
    pub condition: Option<Condition>,
    pub marketplace: MarketplaceKind,
    pub posted_at: Option<DateTime<Utc>>,
    pub found_at: DateTime<Utc>,
    pub description: Option<String>,
}

/// The outcome of a single alert check: new listings found and whether the user has seen them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertResult {
    pub alert_id: Uuid,
    pub alert_name: String,
    pub listings: Vec<Listing>,
    pub checked_at: DateTime<Utc>,
    pub seen: bool,
}

/// Lightweight summary of the most recent check for an alert, including any error message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckStatus {
    pub alert_id: Uuid,
    pub checked_at: DateTime<Utc>,
    pub new_results: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketplaceKind {
    Ebay,
    FacebookMarketplace,
}

impl std::fmt::Display for MarketplaceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ebay => write!(f, "eBay"),
            Self::FacebookMarketplace => write!(f, "Facebook Marketplace"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotifierKind {
    Terminal,
}

impl std::fmt::Display for NotifierKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Terminal => write!(f, "Terminal"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    New,
    LikeNew,
    Used,
    ForParts,
}

impl std::fmt::Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New => write!(f, "New"),
            Self::LikeNew => write!(f, "Like New"),
            Self::Used => write!(f, "Used"),
            Self::ForParts => write!(f, "For Parts"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterKind {
    PriceRange,
    Location,
    Condition,
    Category,
}

/// Serde helper that serializes `Duration` as whole seconds so it round-trips cleanly through TOML/JSON.
pub mod duration_secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
