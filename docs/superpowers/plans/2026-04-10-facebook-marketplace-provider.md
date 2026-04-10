# Facebook Marketplace Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace stub marketplace providers with a real Facebook Marketplace implementation using Facebook's internal GraphQL API.

**Architecture:** Delete the eBay stub and its enum variant. Rewrite the Facebook provider to hit `facebook.com/api/graphql/` with reverse-engineered `doc_id` parameters for location lookup and listing search. Add `default_location` to config and settings. Add location validation to the alert form for Facebook alerts.

**Tech Stack:** Rust, reqwest (existing), Facebook GraphQL API (unauthenticated)

---

## File Structure

```
src/
├── types.rs                              — remove MarketplaceKind::Ebay
├── config.rs                             — add default_location field
├── marketplace/
│   ├── mod.rs                            — remove Ebay from registry
│   └── providers/
│       ├── mod.rs                        — remove ebay module
│       ├── ebay.rs                       — DELETE
│       └── facebook.rs                   — replace stub with real implementation
├── tui/
│   ├── tabs/settings.rs                  — add default_location as 5th editable field
│   └── dialogs/alert_form.rs             — remove ebay refs, add location validation, accept default_location
│   └── app.rs                            — pass default_location to alert form
tests/
├── types_test.rs                         — replace Ebay with FacebookMarketplace
├── config_test.rs                        — add default_location to test configs
├── daemon_test.rs                        — replace Ebay with FacebookMarketplace
├── results_test.rs                       — replace Ebay with FacebookMarketplace
```

---

### Task 1: Remove eBay, Clean Up Types and Registry

**Files:**
- Modify: `src/types.rs`
- Delete: `src/marketplace/providers/ebay.rs`
- Modify: `src/marketplace/providers/mod.rs`
- Modify: `src/marketplace/mod.rs`
- Modify: `src/tui/dialogs/alert_form.rs`
- Modify: `tests/types_test.rs`
- Modify: `tests/config_test.rs`
- Modify: `tests/daemon_test.rs`
- Modify: `tests/results_test.rs`

- [ ] **Step 1: Remove MarketplaceKind::Ebay from types.rs**

In `src/types.rs`, change the `MarketplaceKind` enum and its `Display` impl:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketplaceKind {
    FacebookMarketplace,
}

impl std::fmt::Display for MarketplaceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FacebookMarketplace => write!(f, "Facebook Marketplace"),
        }
    }
}
```

- [ ] **Step 2: Delete ebay.rs and update providers/mod.rs**

Delete `src/marketplace/providers/ebay.rs`.

Replace `src/marketplace/providers/mod.rs`:

```rust
pub mod facebook;
```

- [ ] **Step 3: Update marketplace registry**

Replace `src/marketplace/mod.rs`:

```rust
pub mod providers;

use crate::types::{Alert, FilterKind, Listing, MarketplaceKind};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Marketplace: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> MarketplaceKind;
    fn supported_filters(&self) -> &[FilterKind];
    async fn search(&self, alert: &Alert, default_location: Option<&str>) -> Result<Vec<Listing>>;
}

pub fn create_marketplace(kind: MarketplaceKind) -> Box<dyn Marketplace> {
    match kind {
        MarketplaceKind::FacebookMarketplace => {
            Box::new(providers::facebook::FacebookMarketplace::new())
        }
    }
}
```

Note: the `search` method now takes `default_location: Option<&str>` so providers can fall back to it when the alert has no location set.

- [ ] **Step 4: Update alert_form.rs — remove eBay references**

In `src/tui/dialogs/alert_form.rs`:

Change the `new()` method's Marketplaces field default from `"ebay"` to `"facebook"`:
```rust
FormField::new("Marketplaces", "facebook"),
```

In `from_alert()`, remove the `MarketplaceKind::Ebay` match arm:
```rust
let marketplaces: Vec<String> = alert.marketplaces.iter().map(|m| match m {
    MarketplaceKind::FacebookMarketplace => "facebook".into(),
}).collect();
```

In `to_alert()`, remove the `"ebay"` match arm from the marketplace parsing:
```rust
let marketplaces: Vec<MarketplaceKind> = self.fields[1]
    .value
    .split(',')
    .filter_map(|s| match s.trim().to_lowercase().as_str() {
        "facebook" | "fb" => Some(MarketplaceKind::FacebookMarketplace),
        _ => None,
    })
    .collect();
