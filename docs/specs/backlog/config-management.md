# Specification: Configuration Management Feature

## Overview

SKM needs a dedicated set of commands for managing its configuration programmatically. Currently, users must manually edit YAML files to change settings, which is error-prone and not user-friendly.

## Problem Statement

Currently, configuration can only be modified by:
1. Editing `~/.config/skm/config.yaml` manually
2. Editing `./skills.yaml` manually
3. Using the interactive `skm init` command
4. Running `skm setup` for base configuration

This makes it difficult to:
- Automate configuration changes
- Script SKM operations
- Query current configuration values
- Manage configuration in CI/CD environments
- Make small, targeted changes to configuration

## Requirements

### Configuration Query

#### R1: Get Configuration Value
SKM must provide a command to get a specific configuration value.

#### R2: Show Full Configuration
SKM must display the entire configuration in a readable format.

#### R3: Multiple Output Formats
SKM must support plain text, JSON, and YAML output formats.

### Configuration Modification

#### R4: Set Configuration Value
SKM must allow setting a configuration value by key.

#### R5: Unset Configuration Value
SKM must allow removing a configuration value.

#### R6: Reset Configuration
SKM must allow resetting configuration to defaults.

### Configuration Validation

#### R7: Validate Configuration
SKM must validate configuration changes before applying them.

#### R8: Dry Run Mode
SKM must support previewing changes without applying them.

### Scope Management

#### R9: Global vs Project Configuration
SKM must distinguish between global and project-specific configuration.

#### R10: Scope Indication
SKM must clearly indicate which scope a configuration value belongs to.

### Safety & UX

#### R11: Confirmation for Destructive Changes
SKM must prompt for confirmation before resetting or removing values.

#### R12: Backup Before Changes
SKM must create backups before making changes (optional).

#### R13: Clear Error Messages
SKM must provide actionable error messages for invalid configuration.

## Command Specifications

### 1. Get Configuration Value

```
Command: skm config get <KEY> [OPTIONS]

Description: Get a configuration value

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Get from global configuration | Flag | false |
| `--project`, `-p` | Get from project configuration | Flag | false |
| `--json` | Output in JSON format | Flag | false |
| `--default` | Show default value if key not found | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `KEY` | Configuration key (dot notation for nested keys) | Yes | String |

Examples:
```bash
# Get a value from project config
$ skm config get name

# Get a value from global config
$ skm config get default_registry --global

# Get with default fallback
$ skm config get non_existent_key --default "fallback"

# JSON output
$ skm config get registries --json
```
```

### 2. Set Configuration Value

```
Command: skm config set <KEY> <VALUE> [OPTIONS]

Description: Set a configuration value

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Set in global configuration | Flag | false |
| `--project`, `-p` | Set in project configuration | Flag | false |
| `--json` | Parse VALUE as JSON | Flag | false |
| `--dry-run` | Preview changes without applying | Flag | false |
| `--yes`, `-y` | Skip confirmation for sensitive changes | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `KEY` | Configuration key | Yes | String |
| `VALUE` | Value to set | Yes | String |

Examples:
```bash
# Set project name
$ skm config set name my-project

# Set in global config
$ skm config set check_for_updates false --global

# Set JSON value
$ skm config set registries.my-registry "git@github.com:example/skills.git" --json

# Preview changes
$ skm config set name new-name --dry-run
```
```

### 3. Unset Configuration Value

```
Command: skm config unset <KEY> [OPTIONS]

Description: Remove a configuration value

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Unset from global configuration | Flag | false |
| `--project`, `-p` | Unset from project configuration | Flag | false |
| `--dry-run` | Preview changes without applying | Flag | false |
| `--yes`, `-y` | Skip confirmation | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `KEY` | Configuration key to remove | Yes | String |

Examples:
```bash
# Remove a project configuration key
$ skm config unset version

# Remove with confirmation
$ skm config unset registries.my-registry --global
```
```

### 4. Show Full Configuration

```
Command: skm config show [OPTIONS]

Description: Display the full configuration

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Show global configuration | Flag | false |
| `--project`, `-p` | Show project configuration | Flag | false |
| `--all` | Show both global and project configurations | Flag | false |
| `--json` | Output in JSON format | Flag | false |
| `--yaml` | Output in YAML format | Flag | false |
| `--paths` | Show configuration file paths | Flag | false |

Examples:
```bash
# Show project configuration
$ skm config show

