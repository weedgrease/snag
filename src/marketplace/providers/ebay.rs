use crate::credentials;
use crate::marketplace::Marketplace;
use crate::types::{Alert, Condition, FilterKind, Listing, MarketplaceKind};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const TOKEN_URL: &str = "https://api.ebay.com/identity/v1/oauth2/token";
const SEARCH_URL: &str = "https://api.ebay.com/buy/browse/v1/item_summary/search";
const OAUTH_SCOPE: &str = "https://api.ebay.com/oauth/api_scope";

struct TokenCache {
    access_token: String,
    expires_at: Instant,
}

pub struct EbayMarketplace {
    client: reqwest::Client,
    token_cache: Mutex<Option<TokenCache>>,
}

impl Default for EbayMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

impl EbayMarketplace {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("snag/{}", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            token_cache: Mutex::new(None),
        }
    }

    async fn get_access_token(&self) -> Result<String> {
        {
            let cache = self.token_cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref cached) = *cache
                && Instant::now() < cached.expires_at
            {
                return Ok(cached.access_token.clone());
            }
        }

        let client_id = credentials::get_credential("ebay_client_id")?
            .context("eBay Client ID not configured")?;
        let client_secret = credentials::get_credential("ebay_client_secret")?
            .context("eBay Client Secret not configured")?;

        use base64::Engine;
        let credentials = base64::engine::general_purpose::STANDARD
            .encode(format!("{client_id}:{client_secret}"));

        log::debug!(target: "snag::ebay", "Requesting OAuth token");

        let response = self
            .client
            .post(TOKEN_URL)
            .header("Authorization", format!("Basic {credentials}"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!("grant_type=client_credentials&scope={OAUTH_SCOPE}"))
            .send()
            .await
            .context("failed to request eBay OAuth token")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            log::error!(target: "snag::ebay", "OAuth token request failed ({}): {}", status, body);
            anyhow::bail!("eBay OAuth failed ({}): {}", status, body);
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: u64,
        }

        let token_resp: TokenResponse = response
            .json()
            .await
            .context("failed to parse OAuth token response")?;

        log::debug!(target: "snag::ebay", "OAuth token obtained, expires in {}s", token_resp.expires_in);

        let expires_at =
            Instant::now() + Duration::from_secs(token_resp.expires_in.saturating_sub(60));

        let mut cache = self.token_cache.lock().unwrap_or_else(|e| e.into_inner());
        *cache = Some(TokenCache {
            access_token: token_resp.access_token.clone(),
            expires_at,
        });

        Ok(token_resp.access_token)
    }

    fn build_filter(&self, alert: &Alert) -> String {
        let mut filters = vec![];

        match (alert.price_min, alert.price_max) {
            (Some(min), Some(max)) => {
                filters.push(format!("price:[{min}..{max}],priceCurrency:USD"));
            }
            (Some(min), None) => {
                filters.push(format!("price:[{min}..],priceCurrency:USD"));
            }
            (None, Some(max)) => {
                filters.push(format!("price:[..{max}],priceCurrency:USD"));
            }
            (None, None) => {}
        }

        if let Some(ref condition) = alert.condition {
            let ebay_condition = match condition {
                Condition::New | Condition::LikeNew => "NEW",
                Condition::Used => "USED",
                Condition::ForParts => "FOR_PARTS_OR_NOT_WORKING",
            };
            filters.push(format!("conditions:{{{ebay_condition}}}"));
        }

        filters.join(",")
    }
}

#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default, rename = "itemSummaries")]
    item_summaries: Vec<ItemSummary>,
}

#[derive(Deserialize)]
struct ItemSummary {
    #[serde(rename = "itemId")]
    item_id: Option<String>,
    title: Option<String>,
    price: Option<Price>,
    condition: Option<String>,
    #[serde(rename = "itemWebUrl")]
    item_web_url: Option<String>,
    #[serde(rename = "itemLocation")]
    item_location: Option<ItemLocation>,
    image: Option<Image>,
}

#[derive(Deserialize)]
struct Price {
    value: Option<String>,
    currency: Option<String>,
}

#[derive(Deserialize)]
struct ItemLocation {
    city: Option<String>,
    #[serde(rename = "stateOrProvince")]
    state_or_province: Option<String>,
}

