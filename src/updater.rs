use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config_manager::BaseConfig;

const REPOSITORY: &str = "skills-yaml/skm";
const REPOSITORY_URL: &str = "https://github.com/skills-yaml/skm.git";

/// Cache TTL in seconds (1 hour)
const UPDATE_CACHE_TTL: u64 = 3600;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateChannel {
    Prod,
    Development,
}

impl UpdateChannel {
    pub fn parse(value: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match value {
            "prod" | "production" => Ok(Self::Prod),
            "dev" | "development" => Ok(Self::Development),
            _ => Err(format!("Unsupported update channel '{}'", value).into()),
        }
    }

    pub fn as_installer_arg(self) -> &'static str {
        match self {
            Self::Prod => "prod",
            Self::Development => "development",
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::Prod => "prod-latest",
            Self::Development => "development-latest",
        }
    }
}

pub fn current_build_commit() -> &'static str {
    env!("SKM_BUILD_COMMIT")
}

#[allow(dead_code)]
pub fn current_build_channel() -> &'static str {
    env!("SKM_BUILD_CHANNEL")
}

pub fn check_for_update(channel: UpdateChannel) -> Result<bool, Box<dyn std::error::Error>> {
    let current = current_build_commit();
    let latest = latest_release_commit(channel)?;
    let current_short = short_sha(current);
    let latest_short = short_sha(&latest);

    eprintln!(
        "Current version: {} (commit: {})",
        env!("CARGO_PKG_VERSION"),
        current_short
    );
    eprintln!("Latest {} build: {}", channel.tag(), latest_short);

    if current == "unknown" {
        eprintln!("Update status: unknown (current build commit is not embedded)");
        return Ok(true);
    }

    if current == latest {
        eprintln!("Update status: up to date");
        Ok(false)
    } else {
        eprintln!("Update status: update available");
        Ok(true)
    }
}

pub fn install_update(channel: UpdateChannel) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(windows) {
        install_update_windows(channel)
    } else {
        install_update_unix(channel)
    }
}

fn latest_release_commit(channel: UpdateChannel) -> Result<String, Box<dyn std::error::Error>> {
    let ref_name = format!("refs/tags/{}", channel.tag());
    let output = Command::new("git")
        .args(["ls-remote", REPOSITORY_URL, &ref_name])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to check latest release tag: {}", err).into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let commit = stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("Release tag '{}' was not found", channel.tag()))?;

    Ok(commit.to_string())
}

fn install_update_unix(channel: UpdateChannel) -> Result<(), Box<dyn std::error::Error>> {
    let script_url = format!(
        "https://raw.githubusercontent.com/{}/main/scripts/install.sh",
        REPOSITORY
    );
    let command = format!(
        "curl -fsSL '{}' | sh -s -- {}",
        script_url,
        channel.as_installer_arg()
    );

    eprintln!("Running installer for {}...", channel.tag());
    let status = Command::new("sh").args(["-c", &command]).status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Installer exited with status {}", status).into())
    }
}

#[cfg(windows)]
fn install_update_windows(channel: UpdateChannel) -> Result<(), Box<dyn std::error::Error>> {
    let script_url = format!(
        "https://raw.githubusercontent.com/{}/main/scripts/install.ps1",
        REPOSITORY
    );
    let command = format!(
        "Start-Sleep -Seconds 2; $script = Join-Path $env:TEMP 'skm-install.ps1'; Invoke-WebRequest -Uri '{}' -OutFile $script; & $script -Channel {} -AddToPath",
        script_url,
        channel.as_installer_arg()
    );

    eprintln!("Starting Windows updater for {}...", channel.tag());
    eprintln!("The update will continue in a separate PowerShell process after skm exits.");
    Command::new("cmd")
        .args([
            "/C",
            "start",
            "",
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &command,
        ])
        .spawn()?;

    Ok(())
}

#[cfg(not(windows))]
fn install_update_windows(_channel: UpdateChannel) -> Result<(), Box<dyn std::error::Error>> {
    unreachable!("Windows updater is only used on Windows")
}

/// Struct for caching update check results
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UpdateCache {
    update_available: bool,
    #[serde(default)]
    latest_commit: String,
    checked_at: u64,
    ttl_seconds: u64,
}

impl UpdateCache {
    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.checked_at + self.ttl_seconds
    }
}

/// Get the update cache path
fn get_update_cache_path() -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|d| d.join("skm").join("update_cache.json"))
}

