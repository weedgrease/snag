# Facebook Marketplace Provider Design

Replace the stub marketplace providers with a real Facebook Marketplace implementation using Facebook's internal GraphQL API.

## Cleanup

- Remove `src/marketplace/providers/ebay.rs` and `MarketplaceKind::Ebay`
- Remove `src/marketplace/providers/facebook.rs` (stub)
- Update `create_marketplace` registry to only contain Facebook
- Update any tests referencing `MarketplaceKind::Ebay`

## Config Change

Add `default_location: Option<String>` to `GlobalSettings`:

```rust
pub struct GlobalSettings {
    pub default_check_interval: Duration,
    pub default_max_results: Option<u32>,
    pub default_notifier: NotifierKind,
    pub check_for_updates: bool,
    #[serde(default)]
    pub default_location: Option<String>,
}
```

This is editable in the Settings tab alongside the other defaults. Per-alert location overrides the global default.

## Facebook GraphQL API

Two request types to `https://www.facebook.com/api/graphql/`:

### Location Lookup

Converts a city/address string to lat/long coordinates.

- `doc_id`: `5585904654783609`
- Input: location query string (e.g., "Denver, CO")
- Output: list of matching locations with `name`, `latitude`, `longitude`
- Results cached in a `HashMap<String, (f64, f64)>` on the provider struct so the same location string doesn't hit Facebook repeatedly

### Listing Search

Searches marketplace listings.

- `doc_id`: `7111939778879383`
- Required: latitude, longitude, query keywords
- Optional filters via `commerce_*` GraphQL variables:
  - `commerce_search_and_rp_price_range` — min/max price
  - `commerce_search_and_rp_condition` — item condition
- Returns up to 24 results per request

### Request Format

POST to `https://www.facebook.com/api/graphql/` with form-encoded body:

```
variables=<JSON>&doc_id=<id>
```

Headers mimic a browser:

```
User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36
sec-fetch-site: same-origin
```

### Response Parsing

The search response contains listings at `data.marketplace_search.feed_units.edges`. Each listing node contains:

- `id` — Facebook listing ID
- `marketplace_listing_title` — title
- `listing_price.formatted_amount` — price string (e.g., "$150")
- `listing_price.amount` — numeric price
- `listing_price.currency` — currency code
- `primary_listing_photo.image.uri` — image URL
- `location.reverse_geocode.city_page.display_name` — seller location
- `custom_sub_titles_with_rendering_flags` — condition info

## Filter Mapping

| Alert field | Facebook GraphQL parameter |
|---|---|
| `keywords` | `bqf.query` |
| `price_min` / `price_max` | `commerce_search_and_rp_price_range` |
| `location` + `radius_miles` | lat/long from location lookup + radius |
| `condition` | `commerce_search_and_rp_condition` |

Condition enum mapping:

| Our Condition | Facebook value |
|---|---|
| New | `new` |
| LikeNew | `used_like_new` |
| Used | `used_good` |
| ForParts | `used_fair` |

## Location Resolution

When the Facebook provider runs a search:

1. Use the alert's `location` field if set
2. Fall back to `GlobalSettings.default_location`
3. If neither exists, this should not happen — the alert form validation prevents it

## Alert Form Validation

When saving an alert that includes `FacebookMarketplace`:

- If the alert has no `location` AND `GlobalSettings.default_location` is `None`, refuse to save
- The alert form's `to_alert()` returns `None` in this case (existing pattern for validation failures)

The alert form needs access to the current `default_location` to perform this check. Pass it as a parameter to `to_alert()`.

## Provider Implementation

```rust
pub struct FacebookMarketplace {
    client: reqwest::Client,
    location_cache: HashMap<String, (f64, f64)>,
}
```

The provider:
- Creates a `reqwest::Client` with browser-like headers on construction
- Caches location lookups for the lifetime of the provider instance
- Implements `Marketplace` trait with `search()` that resolves location, builds the GraphQL query, makes the request, and maps the response to `Vec<Listing>`

## Settings Tab

Add `default_location` as a 5th editable field in the Settings tab (text input, same as check interval / max results).

## Files Changed

| File | Change |
|---|---|
| `src/config.rs` | Add `default_location` to GlobalSettings |
| `src/types.rs` | Remove `MarketplaceKind::Ebay` |
| `src/marketplace/mod.rs` | Update registry, remove Ebay |
| `src/marketplace/providers/mod.rs` | Remove ebay module |
| `src/marketplace/providers/ebay.rs` | Delete |
| `src/marketplace/providers/facebook.rs` | Replace stub with real implementation |
| `src/tui/tabs/settings.rs` | Add default_location field |
| `src/tui/dialogs/alert_form.rs` | Add location validation for Facebook alerts, pass default_location |
| `src/tui/app.rs` | Pass default_location when creating alert form |
| `tests/config_test.rs` | Update for new field, remove Ebay references |
| `tests/types_test.rs` | Update for removed Ebay variant |
| `tests/daemon_test.rs` | Update marketplace references |
