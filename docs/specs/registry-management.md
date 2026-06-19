# Specification: Registry Management Feature

## Overview

SKM needs the ability to manage skill registries beyond the initial setup. Users should be able to add, remove, list, and update custom registries to support different skill sources (public, private, team-specific, etc.).

## Problem Statement

Currently, registries can only be configured:
1. During first-time setup (`skm setup`)
2. During project initialization (`skm init --interactive`)
3. By manually editing `~/.config/skm/config.yaml`

This makes it difficult to:
- Add private/organization registries after initial setup
- Remove registries that are no longer needed
- Inspect what registries are currently configured
- Update individual registries independently

## Requirements

### R1: Add Registry
SKM must allow adding a new registry with a name and URL.

### R2: Remove Registry
SKM must allow removing an existing registry.

### R3: List Registries
SKM must display all configured registries with their URLs and status.

### R4: Update Registry
SKM must allow updating a specific registry (git pull).

### R5: Validate Registry
SKM must validate registry names and URLs before adding.

### R6: Default Registry Protection
SKM must prevent removal of the default registry or warn the user.

### R7: Registry Status
SKM must show whether each registry is cached and up-to-date.

### R8: Duplicate Prevention
SKM must prevent adding a registry with a name that already exists.

### R9: URL Validation
SKM must validate that registry URLs are valid Git repositories.

### R10: Registry Switching
SKM must allow changing the default registry.

## Command Specifications

### 1. Add Registry

```
Command: skm registry add <NAME> <URL> [OPTIONS]

Description: Add a new skill registry

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--set-default` | Set this registry as the default | Flag | false |
| `--skip-validate` | Skip URL validation | Flag | false |
| `--json` | Output in JSON format | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `NAME` | Name for the new registry | Yes | String |
| `URL` | Git URL of the registry | Yes | String |

Examples:
```bash
# Add a custom registry
$ skm registry add company git@github.com:my-company/skills.git

# Add and set as default
$ skm registry add company git@github.com:my-company/skills.git --set-default

# Add without validation (for private repos)
$ skm registry add private git@private.example.com/skills.git --skip-validate
```
```

### 2. Remove Registry

```
Command: skm registry remove <NAME> [OPTIONS]

Description: Remove a skill registry

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--force`, `-f` | Force removal even if it's the default | Flag | false |
| `--yes`, `-y` | Skip confirmation | Flag | false |
| `--dry-run` | Preview what would be removed | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `NAME` | Name of the registry to remove | Yes | String |

Examples:
```bash
# Remove a registry
$ skm registry remove company

# Force remove default registry
$ skm registry remove default --force

# Skip confirmation
$ skm registry remove company --yes
```
```

### 3. List Registries

```
Command: skm registry list [OPTIONS]

Description: List all configured registries

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--json` | Output in JSON format | Flag | false |
| `--verbose`, `-v` | Show detailed information | Flag | false |

Examples:
```bash
# List all registries
$ skm registry list

# Output:
# Default: default
#   URL: git@github.com:skills-yaml/skills-registry.git
#   Cached: Yes (up-to-date)
# 
# Registry: company
#   URL: git@github.com:my-company/skills.git
#   Cached: Yes (outdated)

# JSON output
$ skm registry list --json
```
```

### 4. Update Registry

```
Command: skm registry update <NAME> [OPTIONS]

Description: Update a specific registry cache

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--all` | Update all registries | Flag | false |
| `--force` | Force update even if already up-to-date | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `NAME` | Name of the registry to update | No* | String |

*Required unless --all is specified

Examples:
```bash
# Update a specific registry
$ skm registry update company

# Update all registries
$ skm registry update --all

# Force update
$ skm registry update company --force
```
```

### 5. Set Default Registry

```
Command: skm registry default <NAME>

Description: Set the default registry

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `NAME` | Name of the registry to set as default | Yes | String |

Examples:
```bash
# Set company as the default registry
$ skm registry default company
```
```

### 6. Show Registry Info

```
Command: skm registry info <NAME> [OPTIONS]

Description: Show detailed information about a registry

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--json` | Output in JSON format | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `NAME` | Name of the registry | Yes | String |

