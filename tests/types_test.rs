use snag::types::*;
use uuid::Uuid;
use chrono::Utc;
use std::time::Duration;

#[test]
fn alert_round_trips_through_toml() {
    let alert = Alert {
        id: Uuid::nil(),
        name: "Test Alert".into(),
        marketplaces: vec![MarketplaceKind::FacebookMarketplace],
        keywords: vec!["ps5".into()],
        exclude_keywords: vec!["broken".into()],
        price_min: Some(100.0),
        price_max: Some(500.0),
        location: Some("Denver, CO".into()),
        radius_miles: Some(25),
        condition: Some(Condition::Used),
        category: Some("Electronics".into()),
        check_interval: Duration::from_secs(300),
        notifiers: vec![NotifierKind::Terminal],
        max_results: Some(20),
        enabled: true,
    };

    let toml_str = toml::to_string(&alert).unwrap();
    let deserialized: Alert = toml::from_str(&toml_str).unwrap();

    assert_eq!(deserialized.name, "Test Alert");
    assert_eq!(deserialized.keywords, vec!["ps5"]);
    assert_eq!(deserialized.exclude_keywords, vec!["broken"]);
    assert_eq!(deserialized.price_min, Some(100.0));
    assert_eq!(deserialized.marketplaces, vec![MarketplaceKind::FacebookMarketplace]);
    assert_eq!(deserialized.condition, Some(Condition::Used));
    assert!(deserialized.enabled);
}

#[test]
fn listing_round_trips_through_json() {
    let listing = Listing {
        id: "ebay-123".into(),
        title: "PS5 Console".into(),
        price: Some(299.99),
        currency: "USD".into(),
        url: "https://ebay.com/item/123".into(),
        image_url: Some("https://ebay.com/img/123.jpg".into()),
        location: Some("Denver, CO".into()),
        condition: Some(Condition::Used),
        marketplace: MarketplaceKind::FacebookMarketplace,
        posted_at: Some(Utc::now()),
        found_at: Utc::now(),
    };

    let json = serde_json::to_string(&listing).unwrap();
    let deserialized: Listing = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "ebay-123");
    assert_eq!(deserialized.title, "PS5 Console");
    assert_eq!(deserialized.price, Some(299.99));
    assert_eq!(deserialized.marketplace, MarketplaceKind::FacebookMarketplace);
}

#[test]
fn alert_result_round_trips_through_json() {
    let result = AlertResult {
        alert_id: Uuid::nil(),
        alert_name: "Test".into(),
        listings: vec![],
        checked_at: Utc::now(),
        seen: false,
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: AlertResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.alert_name, "Test");
    assert!(!deserialized.seen);
}

#[test]
fn alert_with_minimal_fields() {
    let alert = Alert {
        id: Uuid::nil(),
        name: "Bare Alert".into(),
        marketplaces: vec![MarketplaceKind::FacebookMarketplace],
        keywords: vec!["couch".into()],
        exclude_keywords: vec![],
        price_min: None,
        price_max: None,
        location: None,
        radius_miles: None,
        condition: None,
        category: None,
        check_interval: Duration::from_secs(600),
        notifiers: vec![NotifierKind::Terminal],
        max_results: None,
        enabled: true,
    };

    let toml_str = toml::to_string(&alert).unwrap();
    let deserialized: Alert = toml::from_str(&toml_str).unwrap();

    assert_eq!(deserialized.name, "Bare Alert");
    assert_eq!(deserialized.price_min, None);
    assert_eq!(deserialized.location, None);
}