# Show global configuration
$ skm config show --global

# Show both configurations
$ skm config show --all

# JSON output
$ skm config show --json

# Show config file paths
$ skm config show --paths
```
```

### 5. Reset Configuration

```
Command: skm config reset [OPTIONS]

Description: Reset configuration to defaults

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Reset global configuration | Flag | false |
| `--project`, `-p` | Reset project configuration | Flag | false |
| `--yes`, `-y` | Skip confirmation | Flag | false |
| `--dry-run` | Preview what would be reset | Flag | false |

Examples:
```bash
# Reset project configuration
$ skm config reset --project

# Reset global configuration with confirmation
$ skm config reset --global

# Preview reset
$ skm config reset --dry-run
```
```

### 6. Validate Configuration

```
Command: skm config validate [OPTIONS]

Description: Validate configuration files

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Validate global configuration | Flag | false |
| `--project`, `-p` | Validate project configuration | Flag | false |
| `--all` | Validate all configurations | Flag | false |
| `--strict` | Perform strict validation | Flag | false |

Examples:
```bash
# Validate project configuration
$ skm config validate

# Validate all configurations
$ skm config validate --all

# Strict validation
$ skm config validate --strict
```
```

## Implementation

### Files to Modify

#### `src/main.rs`
Add config commands to the `Commands` enum:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Manage SKM configuration
    #[command(subcommand)]
    Config(ConfigCommands),
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Get a configuration value
    Get {
        /// Configuration key (supports dot notation)
        key: String,
        /// Get from global configuration
        #[arg(short, long)]
        global: bool,
        /// Get from project configuration
        #[arg(short, long)]
        project: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show default value if key not found
        #[arg(long)]
        default: bool,
    },
    
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Value to set
        value: String,
        /// Set in global configuration
        #[arg(short, long)]
        global: bool,
        /// Set in project configuration
        #[arg(short, long)]
        project: bool,
        /// Parse value as JSON
        #[arg(long)]
        json: bool,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation for sensitive changes
        #[arg(short, long)]
        yes: bool,
    },
    
    /// Remove a configuration value
    Unset {
        /// Configuration key to remove
        key: String,
        /// Unset from global configuration
        #[arg(short, long)]
        global: bool,
        /// Unset from project configuration
        #[arg(short, long)]
        project: bool,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
    
    /// Show full configuration
    Show {
        /// Show global configuration
        #[arg(short, long)]
        global: bool,
        /// Show project configuration
        #[arg(short, long)]
        project: bool,
        /// Show both configurations
        #[arg(long)]
        all: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Output in YAML format
        #[arg(long)]
        yaml: bool,
        /// Show configuration file paths
        #[arg(long)]
        paths: bool,
    },
    
    /// Reset configuration to defaults
    Reset {
        /// Reset global configuration
        #[arg(short, long)]
        global: bool,
        /// Reset project configuration
        #[arg(short, long)]
        project: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Preview what would be reset
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Validate configuration files
    Validate {
        /// Validate global configuration
        #[arg(short, long)]
        global: bool,
        /// Validate project configuration
        #[arg(short, long)]
        project: bool,
        /// Validate all configurations
        #[arg(long)]
        all: bool,
        /// Perform strict validation
        #[arg(long)]
        strict: bool,
    },
}
```

Add handler in `run()` function:

```rust
Commands::Config(cmd) => {
    match cmd {
        ConfigCommands::Get { key, global, project, json, default } => {
            config::get_value(&key, global, project, json, default)?;
        }
        ConfigCommands::Set { key, value, global, project, json: parse_json, dry_run, yes } => {
            config::set_value(&key, &value, global, project, parse_json, dry_run, yes)?;
        }
        ConfigCommands::Unset { key, global, project, dry_run, yes } => {
            config::unset_value(&key, global, project, dry_run, yes)?;
        }
        ConfigCommands::Show { global, project, all, json, yaml, paths } => {
            config::show_config(global, project, all, json, yaml, paths)?;
        }
        ConfigCommands::Reset { global, project, yes, dry_run } => {
            config::reset_config(global, project, yes, dry_run)?;
        }
        ConfigCommands::Validate { global, project, all, strict } => {
            config::validate_config(global, project, all, strict)?;
        }
    }
    Ok(())
}
```

#### `src/config.rs` (Extend)
Add configuration management functions:

```rust
use serde_yaml::Value;
use std::collections::HashMap;

/// Configuration scope
enum ConfigScope {
    Project,
    Global,
}

impl ConfigScope {
    fn path(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        match self {
            ConfigScope::Project => {
                let current_dir = std::env::current_dir()?;
                Ok(current_dir.join("skills.yaml"))
            }
            ConfigScope::Global => {
                get_base_config_path().ok_or("Could not determine config directory".into())
            }
        }
    }
}

/// Get a configuration value
pub fn get_value(
    key: &str,
    global: bool,
    project: bool,
    json_output: bool,
    show_default: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = if global {
        ConfigScope::Global
    } else if project {
        ConfigScope::Project
    } else {
        // Default to project if neither specified
        ConfigScope::Project
    };
    
    let path = scope.path()?;
    
    if !path.exists() {
        if show_default {
            // Try to get default value
            let default = get_default_value(key);
            if json_output {
                println!("{}", serde_json::to_string(&default)?);
            } else {
                println!("{}", default);
            }
            return Ok(());
        }
        return Err(format!("Configuration file not found: {}", path.display()).into());
    }
    
    let content = std::fs::read_to_string(&path)?;
    let value: Value = serde_yaml::from_str(&content)?;
    
    let result = get_nested_value(&value, key);
    
    match result {
        Some(v) => {
            if json_output {
                println!("{}", serde_json::to_string(&v)?);
            } else {
                println!("{}", serde_yaml::to_string(&v)?);
            }
            Ok(())
        }
        None => {
            if show_default {
                let default = get_default_value(key);
                if json_output {
                    println!("{}", serde_json::to_string(&default)?);
                } else {
                    println!("{}", default);
                }
                Ok(())
            } else {
                Err(format!("Key '{}' not found in configuration", key).into())
            }
        }
    }
}

/// Set a configuration value
pub fn set_value(
    key: &str,
    value: &str,
    global: bool,
    project: bool,
    parse_json: bool,
    dry_run: bool,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = if global {
        ConfigScope::Global
    } else if project {
        ConfigScope::Project
    } else {
        ConfigScope::Project
    };
    
    let path = scope.path()?;
    
    // Parse value
    let parsed_value: Value = if parse_json {
        serde_json::from_str(value)?
    } else {
        // Try to parse as YAML, fall back to string
        serde_yaml::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()))
    };
    
    // Load existing config or create new
    let mut config: Value = if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        serde_yaml::from_str(&content)?
    } else {
        Value::Mapping(serde_yaml::Mapping::new())
    };
    
    if dry_run {
        println!("Would set '{}' to: {}", key, serde_yaml::to_string(&parsed_value)?);
        if let ConfigScope::Project = scope {
            println!("In: {}", path.display());
        } else {
            println!("In: {}", path.display());
        }
        return Ok(());
    }
    
    // Set the value
    set_nested_value(&mut config, key, parsed_value);
    
    // Save config
    let content = serde_yaml::to_string(&config)?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(&path, content)?;
    
    println!("Set '{}' = {}", key, serde_yaml::to_string(&parsed_value)?);
    
    Ok(())
}