```

- [ ] **Step 5: Update all test files to use FacebookMarketplace instead of Ebay**

In `tests/types_test.rs`, replace all `MarketplaceKind::Ebay` with `MarketplaceKind::FacebookMarketplace`.

In `tests/config_test.rs`, replace all `MarketplaceKind::Ebay` with `MarketplaceKind::FacebookMarketplace`.

In `tests/daemon_test.rs`, replace all `MarketplaceKind::Ebay` with `MarketplaceKind::FacebookMarketplace`.

In `tests/results_test.rs`, replace all `MarketplaceKind::Ebay` with `MarketplaceKind::FacebookMarketplace`.

- [ ] **Step 6: Update the Facebook stub to match the new trait signature**

Temporarily update `src/marketplace/providers/facebook.rs` so it compiles with the new `search` signature:

```rust
use crate::marketplace::Marketplace;
use crate::types::{Alert, FilterKind, Listing, MarketplaceKind};
use anyhow::Result;
use async_trait::async_trait;

pub struct FacebookMarketplace;

impl Default for FacebookMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

impl FacebookMarketplace {
    pub fn new() -> Self {
        Self
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
            FilterKind::Condition,
        ]
    }

    async fn search(&self, _alert: &Alert, _default_location: Option<&str>) -> Result<Vec<Listing>> {
        Ok(vec![])
    }
}
```

- [ ] **Step 7: Update daemon/mod.rs to pass default_location to search**

In `src/daemon/mod.rs`, find the `check_alert` function. It calls `marketplace.search(alert).await`. Update it to pass the default location from config.

The `check_alert` function needs to accept the default location. Change its signature and the call site:

In `run_scheduler`, change the call from:
```rust
if let Err(e) = check_alert(alert, results_path).await {
```
to:
```rust
if let Err(e) = check_alert(alert, results_path, config.settings.default_location.as_deref()).await {
```

In `check_once_with_paths`, change similarly:
```rust
check_alert(alert, results_path, config.settings.default_location.as_deref()).await?;
```

Change the `check_alert` signature:
```rust
async fn check_alert(alert: &crate::types::Alert, results_path: &Path, default_location: Option<&str>) -> Result<()> {
```

And pass it through to the marketplace search:
```rust
match marketplace.search(alert, default_location).await {
```

- [ ] **Step 8: Verify everything compiles and tests pass**

Run: `cargo check && cargo test`
Expected: compiles, all tests pass

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor: remove eBay stub, update trait to pass default_location"
```

---

### Task 2: Add default_location to Config and Settings Tab

**Files:**
- Modify: `src/config.rs`
- Modify: `src/tui/tabs/settings.rs`
- Modify: `tests/config_test.rs`

- [ ] **Step 1: Add default_location to GlobalSettings**

In `src/config.rs`, add to `GlobalSettings` after `check_for_updates`:

```rust
#[serde(default)]
pub default_location: Option<String>,
```

Update the `Default` impl to include `default_location: None`.

- [ ] **Step 2: Update config tests**

In `tests/config_test.rs`, add `default_location: None` (or a test value) to every `GlobalSettings` construction.

Add a test:

```rust
#[test]
fn config_with_default_location_round_trips() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let config = AppConfig {
        settings: GlobalSettings {
            default_check_interval: Duration::from_secs(300),
            default_max_results: Some(20),
            default_notifier: NotifierKind::Terminal,
            check_for_updates: true,
            default_location: Some("Denver, CO".into()),
        },
        alerts: vec![],
    };

    save_config(&config, &config_path).unwrap();
    let loaded = load_config(&config_path).unwrap();

    assert_eq!(loaded.settings.default_location, Some("Denver, CO".into()));
}
```

- [ ] **Step 3: Add default_location as 5th field in settings tab**

In `src/tui/tabs/settings.rs`:

Add the new constant and update `FIELD_COUNT`:
```rust
const FIELD_CHECK_INTERVAL: usize = 0;
const FIELD_MAX_RESULTS: usize = 1;
const FIELD_NOTIFICATION: usize = 2;
const FIELD_CHECK_UPDATES: usize = 3;
const FIELD_DEFAULT_LOCATION: usize = 4;
const FIELD_COUNT: usize = 5;
```

In `current_field_value`, add the new field:
```rust
FIELD_DEFAULT_LOCATION => config
    .settings
    .default_location
    .clone()
    .unwrap_or_default(),
```

In `apply_edit`, add:
```rust
FIELD_DEFAULT_LOCATION => {
    let trimmed = self.edit_buffer.trim().to_string();
    if trimmed.is_empty() {
        config.settings.default_location = None;
    } else {
        config.settings.default_location = Some(trimmed);
    }
}
```

In `handle_key`, the Enter handler should route `FIELD_DEFAULT_LOCATION` to the text editing path (same as check interval and max results — the `_ =>` arm already handles this).

In `render_defaults_section`, add the 5th field to the `fields` array:
```rust
let location_val = config
    .settings
    .default_location
    .clone()
    .unwrap_or_else(|| "not set".into());

let fields = [
    ("Check interval (s)", interval_val),
    ("Max results", max_val),
    ("Notification", notifier_val),
    ("Check for updates", updates_val.to_string()),
    ("Default location", location_val),
];
```

- [ ] **Step 4: Update daemon_test.rs GlobalSettings**

In `tests/daemon_test.rs`, add `default_location: None` to both `GlobalSettings` constructions.

- [ ] **Step 5: Verify everything compiles and tests pass**

Run: `cargo check && cargo test`
Expected: compiles, all tests pass

- [ ] **Step 6: Commit**

```bash
git add src/config.rs src/tui/tabs/settings.rs tests/config_test.rs tests/daemon_test.rs
git commit -m "feat: add default_location to config and settings tab"
```

---

### Task 3: Real Facebook Marketplace Provider

**Files:**
- Modify: `src/marketplace/providers/facebook.rs`

- [ ] **Step 1: Replace facebook.rs with the real implementation**

Replace `src/marketplace/providers/facebook.rs` entirely:

```rust
use crate::marketplace::Marketplace;
use crate::types::{Alert, Condition, FilterKind, Listing, MarketplaceKind};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

const GRAPHQL_URL: &str = "https://www.facebook.com/api/graphql/";
const LOCATION_DOC_ID: &str = "5585904654783609";
const SEARCH_DOC_ID: &str = "7111939778879383";

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
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            location_cache: Mutex::new(HashMap::new()),
        }
    }

    async fn resolve_location(&self, query: &str) -> Result<(f64, f64)> {
        if let Some(cached) = self.location_cache.lock().unwrap().get(query) {
            return Ok(*cached);
        }

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
            .context("failed to fetch location")?
            .error_for_status()
            .context("location request failed")?;

        let body: serde_json::Value = response
            .json()
            .await
            .context("failed to parse location response")?;

        let locations = body
            .pointer("/data/city_street_search/street_results/edges")
            .and_then(|v| v.as_array())
            .context("unexpected location response structure")?;

        let node = locations
            .first()
            .and_then(|edge| edge.get("node"))
            .context("no locations found")?;

        let lat = node
            .get("latitude")
            .and_then(|v| v.as_f64())
            .context("missing latitude")?;

        let lng = node
            .get("longitude")
            .and_then(|v| v.as_f64())
            .context("missing longitude")?;

        self.location_cache
            .lock()
            .unwrap()
            .insert(query.to_string(), (lat, lng));

        Ok((lat, lng))
    }

    fn build_search_variables(
        &self,
        alert: &Alert,
        lat: f64,
        lng: f64,
    ) -> serde_json::Value {
        let mut filters = serde_json::Map::new();

        filters.insert(
            "commerce_search_and_rp_available".into(),
            serde_json::json!({"name": "commerce_search_and_rp_available", "value": "true"}),
        );

        filters.insert(
            "commerce_search_and_rp_condition".into(),
            serde_json::json!({"name": "commerce_search_and_rp_condition", "value": alert.condition.map(|c| match c {
                Condition::New => "new",
                Condition::LikeNew => "used_like_new",
                Condition::Used => "used_good",
                Condition::ForParts => "used_fair",
            }).unwrap_or("all")}),
        );

        if alert.price_min.is_some() || alert.price_max.is_some() {
            let min = alert.price_min.unwrap_or(0.0) as i64;
            let max = alert.price_max.unwrap_or(999999999) as i64;
            filters.insert(
                "commerce_search_and_rp_price_range".into(),
                serde_json::json!({
                    "name": "commerce_search_and_rp_price_range",
                    "value": format!("{{\"currency\":\"USD\",\"min_price\":{},\"max_price\":{}}}", min * 100, max * 100)
                }),
            );
        }

        let radius_km = alert
            .radius_miles
            .map(|m| (m as f64 * 1.60934) as i64)
            .unwrap_or(100);

        let keywords = alert.keywords.join(" ");

        serde_json::json!({
            "count": 24,
            "params": {
                "bqf": {
                    "callsite": "COMMERCE_MKTPLACE_WWW",
                    "query": keywords
                },
                "browse_request_params": {
                    "commerce_enable_local_pickup": true,
                    "commerce_enable_shipping": true,
                    "commerce_search_and_rp_available": true,
                    "commerce_search_and_rp_condition": null,
                    "filter_location_latitude": lat,
                    "filter_location_longitude": lng,
                    "filter_radius_kms": radius_km
                },
                "custom_request_params": serde_json::Value::Object(filters)
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

#[derive(Deserialize)]
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
        let location_query = alert
            .location
            .as_deref()
            .or(default_location)
            .context("no location set for Facebook Marketplace search")?;

        let (lat, lng) = self.resolve_location(location_query).await?;

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
            .context("failed to search Facebook Marketplace")?
            .error_for_status()
            .context("Facebook Marketplace search request failed")?;

        let body: SearchResponse = response
            .json()
            .await
            .context("failed to parse search response")?;

        let edges = body
            .data
            .and_then(|d| d.marketplace_search)
            .and_then(|ms| ms.feed_units)
            .map(|fu| fu.edges)
            .unwrap_or_default();

        let now = Utc::now();
        let exclude_lower: Vec<String> = alert
            .exclude_keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();

        let mut listings = vec![];

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
            if exclude_lower.iter().any(|kw| title_lower.contains(kw)) {
                continue;
            }

            let price = node
                .listing_price
                .as_ref()
                .and_then(|p| p.amount.as_ref())
                .and_then(|a| a.parse::<f64>().ok())
                .map(|cents| cents / 100.0);

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
            });
        }

        Ok(listings)
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add src/marketplace/providers/facebook.rs
git commit -m "feat: implement real Facebook Marketplace provider via GraphQL API"
```

---

### Task 4: Alert Form Location Validation

**Files:**
- Modify: `src/tui/dialogs/alert_form.rs`
- Modify: `src/tui/app.rs`

- [ ] **Step 1: Update alert_form.rs to accept and validate default_location**

In `src/tui/dialogs/alert_form.rs`, change `to_alert` to accept a `default_location` parameter:

```rust
pub fn to_alert(&self, default_location: Option<&str>) -> Option<Alert> {
```

After building the `marketplaces` and `location` values, add validation before constructing the Alert:

```rust
let has_facebook = marketplaces.contains(&MarketplaceKind::FacebookMarketplace);
if has_facebook && location.is_none() && default_location.is_none() {
    return None;
}
```

Also update the `handle_key` method's save handler to pass `None` for now (app.rs will provide the real value):

Change the `'s'` key handler to store the default_location on the dialog struct. Add a field to `AlertFormDialog`:

```rust
pub default_location: Option<String>,
```

Initialize it to `None` in `new()` and `from_alert()`.

Add a setter:
```rust
pub fn set_default_location(&mut self, loc: Option<String>) {
    self.default_location = loc;
}
```

Update the `'s'` handler:
```rust
KeyCode::Char('s') => match self.to_alert(self.default_location.as_deref()) {
    Some(alert) => DialogResult::Submit(alert),
    None => DialogResult::Continue,
},
```

- [ ] **Step 2: Update app.rs to pass default_location when creating alert form dialogs**

In `src/tui/app.rs`, where `AlertFormDialog::new()` and `AlertFormDialog::from_alert()` are called, set the default location after creation:

For `CreateAlert`:
```rust
crate::tui::tabs::alerts::AlertsAction::CreateAlert => {
    let mut dialog = AlertFormDialog::new();
    dialog.set_default_location(self.config.settings.default_location.clone());
    self.active_dialog = Some(ActiveDialog::AlertForm(dialog));
}
```

For `EditAlert`:
```rust
crate::tui::tabs::alerts::AlertsAction::EditAlert(idx) => {
    if let Some(alert) = self.config.alerts.get(idx) {
        let mut dialog = AlertFormDialog::from_alert(alert);
        dialog.set_default_location(self.config.settings.default_location.clone());
        self.active_dialog = Some(ActiveDialog::AlertForm(dialog));
    }
}
```

- [ ] **Step 3: Verify it compiles and tests pass**

Run: `cargo check && cargo test`
Expected: compiles, all tests pass

- [ ] **Step 4: Commit**

```bash
git add src/tui/dialogs/alert_form.rs src/tui/app.rs
git commit -m "feat: add location validation for Facebook Marketplace alerts"
```

---

### Task 5: Final Verification

**Files:** None — verification only

- [ ] **Step 1: Run all tests**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings (fix any that appear)

- [ ] **Step 3: Build release**

Run: `cargo build --release`
Expected: builds successfully

- [ ] **Step 4: Commit any fixes**

```bash
git add -A
git commit -m "chore: fix clippy warnings and verify Facebook Marketplace integration"
```
