use anyhow::{Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/weedgrease/snag/releases/latest";

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Metadata for an available update: the release tag and the platform-specific download URL.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub download_url: String,
    pub release_notes: Option<String>,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
    body: Option<String>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateCache {
    checked_at: chrono::DateTime<chrono::Utc>,
    latest_version: Option<String>,
    download_url: Option<String>,
    release_notes: Option<String>,
}

fn cache_path() -> PathBuf {
    crate::config::data_dir().join("update_cache.json")
}

fn platform_asset_name() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    format!("snag-{}-{}", arch, os)
}

fn parse_version(tag: &str) -> Option<Version> {
    let trimmed = tag.strip_prefix('v').unwrap_or(tag);
    Version::parse(trimmed).ok()
}

fn load_cache() -> Option<UpdateCache> {
    let path = cache_path();
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_cache(cache: &UpdateCache) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = std::fs::write(&path, json);
    }
}

/// Queries the GitHub Releases API and returns `Some(UpdateInfo)` when a newer version exists
/// with a matching platform asset, or `None` if already up to date or no asset is available.
/// Results are cached for 24 hours to avoid hitting the API on every launch.
pub async fn check_for_update() -> Result<Option<UpdateInfo>> {
    let current = match Version::parse(CURRENT_VERSION) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    // Check cache first
    if let Some(cache) = load_cache() {
        let age = chrono::Utc::now().signed_duration_since(cache.checked_at);
        if age < chrono::Duration::hours(24) {
            // Cache is fresh: return cached result if a newer version was recorded
            if let Some(ref cached_version) = cache.latest_version
                && let Some(latest) = parse_version(cached_version)
                && latest > current
                && let Some(url) = cache.download_url.clone()
            {
                return Ok(Some(UpdateInfo {
                    latest_version: cached_version.clone(),
                    download_url: url,
                    release_notes: cache.release_notes.clone(),
                }));
            }
            // Cache is fresh but no update available
            return Ok(None);
        }
    }

    // Cache is stale or missing — hit GitHub
    let client = reqwest::Client::builder()
        .user_agent(format!("snag/{}", CURRENT_VERSION))
        .build()?;

    let release: GitHubRelease = client
        .get(GITHUB_RELEASES_URL)
        .send()
        .await
        .context("failed to fetch latest release")?
        .error_for_status()
        .context("GitHub API returned an error")?
        .json()
        .await
        .context("failed to parse release JSON")?;

    let latest = match parse_version(&release.tag_name) {
        Some(v) => v,
        None => {
            // Write a cache so we don't hammer GitHub on parse failures
            write_cache(&UpdateCache {
                checked_at: chrono::Utc::now(),
                latest_version: None,
                download_url: None,
                release_notes: None,
            });
            return Ok(None);
        }
    };

    if latest <= current {
        write_cache(&UpdateCache {
            checked_at: chrono::Utc::now(),
            latest_version: None,
            download_url: None,
            release_notes: None,
        });
        return Ok(None);
    }

    let expected_name = platform_asset_name();
    let download_url = release
        .assets
        .iter()
        .find(|a| a.name == expected_name)
        .map(|a| a.browser_download_url.clone());

    let download_url = match download_url {
        Some(url) => url,
        None => {
            write_cache(&UpdateCache {
                checked_at: chrono::Utc::now(),
                latest_version: None,
                download_url: None,
                release_notes: None,
            });
            return Ok(None);
        }
    };

    let release_notes = release.body.clone();

    write_cache(&UpdateCache {
        checked_at: chrono::Utc::now(),
        latest_version: Some(release.tag_name.clone()),
        download_url: Some(download_url.clone()),
        release_notes: release_notes.clone(),
    });

    Ok(Some(UpdateInfo {
        latest_version: release.tag_name,
        download_url,
        release_notes,
    }))
}

/// Downloads the new binary and atomically replaces the running executable, keeping a `.bak`
/// copy for rollback if the rename fails.
pub async fn perform_update(info: &UpdateInfo) -> Result<()> {
    let current_exe = std::env::current_exe().context("failed to determine current executable")?;
    let exe_dir = current_exe
        .parent()
        .context("failed to determine executable directory")?;

    println!("Downloading {}...", info.latest_version);

    let client = reqwest::Client::builder()
        .user_agent(format!("snag/{}", CURRENT_VERSION))
        .build()?;

    let bytes = client
        .get(&info.download_url)
        .send()
        .await
        .context("failed to download update")?
        .error_for_status()
        .context("download returned an error")?
        .bytes()
        .await
        .context("failed to read download body")?;

    let temp_path = exe_dir.join("snag.tmp");
    let backup_path = exe_dir.join("snag.bak");

    std::fs::write(&temp_path, &bytes).context("failed to write temp file")?;

    set_executable(&temp_path)?;

    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    std::fs::rename(&current_exe, &backup_path).context("failed to backup current binary")?;

    if let Err(e) = std::fs::rename(&temp_path, &current_exe) {
        let _ = std::fs::rename(&backup_path, &current_exe);
        return Err(e).context("failed to install new binary, restored backup");
    }

    let _ = std::fs::remove_file(&backup_path);

    println!(
        "Updated snag from v{} to {}",
        CURRENT_VERSION, info.latest_version
    );

    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(path, perms).context("failed to set executable permissions")?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

pub async fn run_update() -> Result<()> {
    println!("Checking for updates...");

    let info = check_for_update()
        .await?
        .context("already up to date")?;

    perform_update(&info).await
}