/// Unset a configuration value
pub fn unset_value(
    key: &str,
    global: bool,
    project: bool,
    dry_run: bool,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = if global {
        ConfigScope::Global
    } else if project {
        ConfigScope::Project
    } else {
        ConfigScope::Project
    };
    
    let path = scope.path()?;
    
    if !path.exists() {
        return Err(format!("Configuration file not found: {}", path.display()).into());
    }
    
    let content = std::fs::read_to_string(&path)?;
    let mut config: Value = serde_yaml::from_str(&content)?;
    
    // Check if key exists
    if get_nested_value(&config, key).is_none() {
        return Err(format!("Key '{}' not found in configuration", key).into());
    }
    
    if dry_run {
        println!("Would unset: {}", key);
        return Ok(());
    }
    
    // Confirm for sensitive keys
    if is_sensitive_key(key) && !yes {
        print!("Are you sure you want to remove '{}'? [y/N] ", key);
        std::io::stdout().flush()?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Unset cancelled.");
            return Ok(());
        }
    }
    
    // Remove the key
    unset_nested_value(&mut config, key);
    
    // Save config
    let content = serde_yaml::to_string(&config)?;
    std::fs::write(&path, content)?;
    
    println!("Unset: {}", key);
    
    Ok(())
}

/// Show full configuration
pub fn show_config(
    global: bool,
    project: bool,
    all: bool,
    json_output: bool,
    yaml_output: bool,
    show_paths: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scopes: Vec<ConfigScope> = if all {
        vec![ConfigScope::Global, ConfigScope::Project]
    } else {
        let mut scopes = Vec::new();
        if global {
            scopes.push(ConfigScope::Global);
        }
        if project || (!global && !all) {
            scopes.push(ConfigScope::Project);
        }
        scopes
    };
    
    if show_paths {
        for scope in &scopes {
            let path = scope.path()?;
            let name = match scope {
                ConfigScope::Global => "Global",
                ConfigScope::Project => "Project",
            };
            println!("{} config: {}", name, path.display());
        }
        return Ok(());
    }
    
    for scope in &scopes {
        let path = scope.path()?;
        let name = match scope {
            ConfigScope::Global => "Global Configuration",
            ConfigScope::Project => "Project Configuration",
        };
        
        if !path.exists() {
            println!("{} (not found)", name);
            continue;
        }
        
        let content = std::fs::read_to_string(&path)?;
        let config: Value = serde_yaml::from_str(&content)?;
        
        if json_output {
            if scopes.len() > 1 {
                println!("{{ \"scope\": \"{}\", \"config\": {} }}", name.to_lowercase(), serde_json::to_string(&config)?);
            } else {
                println!("{}", serde_json::to_string(&config)?);
            }
        } else if yaml_output {
            if scopes.len() > 1 {
                println!("# {}", name);
            }
            println!("{}", serde_yaml::to_string(&config)?);
        } else {
            if scopes.len() > 1 {
                println!("=== {} ===", name);
            }
            print_yaml_value(&config, 0);
            println!();
        }
    }
    
    Ok(())
}