#[derive(Deserialize)]
struct Image {
    #[serde(rename = "imageUrl")]
    image_url: Option<String>,
}

pub async fn fetch_item_description(item_id: &str) -> anyhow::Result<Option<String>> {
    if !crate::credentials::ebay_credentials_configured() {
        return Ok(None);
    }

    let provider = EbayMarketplace::new();
    let token = provider.get_access_token().await?;

    let url = format!("https://api.ebay.com/buy/browse/v1/item/{}", item_id);
    let response = provider
        .client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("X-EBAY-C-MARKETPLACE-ID", "EBAY_US")
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(None);
    }

    #[derive(serde::Deserialize)]
    struct ItemDetail {
        description: Option<String>,
        #[serde(rename = "shortDescription")]
        short_description: Option<String>,
    }

    let detail: ItemDetail = response.json().await?;
    Ok(detail.description.or(detail.short_description))
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

    async fn search(&self, alert: &Alert, _default_location: Option<&str>) -> Result<Vec<Listing>> {
        if !credentials::ebay_credentials_configured() {
            anyhow::bail!("eBay not configured — press [e] in Settings to set up API credentials");
        }

        let token = self.get_access_token().await?;

        let keywords = alert.keywords.join(" ");
        let limit = alert.max_results.unwrap_or(50).min(200);
        let filter = self.build_filter(alert);

        log::info!(target: "snag::ebay", "Searching eBay: '{}'", keywords);

        let mut request = self
            .client
            .get(SEARCH_URL)
            .header("Authorization", format!("Bearer {token}"))
            .header("X-EBAY-C-MARKETPLACE-ID", "EBAY_US")
            .query(&[("q", keywords.as_str()), ("limit", &limit.to_string())]);

        if !filter.is_empty() {
            request = request.query(&[("filter", filter.as_str())]);
        }

        let response = request.send().await.context("failed to search eBay")?;

        let status = response.status();
        log::debug!(target: "snag::ebay", "Search API status: {}", status);

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            log::error!(target: "snag::ebay", "Search failed ({}): {}", status,
                body.char_indices().nth(500).map(|(i, _)| &body[..i]).unwrap_or(&body));
            anyhow::bail!("eBay search failed ({})", status);
        }

        let body: SearchResponse = response
            .json()
            .await
            .context("failed to parse eBay search response")?;

        log::info!(target: "snag::ebay", "eBay returned {} listings", body.item_summaries.len());

        let now = Utc::now();
        let exclude_lower: Vec<String> = alert
            .exclude_keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();

        let mut listings = vec![];

        for item in body.item_summaries {
            let id = match item.item_id {
                Some(id) => id,
                None => continue,
            };

            let title = item.title.unwrap_or_default();

            let title_lower = title.to_lowercase();
            if exclude_lower
                .iter()
                .any(|kw| title_lower.contains(kw.as_str()))
            {
                continue;
            }

            let price = item
                .price
                .as_ref()
                .and_then(|p| p.value.as_ref())
                .and_then(|v| v.parse::<f64>().ok());

            let currency = item
                .price
                .as_ref()
                .and_then(|p| p.currency.clone())
                .unwrap_or_else(|| "USD".into());

            let url = item
                .item_web_url
                .unwrap_or_else(|| format!("https://www.ebay.com/itm/{id}"));

            let image_url = item.image.and_then(|i| i.image_url);

            let location = item
                .item_location
                .map(|loc| {
                    let mut parts = vec![];
                    if let Some(city) = loc.city {
                        parts.push(city);
                    }
                    if let Some(state) = loc.state_or_province {
                        parts.push(state);
                    }
                    parts.join(", ")
                })
                .filter(|s| !s.is_empty());

            let condition = item.condition.as_deref().and_then(|c| match c {
                "New" => Some(Condition::New),
                "Used" => Some(Condition::Used),
                "Refurbished" | "Certified Refurbished" => Some(Condition::LikeNew),
                "For parts or not working" => Some(Condition::ForParts),
                _ => None,
            });

            listings.push(Listing {
                id,
                title,
                price,
                currency,
                url,
                image_url,
                location,
                condition,
                marketplace: MarketplaceKind::Ebay,
                posted_at: None,
                found_at: now,
                description: None,
            });
        }

        Ok(listings)
    }
}
