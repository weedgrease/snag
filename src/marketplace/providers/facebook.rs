use crate::marketplace::Marketplace;
use crate::types::{Alert, Condition, FilterKind, Listing, MarketplaceKind};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

const GRAPHQL_URL: &str = "https://www.facebook.com/api/graphql/";
const LOCATION_DOC_ID: &str = "5585904654783609";
const SEARCH_DOC_ID: &str = "7111939778879383";
use crate::marketplace::rate_limit;

const RATE_LIMIT_BACKOFF: Duration = Duration::from_secs(3600);
const MARKETPLACE_ID: &str = "facebook";

/// Maximum price value accepted by Facebook's GraphQL API (in cents).
const FB_MAX_PRICE_CENTS: i64 = 214_748_364_700;

/// Facebook error code for rate limiting.
const FB_RATE_LIMIT_CODE: u64 = 1_675_004;

pub struct FacebookMarketplace {
    client: reqwest::Client,
    location_cache: Mutex<HashMap<String, (f64, f64)>>,
}

impl Default for FacebookMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

impl FacebookMarketplace {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            location_cache: Mutex::new(HashMap::new()),
        }
    }

    async fn resolve_location(&self, query: &str) -> Result<(f64, f64)> {
        if let Some(cached) = self
            .location_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(query)
        {
            log::debug!(target: "snag::facebook", "Location cache hit: {}", query);
            return Ok(*cached);
        }

        log::info!(target: "snag::facebook", "Resolving location: {}", query);

        let variables = serde_json::json!({
            "params": {
                "caller": "MARKETPLACE",
                "page_category": ["CITY", "SUBCITY", "NEIGHBORHOOD", "POSTAL_CODE"],
                "query": query
            }
        });

        let response = self
            .client
            .post(GRAPHQL_URL)
            .header("sec-fetch-site", "same-origin")
            .form(&[
                ("variables", serde_json::to_string(&variables)?),
                ("doc_id", LOCATION_DOC_ID.to_string()),
            ])
            .send()
            .await
            .context("failed to fetch location")?;

        let status = response.status();
        log::debug!(target: "snag::facebook", "Location API status: {}", status);

        let body_text = response
            .text()
            .await
            .context("failed to read location response body")?;
        log::debug!(target: "snag::facebook", "Location API response ({} bytes): {}",
            body_text.len(),
            body_text.char_indices().nth(500).map(|(i, _)| &body_text[..i]).unwrap_or(&body_text)
        );

        let body: serde_json::Value = serde_json::from_str(&body_text)
            .context("failed to parse location response as JSON")?;

        let locations = body
            .pointer("/data/city_street_search/street_results/edges")
            .and_then(|v| v.as_array())
            .context(format!(
                "unexpected location response structure. Keys: {:?}",
                body.as_object().map(|o| o.keys().collect::<Vec<_>>())
            ))?;

        let node = locations
            .first()
            .and_then(|edge| edge.get("node"))
            .context("no locations found")?;

        let location = node
            .get("location")
            .context("missing location object on node")?;

        let lat = location
            .get("latitude")
            .and_then(|v| v.as_f64())
            .context("missing latitude")?;

        let lng = location
            .get("longitude")
            .and_then(|v| v.as_f64())
            .context("missing longitude")?;

        self.location_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(query.to_string(), (lat, lng));

        log::debug!(target: "snag::facebook", "Location resolved: {} -> ({}, {})", query, lat, lng);

        Ok((lat, lng))
    }

    fn build_search_variables(&self, alert: &Alert, lat: f64, lng: f64) -> serde_json::Value {
        let radius_km = alert
            .radius_miles
            .map(|m| (m as f64 * 1.60934) as i64)
            .unwrap_or(16);

        let price_lower = alert.price_min.map(|p| (p * 100.0) as i64).unwrap_or(0);
        let price_upper = alert
            .price_max
            .map(|p| (p * 100.0) as i64)
            .unwrap_or(FB_MAX_PRICE_CENTS);

        let condition: serde_json::Value = alert
            .condition
            .map(|c| {
                serde_json::Value::String(match c {
                    Condition::New => "new".into(),
                    Condition::LikeNew => "used_like_new".into(),
                    Condition::Used => "used_good".into(),
                    Condition::ForParts => "used_fair".into(),
                })
            })
            .unwrap_or(serde_json::Value::Null);

        let keywords = alert.keywords.join(" ");
        let count = alert.max_results.unwrap_or(24).min(100);

        serde_json::json!({
            "count": count,
            "params": {
                "bqf": {
                    "callsite": "COMMERCE_MKTPLACE_WWW",
                    "query": keywords
                },
                "browse_request_params": {
                    "commerce_enable_local_pickup": true,
                    "commerce_enable_shipping": true,
                    "commerce_search_and_rp_available": true,
                    "commerce_search_and_rp_condition": condition,
                    "commerce_search_and_rp_ctime_days": null,
                    "filter_location_latitude": lat,
                    "filter_location_longitude": lng,
                    "filter_price_lower_bound": price_lower,
                    "filter_price_upper_bound": price_upper,
                    "filter_radius_km": radius_km
                },
                "custom_request_params": {
                    "surface": "SEARCH"
                }
            }
        })
    }
}