/// Reset configuration to defaults
pub fn reset_config(
    global: bool,
    project: bool,
    yes: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scopes: Vec<ConfigScope> = {
        let mut scopes = Vec::new();
        if global {
            scopes.push(ConfigScope::Global);
        }
        if project {
            scopes.push(ConfigScope::Project);
        }
        if scopes.is_empty() {
            // Default to both if neither specified
            scopes.push(ConfigScope::Global);
            scopes.push(ConfigScope::Project);
        }
        scopes
    };
    
    for scope in &scopes {
        let path = scope.path()?;
        let name = match scope {
            ConfigScope::Global => "global",
            ConfigScope::Project => "project",
        };
        
        if dry_run {
            println!("Would reset {} configuration to defaults", name);
            continue;
        }
        
        // Confirm with user
        if !yes {
            print!("Are you sure you want to reset {} configuration to defaults? [y/N] ", name);
            std::io::stdout().flush()?;
            
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            
            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("Reset cancelled for {} configuration.", name);
                continue;
            }
        }
        
        // Create default config
        let default_config = match scope {
            ConfigScope::Global => {
                let config = BaseConfig::new();
                serde_yaml::to_value(config)?
            }
            ConfigScope::Project => {
                let mut config = SkillsConfig::default_init("unnamed");
                // Reset to minimal
                config.skills.clear();
                serde_yaml::to_value(config)?
            }
        };
        
        // Save
        let content = serde_yaml::to_string(&default_config)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
        
        println!("Reset {} configuration to defaults", name);
    }
    
    Ok(())
}

