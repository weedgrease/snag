use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use std::path::Path;

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/weedgrease/snag/releases/latest";

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub download_url: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
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

pub async fn check_for_update() -> Result<Option<UpdateInfo>> {
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

    let current = match Version::parse(CURRENT_VERSION) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    let latest = match parse_version(&release.tag_name) {
        Some(v) => v,
        None => return Ok(None),
    };

    if latest <= current {
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
        None => return Ok(None),
    };

    Ok(Some(UpdateInfo {
        latest_version: release.tag_name,
        download_url,
    }))
}

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
