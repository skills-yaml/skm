# Specification: Automatic Update Notification at Launch

## Overview

SKM should automatically check for and notify users about new versions at each launch. If a new version is available, SKM should ask the user if they want to update.

## Problem Statement

Currently, users must explicitly run `skm update --check` to check for new versions. This means many users may be running outdated versions of SKM without realizing it. Automatic notification ensures users are always aware of updates and can benefit from the latest features and bug fixes.

## Requirements

### R1: Automatic Check on Launch
SKM must automatically check for updates every time it is launched, before executing any command.

### R2: Silent Check
The update check must be performed silently (no output unless an update is available or an error occurs).

### R3: Notification When Update Available
When a new version is available, SKM must display a clear notification message before executing the requested command.

### R4: Interactive Update Prompt
After notifying about the update, SKM must ask the user if they want to update now.

### R5: Respect User Choice
- If user confirms (y/yes), run the update process
- If user declines (n/no), proceed with the requested command
- User choice should be remembered for the current session only (no persistent opt-out)

### R6: Use Production Channel by Default
The automatic check should use the production (`prod`) channel by default.

### R7: Graceful Failure
If the update check fails (network error, GitHub unavailable, etc.), SKM must:
- Display a warning message about the failure
- Continue with the requested command normally
- Not fail the entire operation

### R8: No Delay for Users
The update check should be performed asynchronously or with a reasonable timeout to avoid delaying command execution. If the check takes too long, proceed with the command and optionally show a message that the check is still in progress.

### R9: Rate Limiting
To avoid excessive GitHub API calls:
- Cache the update check result for a configurable period (default: 1 hour)
- Store cache in the SKM cache directory
- Respect the cached result if still valid

### R10: Configurable Behavior
Users should be able to disable automatic update checks via:
- Environment variable: `SKM_CHECK_UPDATE=false`
- Configuration file setting: `check_for_updates: false` in base config

## Implementation

### New Functions Required

#### In `src/updater.rs`

```rust
/// Check for update and notify user if available
/// Returns true if an update is available and user agreed to update
pub fn check_and_notify_update() -> Result<bool, Box<dyn std::error::Error>> {
    // Check if update check is disabled
    if is_update_check_disabled() {
        return Ok(false);
    }
    
    // Check cache first
    if let Some(cached_result) = get_cached_update_result() {
        if !cached_result.is_expired() {
            if cached_result.update_available {
                notify_update_available()?;
                return Ok(prompt_for_update());
            }
            return Ok(false);
        }
    }
    
    // Perform fresh check
    let channel = UpdateChannel::Prod;
    match check_for_update_silent(channel) {
        Ok(update_available) => {
            if update_available {
                cache_update_result(true);
                notify_update_available()?;
                Ok(prompt_for_update())
            } else {
                cache_update_result(false);
                Ok(false)
            }
        }
        Err(e) => {
            // Log error but don't fail
            eprintln!("Warning: Update check failed: {}", e);
            Ok(false)
        }
    }
}

/// Silent version of check_for_update that doesn't print status messages
pub fn check_for_update_silent(channel: UpdateChannel) -> Result<bool, Box<dyn std::error::Error>> {
    let current = current_build_commit();
    let latest = latest_release_commit(channel)?;
    
    if current == "unknown" {
        return Ok(true);
    }
    
    Ok(current != latest)
}

/// Display update notification message
fn notify_update_available() -> Result<(), Box<dyn std::error::Error>> {
    let current = current_build_commit();
    let latest = latest_release_commit(UpdateChannel::Prod)?;
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  New SKM version available!");
    println!("  Current: {}", &current[..12.min(current.len())]);
    println!("  Latest:  {}", &latest[..12.min(latest.len())]);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    Ok(())
}

/// Prompt user to update
fn prompt_for_update() -> bool {
    use std::io::{self, Write};
    
    print!("\nWould you like to update now? [y/N] ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Check if update check is disabled
fn is_update_check_disabled() -> bool {
    // Check environment variable
    if std::env::var("SKM_CHECK_UPDATE") == Ok("false".to_string()) {
        return true;
    }
    
    // Check base config
    if let Ok(base_config) = BaseConfig::load() {
        // This would require adding a field to BaseConfig
        // For now, just check env var
    }
    
    false
}

/// Cache structure and functions for update check results
mod cache {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    pub struct UpdateCache {
        pub update_available: bool,
        pub checked_at: u64, // timestamp
        pub ttl_seconds: u64, // default: 3600 (1 hour)
    }
    
    impl UpdateCache {
        pub fn is_expired(&self) -> bool {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now > self.checked_at + self.ttl_seconds
        }
    }
    
    pub fn get_cache_path() -> Option<PathBuf> {
        dirs::cache_dir().map(|d| d.join("skm").join("update_cache.json"))
    }
    
    pub fn get_cached_update_result() -> Option<UpdateCache> {
        let path = get_cache_path()?;
        if !path.exists() {
            return None;
        }
        
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }
    
    pub fn cache_update_result(update_available: bool) {
        let path = get_cache_path()?;
        
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        
        let cache = UpdateCache {
            update_available,
            checked_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: 3600, // 1 hour
        };
        
        let _ = fs::write(&path, serde_json::to_string(&cache).unwrap());
    }
}
```