/// Validate configuration files
pub fn validate_config(
    global: bool,
    project: bool,
    all: bool,
    strict: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scopes: Vec<ConfigScope> = if all {
        vec![ConfigScope::Global, ConfigScope::Project]
    } else {
        let mut scopes = Vec::new();
        if global {
            scopes.push(ConfigScope::Global);
        }
        if project {
            scopes.push(ConfigScope::Project);
        }
        if scopes.is_empty() {
            scopes.push(ConfigScope::Project);
        }
        scopes
    };
    
    let mut valid = true;
    
    for scope in &scopes {
        let path = scope.path()?;
        let name = match scope {
            ConfigScope::Global => "Global",
            ConfigScope::Project => "Project",
        };
        
        if !path.exists() {
            println!("{} configuration: NOT FOUND ({})", name, path.display());
            valid = false;
            continue;
        }
        
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                println!("{} configuration: INVALID (cannot read: {})", name, e);
                valid = false;
                continue;
            }
        };
        
        let config: Value = match serde_yaml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                println!("{} configuration: INVALID (YAML error: {})", name, e);
                valid = false;
                continue;
            }
        };
        
        // Validate structure
        match validate_config_structure(&config, strict, scope) {
            Ok(_) => println!("{} configuration: VALID", name),
            Err(e) => {
                println!("{} configuration: INVALID ({})", name, e);
                valid = false;
            }
        }
    }
    
    if !valid {
        std::process::exit(1);
    }
    
    Ok(())
}

/// Helper functions

