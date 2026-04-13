//! snag — CLI tool for monitoring marketplace listings with alerts.
//! Supports Facebook Marketplace and eBay with a terminal UI.

pub mod config;
pub mod credentials;
pub mod daemon;
pub mod marketplace;
pub mod notifier;
pub mod scheduler;
pub mod tui;
pub mod types;
pub mod uninstall;
pub mod update;