#[derive(Deserialize)]
struct SearchResponse {
    data: Option<SearchData>,
}

#[derive(Deserialize)]
struct SearchData {
    marketplace_search: Option<MarketplaceSearch>,
}

#[derive(Deserialize)]
struct MarketplaceSearch {
    feed_units: Option<FeedUnits>,
}

#[derive(Deserialize)]
struct FeedUnits {
    edges: Vec<FeedEdge>,
}

#[derive(Deserialize)]
struct FeedEdge {
    node: Option<FeedNode>,
}

#[derive(Deserialize)]
struct FeedNode {
    listing: Option<ListingNode>,
}

#[derive(Deserialize)]
struct ListingNode {
    id: Option<String>,
    marketplace_listing_title: Option<String>,
    listing_price: Option<ListingPrice>,
    primary_listing_photo: Option<PrimaryPhoto>,
    location: Option<LocationNode>,
}

#[derive(Debug, Deserialize)]
struct ListingPrice {
    amount: Option<String>,
    currency: Option<String>,
    formatted_amount: Option<String>,
}

#[derive(Deserialize)]
struct PrimaryPhoto {
    image: Option<ImageNode>,
}

#[derive(Deserialize)]
struct ImageNode {
    uri: Option<String>,
}

#[derive(Deserialize)]
struct LocationNode {
    reverse_geocode: Option<ReverseGeocode>,
}

#[derive(Deserialize)]
struct ReverseGeocode {
    city_page: Option<CityPage>,
}

