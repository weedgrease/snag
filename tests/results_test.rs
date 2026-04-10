use chrono::Utc;
use snag::daemon::results::{load_results, save_results};
use snag::types::*;
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn save_and_load_results_round_trips() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("results.json");

    let results = vec![AlertResult {
        alert_id: Uuid::nil(),
        alert_name: "Test".into(),
        listings: vec![Listing {
            id: "ebay-1".into(),
            title: "PS5".into(),
            price: Some(300.0),
            currency: "USD".into(),
            url: "https://ebay.com/1".into(),
            image_url: None,
            location: Some("Denver".into()),
            condition: Some(Condition::Used),
            marketplace: MarketplaceKind::FacebookMarketplace,
            posted_at: None,
            found_at: Utc::now(),
        }],
        checked_at: Utc::now(),
        seen: false,
    }];

    save_results(&results, &path).unwrap();
    let loaded = load_results(&path).unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].alert_name, "Test");
    assert_eq!(loaded[0].listings.len(), 1);
    assert_eq!(loaded[0].listings[0].title, "PS5");
}

#[test]
fn load_missing_results_returns_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.json");

    let results = load_results(&path).unwrap();
    assert!(results.is_empty());
}

#[test]
fn save_results_creates_parent_directories() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nested").join("results.json");

    save_results(&vec![], &path).unwrap();
    assert!(path.exists());
}
