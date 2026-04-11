use chrono::Utc;
use std::path::PathBuf;
use std::time::Duration;

fn rate_limit_path(marketplace: &str) -> PathBuf {
    crate::config::data_dir().join(format!("rate_limit_{}", marketplace))
}

pub fn is_rate_limited(marketplace: &str) -> Option<i64> {
    let path = rate_limit_path(marketplace);
    let content = std::fs::read_to_string(&path).ok()?;
    let until = content.trim().parse::<chrono::DateTime<Utc>>().ok()?;
    let remaining = until.signed_duration_since(Utc::now()).num_seconds();
    if remaining > 0 {
        Some(remaining)
    } else {
        let _ = std::fs::remove_file(&path);
        None
    }
}

pub fn set_rate_limited(marketplace: &str, backoff: Duration) {
    let until = Utc::now() + chrono::Duration::seconds(backoff.as_secs() as i64);
    let path = rate_limit_path(marketplace);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, until.to_rfc3339());
}

pub fn clear_rate_limit(marketplace: &str) {
    let _ = std::fs::remove_file(rate_limit_path(marketplace));
}