/// Get a nested value from a YAML Value using dot notation
fn get_nested_value<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;
    
    for part in parts {
        match current {
            Value::Mapping(map) => {
                current = map.get(Value::String(part.to_string()))?;
            }
            Value::Sequence(vec) => {
                if let Ok(index) = part.parse::<usize>() {
                    current = vec.get(index)?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    
    Some(current)
}

/// Set a nested value in a YAML Value using dot notation
fn set_nested_value(value: &mut Value, key: &str, new_value: Value) {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;
    
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - set the value
            match current {
                Value::Mapping(map) => {
                    map.insert(Value::String(part.to_string()), new_value);
                }
                Value::Sequence(vec) => {
                    if let Ok(index) = part.parse::<usize>() {
                        if index < vec.len() {
                            vec[index] = new_value;
                        } else {
                            vec.push(new_value);
                        }
                    }
                }
                _ => {
                    // Replace the entire value
                    *current = new_value;
                }
            }
        } else {
            // Intermediate part - navigate or create
            match current {
                Value::Mapping(map) => {
                    if !map.contains_key(&Value::String(part.to_string())) {
                        // Create nested mapping
                        map.insert(Value::String(part.to_string()), Value::Mapping(serde_yaml::Mapping::new()));
                    }
                    current = map.get_mut(&Value::String(part.to_string())).unwrap();
                }
                Value::Sequence(vec) => {
                    if let Ok(index) = part.parse::<usize>() {
                        if index >= vec.len() {
                            // Extend the vector
                            vec.resize(index + 1, Value::Null);
                        }
                        current = &mut vec[index];
                    } else {
                        // Can't navigate further
                        *current = new_value;
                        return;
                    }
                }
                _ => {
                    // Replace with a mapping
                    let mut new_map = serde_yaml::Mapping::new();
                    new_map.insert(Value::String(part.to_string()), Value::Mapping(serde_yaml::Mapping::new()));
                    *current = Value::Mapping(new_map);
                    current = current.as_mapping_mut().unwrap().get_mut(&Value::String(part.to_string())).unwrap();
                }
            }
        }
    }
}

/// Unset a nested value in a YAML Value using dot notation
fn unset_nested_value(value: &mut Value, key: &str) {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;
    let mut parents: Vec<(&str, *mut Value)> = Vec::new();
    
    for part in &parts {
        parents.push((*part, current as *mut Value));
        match current {
            Value::Mapping(map) => {
                current = map.get_mut(&Value::String(part.to_string())).unwrap();
            }
            Value::Sequence(vec) => {
                if let Ok(index) = part.parse::<usize>() {
                    current = &mut vec[index];
                } else {
                    return;
                }
            }
            _ => return,
        }
    }
    
    // Now remove from parent
    if let Some((last_part, parent_ptr)) = parents.pop() {
        unsafe {
            match &mut *parent_ptr {
                Value::Mapping(map) => {
                    map.remove(&Value::String(last_part.to_string()));
                }
                Value::Sequence(vec) => {
                    if let Ok(index) = last_part.parse::<usize>() {
                        if index < vec.len() {
                            vec.remove(index);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Print a YAML value with indentation
fn print_yaml_value(value: &Value, indent: usize) {
    let prefix = " ".repeat(indent);
    
    match value {
        Value::Null => println!("{}null", prefix),
        Value::Bool(b) => println!("{} {}", prefix, b),
        Value::Number(n) => println!("{} {}", prefix, n),
        Value::String(s) => println!("{} {}", prefix, s),
        Value::Sequence(vec) => {
            println!("{}[", prefix);
            for item in vec {
                print_yaml_value(item, indent + 2);
            }
            println!("{}]", prefix);
        }
        Value::Mapping(map) => {
            println!("{}{", prefix);
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort_by(|a, b| {
                let a_str = a.as_str().unwrap_or("");
                let b_str = b.as_str().unwrap_or("");
                a_str.cmp(b_str)
            });
            for key in keys {
                if let Some(k) = key.as_str() {
                    print!("{}  {}: ", prefix, k);
                }
                print_yaml_value(map.get(key).unwrap(), indent + 4);
            }
            println!("{}{", prefix);
        }
        _ => println!("{}<unknown>", prefix),
    }
}

/// Get default value for a key
fn get_default_value(key: &str) -> Value {
    match key {
        "default_registry" => Value::String("default".to_string()),
        "check_for_updates" => Value::Bool(true),
        _ => Value::Null,
    }
}

/// Check if a key is sensitive (requires confirmation)
fn is_sensitive_key(key: &str) -> bool {
    matches!(key, "default_registry" | "registries")
}

/// Validate configuration structure
fn validate_config_structure(
    config: &Value,
    strict: bool,
    scope: &ConfigScope,
) -> Result<(), Box<dyn std::error::Error>> {
    match scope {
        ConfigScope::Global => {
            // Validate BaseConfig structure
            if let Value::Mapping(map) = config {
                // Check for required fields
                if strict {
                    if !map.contains_key(&Value::String("default_registry".to_string())) {
                        return Err("Missing required field: default_registry".into());
                    }
                    if !map.contains_key(&Value::String("registries".to_string())) {
                        return Err("Missing required field: registries".into());
                    }
                }
                
                // Validate registries if present
                if let Some(Value::Mapping(registries)) = map.get(&Value::String("registries".to_string())) {
                    for (key, value) in registries {
                        if let Some(name) = key.as_str() {
                            if name.is_empty() {
                                return Err("Registry name cannot be empty".into());
                            }
                        }
                        if let Some(Value::String(url)) = value.as_str() {
                            if url.is_empty() {
                                return Err("Registry URL cannot be empty".into());
                            }
                        } else {
                            return Err("Registry URL must be a string".into());
                        }
                    }
                }
            } else {
                return Err("Global configuration must be a mapping".into());
            }
        }
        ConfigScope::Project => {
            // Validate SkillsConfig structure
            if let Value::Mapping(map) = config {
                if strict {
                    if !map.contains_key(&Value::String("name".to_string())) {
                        return Err("Missing required field: name".into());
                    }
                    if !map.contains_key(&Value::String("agents".to_string())) {
                        return Err("Missing required field: agents".into());
                    }
                }
                
                // Validate agents if present
                if let Some(Value::Sequence(agents)) = map.get(&Value::String("agents".to_string())) {
                    for agent in agents {
                        if let Some(Value::String(name)) = agent.as_str() {
                            if !crate::linker::is_supported_agent(name) {
                                return Err(format!("Unsupported agent: {}", name).into());
                            }
                        }
                    }
                }
            } else {
                return Err("Project configuration must be a mapping".into());
            }
        }
    }
    
    Ok(())
}
```

## Behavior Flow

### Get Value
```
1. User runs: skm config get name
2. SKM determines scope (project by default)
3. SKM loads configuration file
4. SKM parses key using dot notation
5. SKM retrieves value
6. SKM formats output (text or JSON)
```

### Set Value
```
1. User runs: skm config set name my-project
2. SKM determines scope (project by default)
3. SKM parses value (as JSON if --json flag)
4. If --dry-run: Show what would be set
5. SKM loads existing configuration
6. SKM sets the nested value
7. SKM saves configuration
8. SKM prints confirmation
```

### Unset Value
```
1. User runs: skm config unset version
2. SKM determines scope (project by default)
3. SKM loads configuration
4. If --dry-run: Show what would be unset
5. For sensitive keys: Prompt for confirmation
6. SKM removes the nested value
7. SKM saves configuration
8. SKM prints confirmation
```

### Show Configuration
```
1. User runs: skm config show
2. SKM determines which scopes to show
3. SKM loads each configuration file
4. SKM formats output (text, YAML, or JSON)
5. SKM prints configuration
```

### Reset Configuration
```
1. User runs: skm config reset
2. SKM determines which scopes to reset
3. If --dry-run: Show what would be reset
4. For each scope: Prompt for confirmation
5. SKM creates default configuration
6. SKM saves to configuration file
7. SKM prints confirmation
```

### Validate Configuration
```
1. User runs: skm config validate
2. SKM determines which scopes to validate
3. SKM loads each configuration file
4. SKM validates structure and values
5. SKM prints validation results
6. If any invalid: Exit with error code 1
```

## Error Handling

| Scenario | Behavior |
|----------|----------|
| File not found | Clear error with path and scope |
| Key not found | Error unless --default is used |
| Invalid YAML | Clear error with line/column info |
| Invalid key format | Clear error with expected format |
| Invalid value | Clear error with expected type |
| Permission denied | Clear error with path |
| Sensitive key without confirmation | Prompt for confirmation |

## Testing Requirements

### Get Value
1. Test getting existing keys from project config
2. Test getting existing keys from global config
3. Test getting non-existent keys with --default
4. Test getting non-existent keys without --default (should error)
5. Test JSON output
6. Test nested keys (dot notation)

### Set Value
1. Test setting simple string values
2. Test setting with --json for complex values
3. Test setting nested keys
4. Test setting in global vs project scope
5. Test dry run mode
6. Test creating new configuration file

### Unset Value
1. Test unsetting existing keys
2. Test unsetting non-existent keys (should error)
3. Test unsetting nested keys
4. Test dry run mode
5. Test sensitive key confirmation

### Show Configuration
1. Test showing project configuration
2. Test showing global configuration
3. Test showing both with --all
4. Test JSON output
5. Test YAML output
6. Test --paths flag
7. Test non-existent configuration files

### Reset Configuration
1. Test resetting project configuration
2. Test resetting global configuration
3. Test dry run mode
4. Test confirmation prompts

### Validate Configuration
1. Test validating valid configuration
2. Test validating invalid configuration
3. Test strict mode
4. Test multiple scopes

## Migration Path

This feature is fully backward compatible:
- Existing configurations are unaffected
- No changes to existing commands
- New commands are additive
- Configuration file formats remain unchanged

## User Experience

### Get Value Output
```
$ skm config get name
my-project

$ skm config get default_registry --global
default

$ skm config get registries --json
{"default":"git@github.com:skills-yaml/skills-registry.git"}
```

### Set Value Output
```
$ skm config set name new-name
Set 'name' = new-name

$ skm config set version v2.0.0
Set 'version' = v2.0.0

$ skm config set registries.company "git@github.com:my-company/skills.git"
Set 'registries.company' = git@github.com:my-company/skills.git
```

### Show Configuration Output
```
$ skm config show

name: my-project
version: v0.1.0
agents:
  - claude
  - cursor
skills:
  - name: software-development/symphony-spec-writing
    version: latest
    source: default
```

### JSON Output
```json
{
  "name": "my-project",
  "version": "v0.1.0",
  "agents": ["claude", "cursor"],
  "skills": [
    {
      "name": "software-development/symphony-spec-writing",
      "version": "latest",
      "source": "default"
    }
  ]
}
```

### Reset Output
```
$ skm config reset --project

Are you sure you want to reset project configuration to defaults? [y/N] y
Reset project configuration to defaults
```

### Validate Output
```
$ skm config validate --all

Global configuration: VALID
Project configuration: VALID

$ skm config validate

Project configuration: VALID
```

## Related Features

- BaseConfig - Global configuration structure
- SkillsConfig - Project configuration structure
- `skm init` - Creates project configuration
- `skm setup` - Creates global configuration
- Environment variables - Alternative to config files