#[derive(Deserialize)]
struct CityPage {
    display_name: Option<String>,
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
            FilterKind::Condition,
        ]
    }

    async fn search(&self, alert: &Alert, default_location: Option<&str>) -> Result<Vec<Listing>> {
        if let Some(remaining) = rate_limit::is_rate_limited(MARKETPLACE_ID) {
            log::warn!(target: "snag::facebook", "Rate limited, waiting {}s before next request", remaining);
            anyhow::bail!("Rate limited, retry in {}s", remaining);
        }

        let location_query = alert
            .location
            .as_deref()
            .or(default_location)
            .context("no location set for Facebook Marketplace search")?;

        let (lat, lng) = self.resolve_location(location_query).await?;

        log::info!(target: "snag::facebook", "Searching Facebook Marketplace: '{}'", alert.keywords.join(" "));

        let variables = self.build_search_variables(alert, lat, lng);

        let response = self
            .client
            .post(GRAPHQL_URL)
            .header("sec-fetch-site", "same-origin")
            .form(&[
                ("variables", serde_json::to_string(&variables)?),
                ("doc_id", SEARCH_DOC_ID.to_string()),
            ])
            .send()
            .await
            .context("failed to search Facebook Marketplace")?;

        let status = response.status();
        log::debug!(target: "snag::facebook", "Search API status: {}", status);

        let body_text = response
            .text()
            .await
            .context("failed to read search response body")?;
        log::debug!(target: "snag::facebook", "Search API response ({} bytes): {}",
            body_text.len(),
            body_text.char_indices().nth(500).map(|(i, _)| &body_text[..i]).unwrap_or(&body_text)
        );

        if let Ok(error_check) = serde_json::from_str::<serde_json::Value>(&body_text)
            && let Some(errors) = error_check.get("errors").and_then(|e| e.as_array())
            && let Some(first) = errors.first()
        {
            let msg = first
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown");
            let code = first.get("code").and_then(|c| c.as_u64()).unwrap_or(0);

            if code == FB_RATE_LIMIT_CODE {
                rate_limit::set_rate_limited(MARKETPLACE_ID, RATE_LIMIT_BACKOFF);
                log::error!(target: "snag::facebook", "Rate limited — backing off for {}s", RATE_LIMIT_BACKOFF.as_secs());
            } else {
                log::error!(target: "snag::facebook", "Facebook API error (code {}): {}", code, msg);
            }

            anyhow::bail!("Facebook API error (code {}): {}", code, msg);
        }

        // Successful request — clear rate limit
        rate_limit::clear_rate_limit(MARKETPLACE_ID);

        let body: SearchResponse =
            serde_json::from_str(&body_text).context("failed to parse search response as JSON")?;

        let edges = body
            .data
            .and_then(|d| d.marketplace_search)
            .and_then(|ms| ms.feed_units)
            .map(|fu| fu.edges)
            .unwrap_or_default();

        log::debug!(target: "snag::facebook", "Search returned {} edges", edges.len());

        let now = Utc::now();
        let keywords_lower: Vec<String> = alert.keywords.iter().map(|k| k.to_lowercase()).collect();
        let exclude_lower: Vec<String> = alert
            .exclude_keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();

        let mut listings = vec![];
        let mut filtered_count = 0usize;

        for edge in edges {
            let node = match edge.node.and_then(|n| n.listing) {
                Some(n) => n,
                None => continue,
            };

            let id = match node.id {
                Some(id) => id,
                None => continue,
            };

            let title = node.marketplace_listing_title.unwrap_or_default();

            let title_lower = title.to_lowercase();

            if !keywords_lower.iter().any(|kw| title_lower.contains(kw)) {
                filtered_count += 1;
                continue;
            }

            if exclude_lower.iter().any(|kw| title_lower.contains(kw)) {
                continue;
            }

            let price = node.listing_price.as_ref().and_then(|p| {
                p.amount
                    .as_ref()
                    .and_then(|a| a.parse::<f64>().ok())
                    .or_else(|| {
                        p.formatted_amount.as_ref().and_then(|f| {
                            let cleaned = f.replace(['$', ',', ' '], "");
                            cleaned.parse::<f64>().ok()
                        })
                    })
            });

            let currency = node
                .listing_price
                .as_ref()
                .and_then(|p| p.currency.clone())
                .unwrap_or_else(|| "USD".into());

            let image_url = node
                .primary_listing_photo
                .and_then(|p| p.image)
                .and_then(|i| i.uri);

            let location = node
                .location
                .and_then(|l| l.reverse_geocode)
                .and_then(|r| r.city_page)
                .and_then(|c| c.display_name);

            let url = format!("https://www.facebook.com/marketplace/item/{}/", id);

            listings.push(Listing {
                id,
                title,
                price,
                currency,
                url,
                image_url,
                location,
                condition: alert.condition,
                marketplace: MarketplaceKind::FacebookMarketplace,
                posted_at: None,
                found_at: now,
                description: None,
            });
        }

        if filtered_count > 0 {
            log::info!(target: "snag::facebook", "Filtered {} listings not matching keywords", filtered_count);
        }
        log::info!(target: "snag::facebook", "Facebook returned {} listings", listings.len());

        Ok(listings)
    }
}