/// Load cached update result if valid
fn get_cached_update_result() -> Option<UpdateCache> {
    let path = get_update_cache_path()?;
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Cache the update check result
fn cache_update_result(update_available: bool, latest_commit: &str) {
    let path = match get_update_cache_path() {
        Some(p) => p,
        None => return,
    };

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let cache = UpdateCache {
        update_available,
        latest_commit: latest_commit.to_string(),
        checked_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        ttl_seconds: UPDATE_CACHE_TTL,
    };

    let content = serde_json::to_string(&cache).unwrap();
    let _ = fs::write(&path, content);
}

/// Check if update check is disabled via environment variable or config
fn is_update_check_disabled() -> bool {
    // Check environment variable
    if std::env::var("SKM_CHECK_UPDATE") == Ok("false".to_string()) {
        return true;
    }

    // Check base config
    if let Ok(base_config) = BaseConfig::load() {
        if !base_config.check_for_updates {
            return true;
        }
    }

    false
}

/// Silent version of check_for_update that doesn't print status messages
pub fn check_for_update_silent(
    channel: UpdateChannel,
) -> Result<(bool, String), Box<dyn std::error::Error>> {
    let current = current_build_commit();
    let latest = latest_release_commit(channel)?;

    if current == "unknown" {
        return Ok((true, latest));
    }

    Ok((current != latest, latest))
}

/// Display update notification message
fn notify_update_available(latest: &str) {
    let current = current_build_commit();

    eprintln!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("  New SKM update available!");
    eprintln!(
        "  Current version: {} (commit: {})",
        env!("CARGO_PKG_VERSION"),
        &current[..current.len().min(12)]
    );
    eprintln!("  Latest commit:   {}", &latest[..latest.len().min(12)]);
    eprintln!("  Run `skm update` to update!");
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}

/// Check for update and notify user if available
pub fn check_and_notify_update() -> Result<(), Box<dyn std::error::Error>> {
    // Check if update check is disabled
    if is_update_check_disabled() {
        return Ok(());
    }

    let current = current_build_commit();

    // Check cache first
    if let Some(mut cached_result) = get_cached_update_result() {
        if !cached_result.is_expired() {
            // If the cached latest commit matches our current commit, we've already updated!
            if cached_result.update_available
                && current != "unknown"
                && current == cached_result.latest_commit
            {
                cached_result.update_available = false;
                cache_update_result(false, &cached_result.latest_commit);
            } else if cached_result.update_available {
                notify_update_available(&cached_result.latest_commit);
            }
            return Ok(());
        }
    }

    // Perform fresh check
    let channel = UpdateChannel::Prod;
    match check_for_update_silent(channel) {
        Ok((update_available, latest)) => {
            cache_update_result(update_available, &latest);
            if update_available {
                notify_update_available(&latest);
            }
            Ok(())
        }
        Err(e) => {
            // Log error but don't fail
            eprintln!("Warning: Update check failed: {}", e);
            Ok(())
        }
    }
}

fn short_sha(value: &str) -> &str {
    if value.len() > 12 {
        &value[..12]
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_is_update_check_disabled_with_env_var() {
        // Set environment variable to disable
        env::set_var("SKM_CHECK_UPDATE", "false");
        assert!(is_update_check_disabled());
        env::remove_var("SKM_CHECK_UPDATE");
    }

    #[test]
    #[serial]
    fn test_is_update_check_disabled_without_env_var() {
        // Ensure env var is not set
        env::remove_var("SKM_CHECK_UPDATE");
        // Should not be disabled (assuming config check passes or defaults to true)
        // Note: This test may be affected by the actual config file
        // We're mainly testing that it doesn't panic
        let _ = is_update_check_disabled();
    }

    #[test]
    fn test_update_cache_struct() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let cache = UpdateCache {
            update_available: true,
            latest_commit: "dummy".to_string(),
            checked_at: now, // Just checked now
            ttl_seconds: 3600,
        };

        // Should not be expired (just checked)
        assert!(!cache.is_expired());
    }

    #[test]
    fn test_update_cache_expiration() {
        let cache = UpdateCache {
            update_available: true,
            latest_commit: "dummy".to_string(),
            checked_at: 0,
            ttl_seconds: 3600,
        };

        // Should be expired (checked_at is 0, TTL is 3600, now is much later)
        assert!(cache.is_expired());
    }

    #[test]
    fn test_check_for_update_silent_returns_bool() {
        // This test just verifies the function signature
        // Actual behavior depends on GitHub and current version
        let result = check_for_update_silent(UpdateChannel::Prod);
        // Should either return Ok(true) or Ok(false)
        assert!(result.is_ok());
    }
}