Examples:
```bash
# Show registry info
$ skm registry info company

# Output:
# Name: company
# URL: git@github.com:my-company/skills.git
# Default: No
# Cached: Yes
# Cache Path: ~/.cache/skm/registries/company/skills
# Last Updated: 2024-06-15T10:30:00Z
# Skills: 25
```
```

## Implementation

### Files to Modify

#### `src/main.rs`
Add registry commands to the `Commands` enum:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Manage skill registries
    #[command(subcommand)]
    Registry(RegistryCommands),
}

#[derive(Subcommand)]
enum RegistryCommands {
    /// Add a new skill registry
    Add {
        /// Name for the new registry
        name: String,
        /// Git URL of the registry
        url: String,
        /// Set this registry as the default
        #[arg(long)]
        set_default: bool,
        /// Skip URL validation
        #[arg(long)]
        skip_validate: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    
    /// Remove a skill registry
    Remove {
        /// Name of the registry to remove
        name: String,
        /// Force removal even if it's the default
        #[arg(short, long)]
        force: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Preview what would be removed
        #[arg(long)]
        dry_run: bool,
    },
    
    /// List all configured registries
    List {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Update a registry cache
    Update {
        /// Name of the registry to update
        name: Option<String>,
        /// Update all registries
        #[arg(long)]
        all: bool,
        /// Force update even if already up-to-date
        #[arg(long)]
        force: bool,
    },
    
    /// Set the default registry
    Default {
        /// Name of the registry to set as default
        name: String,
    },
    
    /// Show detailed registry information
    Info {
        /// Name of the registry
        name: String,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
}
```

Add handler in `run()` function:

```rust
Commands::Registry(cmd) => {
    match cmd {
        RegistryCommands::Add { name, url, set_default, skip_validate, json } => {
            registry::add(name, url, set_default, skip_validate, json)?;
        }
        RegistryCommands::Remove { name, force, yes, dry_run } => {
            registry::remove(name, force, yes, dry_run)?;
        }
        RegistryCommands::List { json, verbose } => {
            registry::list(json, verbose)?;
        }
        RegistryCommands::Update { name, all, force } => {
            if all {
                registry::update_all(force)?;
            } else if let Some(name) = name {
                registry::update(name, force)?;
            } else {
                return Err("Must specify a registry name or use --all".into());
            }
        }
        RegistryCommands::Default { name } => {
            registry::set_default(name)?;
        }
        RegistryCommands::Info { name, json } => {
            registry::info(name, json)?;
        }
    }
    Ok(())
}
```

#### `src/registry.rs` (New File)
Create a dedicated registry management module:

```rust
use crate::config_manager::{BaseConfig, get_base_config_path};
use std::fs;
use std::path::PathBuf;

/// Registry information
#[derive(Debug, Clone)]
pub struct RegistryInfo {
    pub name: String,
    pub url: String,
    pub is_default: bool,
    pub is_cached: bool,
    pub is_up_to_date: bool,
    pub cache_path: Option<PathBuf>,
    pub skill_count: Option<usize>,
    pub last_updated: Option<String>,
}

/// Add a new registry
pub fn add(
    name: String,
    url: String,
    set_default: bool,
    skip_validate: bool,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate registry name
    if !is_valid_registry_name(&name) {
        return Err(format!("Invalid registry name: '{}'", name).into());
    }
    
    // Check if registry already exists
    let mut config = BaseConfig::load()?;
    if config.registries.contains_key(&name) {
        return Err(format!("Registry '{}' already exists", name).into());
    }
    
    // Validate URL if not skipped
    if !skip_validate {
        validate_registry_url(&url)?;
    }
    
    // Add registry
    config.registries.insert(name.clone(), url);
    
    // Set as default if requested
    if set_default {
        config.default_registry = name;
    }
    
    // Save config
    config.save()?;
    
    // Output result
    if json_output {
        println!("{{" \"name\": \"{}\", \"url\": \"{}\", \"default\": {} }}", name, url, set_default);
    } else {
        println!("Added registry '{}' with URL: {}", name, url);
        if set_default {
            println!("Set as default registry");
        }
    }
    
    Ok(())
}

/// Remove a registry
pub fn remove(
    name: String,
    force: bool,
    yes: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = BaseConfig::load()?;
    
    // Check if registry exists
    if !config.registries.contains_key(&name) {
        return Err(format!("Registry '{}' not found", name).into());
    }
    
    // Check if it's the default registry
    let is_default = config.default_registry == name;
    
    if is_default && !force {
        return Err(format!(
            "Registry '{}' is the default registry. Use --force to remove it.",
            name
        ).into());
    }
    
    if dry_run {
        println!("Would remove registry: {}", name);
        if is_default {
            println!("Warning: This is the default registry");
        }
        return Ok(());
    }
    
    // Confirm with user if not --yes
    if !yes {
        if is_default {
            print!("Registry '{}' is the default. Remove anyway? [y/N] ", name);
        } else {
            print!("Remove registry '{}'? [y/N] ", name);
        }
        std::io::stdout().flush()?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Removal cancelled.");
            return Ok(());
        }
    }
    
    // Remove registry
    config.registries.remove(&name);
    
    // If it was the default, set a new default
    if is_default {
        if config.registries.is_empty() {
            // Reset to default
            config.default_registry = "default".to_string();
            config.registries.insert("default".to_string(), crate::config_manager::DEFAULT_REGISTRY_URL.to_string());
        } else {
            // Set first registry as default
            config.default_registry = config.registries.keys().next().unwrap().clone();
        }
    }
    
    // Save config
    config.save()?;
    
    // Also remove cache if it exists
    if let Some(path) = crate::linker::resolve_registry_path(&name) {
        if path.exists() {
            fs::remove_dir_all(&path)?;
            println!("Removed cache directory: {}", path.display());
        }
    }
    
    println!("Removed registry: {}", name);
    
    Ok(())
}

/// List all registries
pub fn list(json_output: bool, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = BaseConfig::load()?;
    let mut registries: Vec<RegistryInfo> = Vec::new();
    
    for (name, url) in &config.registries {
        let is_default = *name == config.default_registry;
        let cache_path = crate::linker::resolve_registry_path(name);
        let is_cached = cache_path.as_ref().map_or(false, |p| p.exists());
        let is_up_to_date = if is_cached {
            // Check if cache is up-to-date (simplified)
            true // Would need git status check
        } else {
            false
        };
        let skill_count = if is_cached {
            count_skills(cache_path.unwrap()).ok()
        } else {
            None
        };
        
        registries.push(RegistryInfo {
            name: name.clone(),
            url: url.clone(),
            is_default,
            is_cached,
            is_up_to_date,
            cache_path,
            skill_count,
            last_updated: None,
        });
    }
    
    if json_output {
        // Output as JSON
        let json = serde_json::to_string_pretty(&registries)
            .map_err(|e| format!("Failed to serialize registries: {}", e))?;
        println!("{}", json);
    } else if verbose {
        // Detailed output
        for reg in &registries {
            println!("Registry: {}", reg.name);
            println!("  URL: {}", reg.url);
            println!("  Default: {}", if reg.is_default { "Yes" } else { "No" });
            println!("  Cached: {}", if reg.is_cached { "Yes" } else { "No" });
            if let Some(count) = reg.skill_count {
                println!("  Skills: {}", count);
            }
            println!();
        }
    } else {
        // Simple output
        for reg in &registries {
            let default_marker = if reg.is_default { " (default)" } else { "" };
            let cached_marker = if reg.is_cached { " (cached)" } else { "" };
            println!("{}{}{}: {}", reg.name, default_marker, cached_marker, reg.url);
        }
    }
    
    Ok(())
}

/// Update a specific registry
pub fn update(name: String, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = BaseConfig::load()?;
    
    // Check if registry exists
    if !config.registries.contains_key(&name) {
        return Err(format!("Registry '{}' not found", name).into());
    }
    
    let url = &config.registries[&name];
    let cache_path = crate::linker::resolve_registry_path(&name)
        .ok_or_else(|| format!("Could not resolve cache path for registry: {}", name))?;
    
    if !cache_path.exists() {
        // Clone new registry
        println!("Cloning registry '{}' from '{}'...", name, url);
        clone_registry(url, &cache_path)?;
    } else if force {
        // Force update (pull)
        println!("Updating registry '{}'...", name);
        update_registry(&cache_path)?;
    } else {
        // Check if update is needed
        if !is_registry_up_to_date(&cache_path)? {
            println!("Updating registry '{}'...", name);
            update_registry(&cache_path)?;
        } else {
            println!("Registry '{}' is already up-to-date", name);
        }
    }
    
    println!("Registry '{}' updated successfully", name);
    
    Ok(())
}

/// Update all registries
pub fn update_all(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = BaseConfig::load()?;
    let mut updated = 0;
    let mut skipped = 0;
    let mut failed = 0;
    
    for (name, _) in &config.registries {
        match update(name.clone(), force) {
            Ok(_) => updated += 1,
            Err(e) if e.to_string().contains("already up-to-date") => skipped += 1,
            Err(e) => {
                eprintln!("Failed to update registry '{}': {}", name, e);
                failed += 1;
            }
        }
    }
    
    println!("\nSummary:");
    println!("  Updated: {}", updated);
    println!("  Skipped: {}", skipped);
    println!("  Failed: {}", failed);
    
    if failed > 0 {
        return Err(format!("Failed to update {} registries", failed).into());
    }
    
    Ok(())
}

/// Set default registry
pub fn set_default(name: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = BaseConfig::load()?;
    
    // Check if registry exists
    if !config.registries.contains_key(&name) {
        return Err(format!("Registry '{}' not found", name).into());
    }
    
    config.default_registry = name;
    config.save()?;
    
    println!("Default registry set to: {}", name);
    
    Ok(())
}

/// Show registry info
pub fn info(name: String, json_output: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = BaseConfig::load()?;
    
    // Check if registry exists
    let url = config.registries.get(&name)
        .ok_or_else(|| format!("Registry '{}' not found", name).into())?;
    
    let is_default = config.default_registry == name;
    let cache_path = crate::linker::resolve_registry_path(&name);
    let is_cached = cache_path.as_ref().map_or(false, |p| p.exists());
    let skill_count = if is_cached {
        count_skills(cache_path.unwrap()).ok()
    } else {
        None
    };
    
    let info = RegistryInfo {
        name: name.clone(),
        url: url.clone(),
        is_default,
        is_cached,
        is_up_to_date: false,
        cache_path,
        skill_count,
        last_updated: None,
    };
    
    if json_output {
        let json = serde_json::to_string_pretty(&info)
            .map_err(|e| format!("Failed to serialize registry info: {}", e))?;
        println!("{}", json);
    } else {
        println!("Name: {}", info.name);
        println!("URL: {}", info.url);
        println!("Default: {}", if info.is_default { "Yes" } else { "No" });
        println!("Cached: {}", if info.is_cached { "Yes" } else { "No" });
        if let Some(path) = &info.cache_path {
            println!("Cache Path: {}", path.display());
        }
        if let Some(count) = info.skill_count {
            println!("Skills: {}", count);
        }
    }
    
    Ok(())
}

/// Helper functions

fn is_valid_registry_name(name: &str) -> bool {
    !name.is_empty() &&
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') &&
    !name.starts_with('.') &&
    name != "default"
}

fn validate_registry_url(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Check if it looks like a git URL
    if url.starts_with("git@") || url.starts_with("https://") || url.starts_with("http://") {
        // Try to validate by cloning (shallow, just to check if it exists)
        // This is a simplified check
        Ok(())
    } else {
        Err(format!("Invalid Git URL: {}", url).into())
    }
}

fn clone_registry(url: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create parent directory
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Clone the repository
    let output = std::process::Command::new("git")
        .args(["clone", url, path.to_str().unwrap()])
        .output()?;
    
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to clone registry: {}", err).into());
    }
    
    Ok(())
}

fn update_registry(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(["-C", path.to_str().unwrap(), "pull"])
        .output()?;
    
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to update registry: {}", err).into());
    }
    
    Ok(())
}

fn is_registry_up_to_date(path: &Path) -> bool {
    let output = std::process::Command::new("git")
        .args(["-C", path.to_str().unwrap(), "status", "--porcelain"])
        .output();
    
    match output {
        Ok(output) => output.stdout.is_empty(),
        Err(_) => false,
    }
}

fn count_skills(path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    let skills_dir = path.join("skills");
    if !skills_dir.exists() {
        return Ok(0);
    }
    
    Ok(fs::read_dir(&skills_dir)?.count())
}
```

