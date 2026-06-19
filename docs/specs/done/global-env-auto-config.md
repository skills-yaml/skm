# Specification: Global Environment Auto-Configuration

## Overview

SKM must **always** check if the global environment is configured. If not, it must automatically create and configure it with default values, the same as what the `skm setup` command does.

## Problem Statement

Currently, SKM only performs first-time setup when both the base configuration and registries cache are missing. This means:
- If a user manually deletes their base config, SKM won't recreate it automatically
- Commands may fail if the base configuration is missing
- The behavior is inconsistent across commands

## Requirements

### R1: Always Ensure Base Configuration
SKM must ensure the global base configuration (`~/.config/skm/config.yaml`) exists before executing any command.

### R2: Auto-Create with Defaults
If the base configuration doesn't exist, SKM must automatically create it with the same default values used by the `skm setup` command:
- `default_registry`: `"default"`
- `registries`:
  - `default`: `"git@github.com:skills-yaml/skills-registry.git"`

### R3: Apply to All Commands
This check and auto-creation must apply to **all** SKM commands, with no exceptions.

### R4: Registries Cache is Separate
The registries cache update (cloning the default registry) should remain a separate concern:
- The base configuration must be created if missing
- The registries cache may be updated on-demand (via `skm cache-update`) or as part of first-time experience
- Commands that require cached registries should handle missing cache gracefully

### R5: Idempotent Operation
The base configuration creation must be idempotent:
- If config already exists, do nothing
- If config is corrupted, it should not be overwritten (fail with clear error)
- Multiple simultaneous SKM instances should not cause race conditions

## Implementation

### Modified Behavior Flow

```
1. User runs any SKM command (skm init, skm install, skm add, etc.)
2. SKM checks if base configuration exists (get_base_config_path())
3. If base configuration does NOT exist:
   a. Call init_base_config() to create with defaults
   b. Log: "Created global configuration at {path}"
4. Proceed with the requested command
```

### Code Changes Required

#### In `src/config_manager.rs`

Add a new function that only checks and creates base config (without cache):

```rust
/// Ensure base configuration exists, create with defaults if missing
/// This is separate from first_time_setup() which also updates cache
pub fn ensure_global_env() -> Result<(), Box<dyn std::error::Error>> {
    let path = get_base_config_path().ok_or("Could not determine config directory")?;
    
    if !path.exists() {
        println!("Initializing global SKM configuration...");
        init_base_config()?;
    }
    
    Ok(())
}
```

#### In `src/main.rs`

Replace the `is_first_time()` check with `ensure_global_env()`:

```rust
fn main() {
    let cli = Cli::parse();

    // Always ensure global environment is configured
    if let Err(e) = config_manager::ensure_global_env() {
        eprintln!("Warning: Failed to initialize global configuration: {}", e);
        eprintln!("SKM may not function correctly. Run 'skm setup' to manually configure.");
    }

    if let Err(e) = run(cli.command) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```

### Behavior Matrix

| Command | Current Behavior | New Behavior |
|---------|----------------|--------------|
| `skm setup` | Creates base config + cache | Creates base config + cache (unchanged) |
| `skm init` | Auto-setup on first time | Always ensures base config exists |
| `skm install` | Auto-setup on first time | Always ensures base config exists |
| `skm add` | Auto-setup on first time | Always ensures base config exists |
| `skm list` | Auto-setup on first time | Always ensures base config exists |
| `skm check` | Auto-setup on first time | Always ensures base config exists |
| `skm update` | No auto-setup | Always ensures base config exists |
| `skm cache-update` | No auto-setup | Always ensures base config exists |
| `skm init-config` | No auto-setup | Always ensures base config exists |

## Default Values

The base configuration created automatically must use these exact default values:

```yaml
default_registry: default
registries:
  default: git@github.com:skills-yaml/skills-registry.git
```

## Error Handling

1. If config directory cannot be determined: Print warning, continue with command
2. If base config cannot be created: Print warning, continue with command
3. If base config exists but is invalid YAML: Print error, suggest manual fix
4. Commands that require valid config should fail gracefully with helpful message

## Testing Requirements

1. Test that running any command with missing base config creates it
2. Test that running commands with existing base config doesn't overwrite it
3. Test that base config created automatically matches `skm setup` output
4. Test that commands work correctly after auto-creation
5. Test error cases (invalid YAML, missing config dir, etc.)

## Migration Path

This change is backward compatible:
- Existing users with base config: No change in behavior
- New users: Get automatic base config creation
- Users who deleted their config: Get it recreated automatically

## Related Functions

- `config_manager::init_base_config()` - Creates base config with defaults
- `config_manager::first_time_setup()` - Creates base config AND updates cache
- `config_manager::ensure_base_config()` - Returns base config, creating if needed
- `config_manager::is_first_time()` - Checks if both config and cache are missing (to be deprecated)