### Changes to `src/main.rs`

Update the main function to check for updates at launch:

```rust
use updater::check_and_notify_update;

fn main() {
    let cli = Cli::parse();

    // Always ensure global environment is configured
    if let Err(e) = ensure_global_env() {
        eprintln!("Warning: Failed to initialize global configuration: {}", e);
        eprintln!("SKM may not function correctly. Run 'skm setup' to manually configure.");
    }

    // Check for updates at launch
    if let Ok(should_update) = check_and_notify_update() {
        if should_update {
            // User agreed to update, run update and exit
            if let Err(e) = updater::install_update(UpdateChannel::Prod) {
                eprintln!("Update failed: {}", e);
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }

    if let Err(e) = run(cli.command) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```

### Changes to `src/config_manager.rs`

Add a new field to `BaseConfig` to allow users to disable update checks:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseConfig {
    pub default_registry: String,
    pub registries: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub check_for_updates: bool,
}

fn default_true() -> bool {
    true
}
```

## Behavior Flow

```
1. User runs any SKM command (e.g., `skm init`)
2. SKM checks if update check is disabled (env var or config)
3. If disabled, proceed to command execution
4. If enabled:
   a. Check cache for recent update check result
   b. If cache is valid and no update, proceed to command
   c. If cache is expired or missing, perform fresh check
   d. If update available:
      - Display notification with current and latest versions
      - Prompt user: "Would you like to update now? [y/N]"
      - If user says yes:
         * Run update process
         * Exit SKM after update completes
      - If user says no:
         * Cache the result (update available = true)
         * Proceed with requested command
   e. If no update available:
      - Cache the result (update available = false)
      - Proceed with requested command
5. Execute the requested command
```

## Configuration

### Environment Variable
```bash
# Disable update checks
SKM_CHECK_UPDATE=false skm init
```

### Configuration File
```yaml
# ~/.config/skm/config.yaml
default_registry: default
registries:
  default: git@github.com:skills-yaml/skills-registry.git
check_for_updates: false  # Disable automatic update checks
```

## Testing Requirements

1. Test that update notification appears when new version is available
2. Test that user can accept update and it installs correctly
3. Test that user can decline update and command proceeds normally
4. Test that update check is skipped when disabled via environment variable
5. Test that update check is cached and not performed repeatedly within TTL
6. Test that update check failure doesn't break command execution
7. Test that silent check produces no output when up to date

## Migration Path

This change is backward compatible:
- Existing users will start seeing update notifications
- No breaking changes to existing commands
- Users can disable the feature if desired

## Performance Considerations

- The update check involves a GitHub API call (git ls-remote)
- With caching (1 hour TTL), this should have minimal impact
- The check is performed synchronously but should be fast (< 1 second in most cases)
- If network is slow/unavailable, the check will timeout and continue with the command

## User Experience

Example output when update is available:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  New SKM version available!
  Current: a1b2c3d4e5f6
  Latest:  x9y8z7w6v5u4
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Would you like to update now? [y/N] 
```

## Related Functions

- `updater::check_for_update()` - Existing function, modified for silent mode
- `updater::install_update()` - Existing function, used for actual update
- `updater::check_and_notify_update()` - New function, main entry point
- `BaseConfig` - Extended with `check_for_updates` field