#### `src/config_manager.rs`
No changes needed - BaseConfig already supports multiple registries.

## Behavior Flow

### Add Registry
```
1. User runs: skm registry add company git@github.com:my-company/skills.git
2. SKM validates registry name
3. SKM checks if registry already exists
4. SKM validates URL (unless --skip-validate)
5. SKM adds registry to BaseConfig
6. SKM saves updated config
7. SKM prints confirmation
```

### Remove Registry
```
1. User runs: skm registry remove company
2. SKM checks if registry exists
3. SKM checks if it's the default registry
4. If --dry-run: Show what would be removed
5. If --yes: Skip confirmation
6. If not --yes: Prompt user for confirmation
7. SKM removes registry from BaseConfig
8. If it was default: Set new default
9. SKM removes cache directory
10. SKM saves updated config
11. SKM prints confirmation
```

### List Registries
```
1. User runs: skm registry list
2. SKM loads BaseConfig
3. SKM gathers registry information
4. SKM formats output (text or JSON)
5. SKM prints registry list
```

### Update Registry
```
1. User runs: skm registry update company
2. SKM checks if registry exists
3. SKM checks if cache exists
4. If not cached: Clone registry
5. If cached: Pull updates (or skip if up-to-date)
6. SKM prints status
```

### Set Default Registry
```
1. User runs: skm registry default company
2. SKM checks if registry exists
3. SKM updates default_registry in BaseConfig
4. SKM saves config
5. SKM prints confirmation
```

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Registry name invalid | Clear error with valid format |
| Registry already exists | Clear error message |
| Registry not found | Clear error message |
| URL invalid | Clear error with URL format requirements |
| Permission denied | Clear error with path |
| Git clone/pull fails | Clear error with Git output |
| Cannot remove default without --force | Clear error with instructions |

