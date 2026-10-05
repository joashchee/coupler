//! Checking for a newer Coupler, only when the player asks for it: gear →
//! Check for Updates Now, or Check for Updates Automatically turned on
//! (off until then; `lib/updates.ts`). The one place Coupler talks to
//! anything but CoffeeMUD (CLAUDE.md rules 1 and 2): a single GET of
//! GitHub's latest release of Coupler's own repository, sending nothing
//! but Coupler's version in the User-Agent GitHub requires. No account,
//! no ID, nothing about the player or their play; the answer's only read
//! for the version and the release's page. Nothing is downloaded or
//! installed: `open` shows the release's page in the player's browser.
//!
//! Releases are made by the Windows and Linux builds
//! (`scripts/publish-release.sh`); GitHub's "latest" never counts a
//! prerelease, so the `dev` builds between versions aren't offered.

use serde::{Deserialize, Serialize};

/// The latest release of Coupler, asked of GitHub's API.
pub const LATEST: &str = "https://api.github.com/repos/joashchee/coupler/releases/latest";
/// The page `open` shows (also the fallback when GitHub names no page).
pub const PAGE: &str = "https://github.com/joashchee/coupler/releases/latest";

/// What a check found, for the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// This copy's version ("0.26.0").
    pub current: String,
    /// The latest release's version, its tag's leading `v` dropped.
    pub latest: String,
    /// The latest is newer than this copy.
    pub newer: bool,
}

/// The part of GitHub's release JSON read.
#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

/// The latest release's version from GitHub's JSON, or None.
pub fn latest_version(json: &str) -> Option<String> {
    let release: Release = serde_json::from_str(json).ok()?;
    let version = release.tag_name.trim().trim_start_matches(['v', 'V']).to_string();
    parse(&version).map(|_| version)
}

/// "1.2.3" as numbers, a suffix after `-` or `+` ignored; None if it
/// isn't one.
fn parse(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let major = parts.next()??;
    let minor = parts.next().unwrap_or(Some(0))?;
    let patch = parts.next().unwrap_or(Some(0))?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// `latest` is a newer version than `current`. Anything unreadable never is.
pub fn newer(current: &str, latest: &str) -> bool {
    match (parse(current), parse(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

/// Asks GitHub for the latest release, compared with this copy's version.
pub async fn check(current: &str) -> Result<Check, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("Coupler/{current}"))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(LATEST)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "GitHub couldn't be reached to check for updates.".to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("No Coupler release has been published yet.".into());
    }
    if !response.status().is_success() {
        return Err(format!("GitHub didn't answer the update check ({}).", response.status().as_u16()));
    }
    let body = response.text().await.map_err(|e| e.to_string())?;
    let latest = latest_version(&body).ok_or("GitHub's answer had no version in it.")?;
    Ok(Check { current: current.to_string(), newer: newer(current, &latest), latest })
}

/// Shows `PAGE` in the player's browser by the system's own opener
/// (`open` on macOS, the URL handler on Windows, `xdg-open` on Linux;
/// docs/platform-parity.md). Only ever this one fixed address.
pub fn open() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = std::process::Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = std::process::Command::new("xdg-open");
    let mut child = command.arg(PAGE).spawn().map_err(|_| "The release's page couldn't be opened.".to_string())?;
    // Reaped once the opener hands the page over, so it leaves no zombie.
    std::thread::spawn(move || child.wait());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_tag() {
        let json = r#"{"tag_name":"v0.27.0","name":"Coupler 0.27.0","html_url":"https://github.com/joashchee/coupler/releases/tag/v0.27.0","assets":[]}"#;
        assert_eq!(latest_version(json).as_deref(), Some("0.27.0"));
        assert_eq!(latest_version(r#"{"tag_name":"0.3"}"#).as_deref(), Some("0.3"));
        assert_eq!(latest_version(r#"{"tag_name":"dev"}"#), None);
        assert_eq!(latest_version(r#"{"message":"Not Found"}"#), None);
        assert_eq!(latest_version("not json"), None);
    }

    #[test]
    fn compares_versions_as_numbers() {
        assert!(newer("0.26.0", "0.27.0"));
        assert!(newer("0.9.0", "0.10.0"));
        assert!(newer("0.26.0", "1.0"));
        assert!(newer("0.26.0", "0.26.1-beta"));
        assert!(!newer("0.26.0", "0.26.0"));
        assert!(!newer("0.27.0", "0.26.9"));
        assert!(!newer("0.26.0", "dev"));
        assert!(!newer("0.26.0", "1.2.3.4"));
    }
}