## Testing Requirements

### Add Registry
1. Test adding a valid registry
2. Test adding with --set-default
3. Test adding with invalid name
4. Test adding duplicate registry
5. Test adding with --skip-validate
6. Test JSON output

### Remove Registry
1. Test removing a non-default registry
2. Test removing with --yes
3. Test removing with --dry-run
4. Test removing default registry without --force (should fail)
5. Test removing default registry with --force
6. Test removing non-existent registry

### List Registries
1. Test listing with no registries
2. Test listing with multiple registries
3. Test verbose output
4. Test JSON output

### Update Registry
1. Test updating a cached registry
2. Test updating with --all
3. Test updating non-existent registry
4. Test force update

### Set Default
1. Test setting valid registry as default
2. Test setting non-existent registry as default

### Info
1. Test info for existing registry
2. Test info for non-existent registry
3. Test JSON output

## Migration Path

This feature is fully backward compatible:
- Existing configurations are unaffected
- No changes to existing commands
- New commands are additive
- BaseConfig already supports multiple registries

## User Experience

### Add Registry Output
```
$ skm registry add company git@github.com:my-company/skills.git

Added registry 'company' with URL: git@github.com:my-company/skills.git

To use this registry:
  skm add my-skill --source company
```

### List Registries Output
```
$ skm registry list

default (default): git@github.com:skills-yaml/skills-registry.git (cached)
company: git@github.com:my-company/skills.git (cached)
private: git@private.example.com/skills.git
```

### Verbose List Output
```
$ skm registry list --verbose

Registry: default
  URL: git@github.com:skills-yaml/skills-registry.git
  Default: Yes
  Cached: Yes
  Skills: 42

Registry: company
  URL: git@github.com:my-company/skills.git
  Default: No
  Cached: Yes
  Skills: 15

Registry: private
  URL: git@private.example.com/skills.git
  Default: No
  Cached: No
```

### JSON Output
```json
[
  {
    "name": "default",
    "url": "git@github.com:skills-yaml/skills-registry.git",
    "is_default": true,
    "is_cached": true,
    "is_up_to_date": true,
    "cache_path": "/home/user/.cache/skm/registries/default",
    "skill_count": 42,
    "last_updated": null
  },
  {
    "name": "company",
    "url": "git@github.com:my-company/skills.git",
    "is_default": false,
    "is_cached": true,
    "is_up_to_date": false,
    "cache_path": "/home/user/.cache/skm/registries/company",
    "skill_count": 15,
    "last_updated": null
  }
]
```

### Remove Registry Output
```
$ skm registry remove company

Remove registry 'company'? [y/N] y
Removed registry: company
Removed cache directory: /home/user/.cache/skm/registries/company
```

## Related Features

- `skm add --source` - Uses registries for skill sources
- `skm cache-update` - Updates all registries
- `skm setup` - Initializes with default registry
- BaseConfig - Stores registry configuration
