# Specification: Cleanup Commands Feature

## Overview

SKM needs cleanup commands to help users remove orphaned symlinks, clear caches, and reset SKM state. This is essential for maintenance, troubleshooting, and freeing up disk space.

## Problem Statement

Currently, there is no built-in way to:
1. Remove broken or orphaned symlinks from agent directories
2. Clear the registry cache to free up disk space
3. Reset SKM to a clean state
4. Clean up old versions of skills

Users must manually:
- Find and remove broken symlinks
- Delete cache directories
- Reinitialize configuration

This is error-prone and time-consuming.

## Requirements

### Symlink Cleanup

#### R1: Remove Broken Symlinks
SKM must identify and remove symlinks that point to non-existent targets.

#### R2: Remove Orphaned Symlinks
SKM must identify and remove symlinks for skills that are no longer in the configuration.

#### R3: Verify Before Removal
SKM must verify with the user before removing any symlinks.

#### R4: Dry Run Mode
SKM must support previewing what would be removed without making changes.

### Cache Cleanup

#### R5: Clear Registry Cache
SKM must allow clearing the cache for specific registries or all registries.

#### R6: Remove Old Versions
SKM must identify and remove old skill versions that are no longer referenced.

#### R7: Cache Statistics
SKM must display cache size and usage statistics.

### Full Cleanup

#### R8: Reset to Clean State
SKM must provide a command to remove all SKM files and start fresh.

#### R9: Selective Cleanup
SKM must allow cleaning up specific components (config, cache, symlinks).

### Safety

#### R10: Confirmation Required
SKM must require explicit confirmation for destructive cleanup operations.

#### R11: Backup Option
SKM must provide an option to backup before cleanup.

#### R12: Non-Destructive by Default
SKM must be non-destructive by default (dry run or confirmation).

## Command Specifications

### 1. Clean Symlinks

```
Command: skm clean [OPTIONS]

Description: Clean up broken and orphaned symlinks

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Clean global symlinks | Flag | false |
| `--broken` | Only clean broken symlinks | Flag | false |
| `--orphaned` | Only clean orphaned symlinks (not in config) | Flag | false |
| `--all` | Clean all symlinks (broken + orphaned) | Flag | false |
| `--dry-run` | Preview what would be removed | Flag | false |
| `--yes`, `-y` | Skip confirmation | Flag | false |
| `--verbose`, `-v` | Show detailed information | Flag | false |

Examples:
```bash
# Clean broken symlinks in current project
$ skm clean --broken

# Clean orphaned symlinks globally
$ skm clean --global --orphaned

# Clean all symlinks (preview first)
$ skm clean --all --dry-run

# Actually clean all symlinks
$ skm clean --all --yes
```
```

### 2. Clean Cache

```
Command: skm clean cache [OPTIONS] [REGISTRY]

Description: Clean up registry cache

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--all` | Clean all registry caches | Flag | false |
| `--old-versions` | Remove old skill versions | Flag | false |
| `--keep`, `-k` | Keep N most recent versions (default: 5) | Number | 5 |
| `--dry-run` | Preview what would be removed | Flag | false |
| `--yes`, `-y` | Skip confirmation | Flag | false |
| `--stats` | Show cache statistics only | Flag | false |
| `--verbose`, `-v` | Show detailed information | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `REGISTRY` | Specific registry to clean | No | String |

Examples:
```bash
# Show cache statistics
$ skm clean cache --stats

# Clean all caches
$ skm clean cache --all --yes

# Clean specific registry
$ skm clean cache company

# Remove old versions, keep last 3
$ skm clean cache --old-versions --keep 3
```
```

### 3. Full Reset

```
Command: skm clean reset [OPTIONS]

Description: Reset SKM to clean state

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--config` | Reset configuration files | Flag | false |
| `--cache` | Clear all caches | Flag | false |
| `--symlinks` | Remove all symlinks | Flag | false |
| `--all` | Reset everything (config + cache + symlinks) | Flag | false |
| `--backup` | Create backup before reset | Flag | false |
| `--backup-dir` | Directory to store backups | String | ~/.skm-backups |
| `--dry-run` | Preview what would be removed | Flag | false |
| `--yes`, `-y` | Skip confirmation | Flag | false |

Examples:
```bash
# Preview full reset
$ skm clean reset --all --dry-run

# Reset with backup
$ skm clean reset --all --backup --yes

# Only reset configuration
$ skm clean reset --config --yes

# Only clear cache
$ skm clean reset --cache --yes
```
```

## Implementation

### Files to Modify

#### `src/main.rs`
Add clean commands to the `Commands` enum:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Clean up SKM artifacts (broken symlinks, cache, etc.)
    #[command(subcommand)]
    Clean(CleanCommands),
}

#[derive(Subcommand)]
enum CleanCommands {
    /// Clean up broken and orphaned symlinks
    Symlinks {
        /// Clean global symlinks
        #[arg(short, long)]
        global: bool,
        /// Only clean broken symlinks
        #[arg(long)]
        broken: bool,
        /// Only clean orphaned symlinks
        #[arg(long)]
        orphaned: bool,
        /// Clean all symlinks (broken + orphaned)
        #[arg(long)]
        all: bool,
        /// Preview what would be removed
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Clean up registry cache
    Cache {
        /// Clean all registry caches
        #[arg(long)]
        all: bool,
        /// Remove old skill versions
        #[arg(long)]
        old_versions: bool,
        /// Keep N most recent versions
        #[arg(short, long, default_value = "5")]
        keep: usize,
        /// Preview what would be removed
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Show cache statistics
        #[arg(long)]
        stats: bool,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
        /// Specific registry to clean
        registry: Option<String>,
    },
    
    /// Reset SKM to clean state
    Reset {
        /// Reset configuration files
        #[arg(long)]
        config: bool,
        /// Clear all caches
        #[arg(long)]
        cache: bool,
        /// Remove all symlinks
        #[arg(long)]
        symlinks: bool,
        /// Reset everything
        #[arg(long)]
        all: bool,
        /// Create backup before reset
        #[arg(long)]
        backup: bool,
        /// Directory to store backups
        #[arg(long)]
        backup_dir: Option<String>,
        /// Preview what would be removed
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
}
```

Add handler in `run()` function:

```rust
Commands::Clean(cmd) => {
    match cmd {
        CleanCommands::Symlinks { global, broken, orphaned, all, dry_run, yes, verbose } => {
            cleaner::clean_symlinks(global, broken, orphaned, all, dry_run, yes, verbose)?;
        }
        CleanCommands::Cache { all, old_versions, keep, dry_run, yes, stats, verbose, registry } => {
            cleaner::clean_cache(all, old_versions, keep, dry_run, yes, stats, verbose, registry)?;
        }
        CleanCommands::Reset { config, cache, symlinks, all, backup, backup_dir, dry_run, yes } => {
            cleaner::reset(config, cache, symlinks, all, backup, backup_dir, dry_run, yes)?;
        }
    }
    Ok(())
}
```

#### `src/cleaner.rs` (New File)
Create a dedicated cleanup module:

```rust
use crate::config::{SkillSpec, SkillsConfig};
use crate::config_manager::{BaseConfig, get_base_config_path, get_cache_dir};
use crate::linker;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Symlink status
#[derive(Debug, Clone)]
pub enum SymlinkStatus {
    Valid,
    Broken,
    Orphaned,
    NotSymlink,
}

/// Symlink information
#[derive(Debug, Clone)]
pub struct SymlinkInfo {
    pub path: PathBuf,
    pub target: PathBuf,
    pub agent: String,
    pub skill_name: String,
    pub status: SymlinkStatus,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub registry: String,
    pub path: PathBuf,
    pub size: u64,
    pub skill_count: usize,
    pub version_count: usize,
    pub last_updated: Option<String>,
}

/// Clean up broken and orphaned symlinks
pub fn clean_symlinks(
    global: bool,
    broken: bool,
    orphaned: bool,
    all: bool,
    dry_run: bool,
    yes: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    
    // Determine what to clean
    let clean_broken = broken || all;
    let clean_orphaned = orphaned || all;
    
    if !clean_broken && !clean_orphaned {
        return Err("Must specify at least one of --broken, --orphaned, or --all".into());
    }
    
    // Get project configuration if not global
    let config_skills: Vec<String> = if global {
        Vec::new() // For global, we check all configured skills
    } else {
        let config_path = current_dir.join("skills.yaml");
        if config_path.exists() {
            let config = SkillsConfig::load_from_file(&config_path)?;
            config.skills.iter().map(|s| s.name.clone()).collect()
        } else {
            Vec::new()
        }
    };
    
    // Get base config for global skills
    let base_config = BaseConfig::load().ok();
    let all_skills: Vec<String> = if global {
        base_config.as_ref().map_or(Vec::new(), |c| {
            c.registries.keys().cloned().collect()
        })
    } else {
        config_skills
    };
    
    // Find all symlinks
    let agents = if global {
        // All known agents
        vec!["claude", "cursor", "codex", "copilot", "grok", "hermes"]
            .iter().map(|s| s.to_string()).collect()
    } else {
        // Agents from project config
        let config_path = current_dir.join("skills.yaml");
        if config_path.exists() {
            let config = SkillsConfig::load_from_file(&config_path)?;
            config.agents
        } else {
            Vec::new()
        }
    };
    
    let mut symlinks_to_clean: Vec<SymlinkInfo> = Vec::new();
    
    for agent in &agents {
        let base_dir = if global {
            linker::get_global_agent_skills_dir(agent)
        } else {
            linker::get_project_agent_skills_dir(agent, &current_dir)
        };
        
        let Some(base_dir) = base_dir else {
            continue;
        };
        
        if !base_dir.exists() {
            if verbose {
                println!("Agent directory does not exist: {}", base_dir.display());
            }
            continue;
        }
        
        // Find all symlinks in this agent directory
        for entry in fs::read_dir(&base_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_symlink() {
                let target = fs::read_link(&path)?;
                let skill_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                
                let status = if !target.exists() {
                    SymlinkStatus::Broken
                } else if clean_orphaned && !all_skills.contains(&skill_name) {
                    SymlinkStatus::Orphaned
                } else if clean_broken && !target.exists() {
                    SymlinkStatus::Broken
                } else {
                    SymlinkStatus::Valid
                };
                
                if clean_broken && matches!(status, SymlinkStatus::Broken) {
                    symlinks_to_clean.push(SymlinkInfo {
                        path: path.clone(),
                        target,
                        agent: agent.clone(),
                        skill_name,
                        status,
                    });
                } else if clean_orphaned && matches!(status, SymlinkStatus::Orphaned) {
                    symlinks_to_clean.push(SymlinkInfo {
                        path: path.clone(),
                        target,
                        agent: agent.clone(),
                        skill_name,
                        status,
                    });
                }
            }
        }
    }
    
    // Preview or execute
    if dry_run || symlinks_to_clean.is_empty() {
        if symlinks_to_clean.is_empty() {
            println!("No symlinks to clean");
        } else {
            println!("Would clean {} symlinks:", symlinks_to_clean.len());
            for info in &symlinks_to_clean {
                let status = match info.status {
                    SymlinkStatus::Broken => "broken",
                    SymlinkStatus::Orphaned => "orphaned",
                    _ => "unknown",
                };
                println!("  [{:?}] {} -> {} (agent: {}, skill: {})", 
                    status, info.path.display(), info.target.display(), info.agent, info.skill_name);
            }
        }
        return Ok(());
    }
    
    // Confirm with user
    if !yes {
        println!("Found {} symlinks to clean:", symlinks_to_clean.len());
        for info in &symlinks_to_clean {
            let status = match info.status {
                SymlinkStatus::Broken => "broken",
                SymlinkStatus::Orphaned => "orphaned",
                _ => "unknown",
            };
            println!("  [{}] {} -> {}", status, info.path.display(), info.target.display());
        }
        print!("\nClean these symlinks? [y/N] ");
        std::io::stdout().flush()?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Cleanup cancelled.");
            return Ok(());
        }
    }
    
    // Clean the symlinks
    let mut cleaned = 0;
    for info in &symlinks_to_clean {
        if verbose {
            println!("Removing: {}", info.path.display());
        }
        fs::remove_file(&info.path)?;
        cleaned += 1;
    }
    
    println!("Cleaned {} symlinks", cleaned);
    
    Ok(())
}

/// Clean up registry cache
pub fn clean_cache(
    all: bool,
    old_versions: bool,
    keep: usize,
    dry_run: bool,
    yes: bool,
    stats: bool,
    verbose: bool,
    registry: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if stats {
        return show_cache_stats(registry);
    }
    
    let base_config = BaseConfig::load()?;
    
    let registries_to_clean: Vec<String> = if all {
        base_config.registries.keys().cloned().collect()
    } else if let Some(ref name) = registry {
        if !base_config.registries.contains_key(name) {
            return Err(format!("Registry '{}' not found", name).into());
        }
        vec![name.clone()]
    } else {
        return Err("Must specify a registry or use --all".into());
    };
    
    if old_versions {
        return clean_old_versions(&registries_to_clean, keep, dry_run, yes, verbose);
    }
    
    // Full cache cleanup
    let mut total_size = 0;
    let mut total_removed = 0;
    
    for reg_name in &registries_to_clean {
        let cache_path = linker::resolve_registry_path(reg_name)
            .ok_or_else(|| format!("Could not resolve path for registry: {}", reg_name))?;
        
        if !cache_path.exists() {
            if verbose {
                println!("Cache for '{}' does not exist: {}", reg_name, cache_path.display());
            }
            continue;
        }
        
        // Calculate size
        let size = calculate_directory_size(&cache_path)?;
        total_size += size;
        
        if dry_run {
            println!("Would remove cache for '{}': {} ({} bytes)", reg_name, cache_path.display(), size);
            total_removed += 1;
        } else {
            if verbose {
                println!("Removing cache for '{}': {}", reg_name, cache_path.display());
            }
            fs::remove_dir_all(&cache_path)?;
            total_removed += 1;
        }
    }
    
    if dry_run {
        println!("\nWould clean {} registry caches ({} bytes total)", total_removed, total_size);
    } else {
        println!("Cleaned {} registry caches ({} bytes freed)", total_removed, total_size);
    }
    
    Ok(())
}

/// Clean old skill versions
pub fn clean_old_versions(
    registries: &[String],
    keep: usize,
    dry_run: bool,
    yes: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut total_freed = 0;
    let mut versions_removed = 0;
    
    for reg_name in registries {
        let cache_path = linker::resolve_registry_path(reg_name)
            .ok_or_else(|| format!("Could not resolve path for registry: {}", reg_name))?;
        
        if !cache_path.exists() {
            if verbose {
                println!("Cache for '{}' does not exist", reg_name);
            }
            continue;
        }
        
        let skills_dir = cache_path.join("skills");
        if !skills_dir.exists() {
            continue;
        }
        
        // For each skill, clean old versions
        for skill_entry in fs::read_dir(&skills_dir)? {
            let skill_entry = skill_entry?;
            let skill_path = skill_entry.path();
            
            if !skill_path.is_dir() {
                continue;
            }
            
            // Get all version directories
            let mut versions: Vec<PathBuf> = Vec::new();
            for version_entry in fs::read_dir(&skill_path)? {
                let version_entry = version_entry?;
                let version_path = version_entry.path();
                
                if version_path.is_dir() {
                    versions.push(version_path);
                }
            }
            
            // Sort by modification time (oldest first)
            versions.sort_by(|a, b| {
                let a_time = a.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::now());
                let b_time = b.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::now());
                a_time.cmp(&b_time)
            });
            
            // Keep the newest 'keep' versions
            let to_remove = if versions.len() > keep {
                &versions[..versions.len() - keep]
            } else {
                &[][..]
            };
            
            for version_path in to_remove {
                let size = calculate_directory_size(version_path)?;
                
                if dry_run {
                    println!("Would remove old version: {} ({} bytes)", version_path.display(), size);
                    total_freed += size;
                    versions_removed += 1;
                } else {
                    if verbose {
                        println!("Removing old version: {}", version_path.display());
                    }
                    fs::remove_dir_all(version_path)?;
                    total_freed += size;
                    versions_removed += 1;
                }
            }
        }
    }
    
    if dry_run {
        println!("\nWould remove {} old versions ({} bytes total)", versions_removed, total_freed);
    } else if !yes {
        println!("Removed {} old versions ({} bytes freed)", versions_removed, total_freed);
    }
    
    Ok(())
}

/// Reset SKM to clean state
pub fn reset(
    config: bool,
    cache: bool,
    symlinks: bool,
    all: bool,
    backup: bool,
    backup_dir: Option<String>,
    dry_run: bool,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    
    // Determine what to reset
    let reset_config = config || all;
    let reset_cache = cache || all;
    let reset_symlinks = symlinks || all;
    
    if !reset_config && !reset_cache && !reset_symlinks {
        return Err("Must specify at least one of --config, --cache, --symlinks, or --all".into());
    }
    
    // Determine backup directory
    let backup_dir = if backup {
        let dir = backup_dir.unwrap_or_else(|| {
            dirs::home_dir().map(|d| d.join(".skm-backups")).unwrap_or_else(|| PathBuf::from(".skm-backups"))
        });
        fs::create_dir_all(&dir)?;
        Some(dir)
    } else {
        None
    };
    
    // Collect items to reset
    let mut items: Vec<(String, PathBuf, String)> = Vec::new();
    
    if reset_config {
        // Global config
        if let Some(path) = get_base_config_path() {
            if path.exists() {
                items.push(("Global configuration".to_string(), path, "config".to_string()));
            }
        }
        
        // Project config
        let project_config = current_dir.join("skills.yaml");
        if project_config.exists() {
            items.push(("Project configuration".to_string(), project_config, "config".to_string()));
        }
    }
    
    if reset_cache {
        // Cache directory
        if let Some(cache_dir) = get_cache_dir() {
            if cache_dir.exists() {
                items.push(("Cache directory".to_string(), cache_dir, "cache".to_string()));
            }
        }
    }
    
    if reset_symlinks {
        // Global symlinks for all agents
        let agents = ["claude", "cursor", "codex", "copilot", "grok", "hermes"];
        for agent in &agents {
            if let Some(dir) = linker::get_global_agent_skills_dir(agent) {
                if dir.exists() {
                    items.push((format!("Global {} symlinks", agent), dir, "symlinks".to_string()));
                }
            }
        }
        
        // Project symlinks for configured agents
        let project_config = current_dir.join("skills.yaml");
        if project_config.exists() {
            let config = SkillsConfig::load_from_file(&project_config).ok();
            if let Some(config) = config {
                for agent in &config.agents {
                    if let Some(dir) = linker::get_project_agent_skills_dir(agent, &current_dir) {
                        if dir.exists() {
                            items.push((format!("Project {} symlinks", agent), dir, "symlinks".to_string()));
                        }
                    }
                }
            }
        }
    }
    
    if items.is_empty() {
        println!("No items to reset");
        return Ok(());
    }
    
    // Preview or execute
    if dry_run {
        println!("Would reset the following items:");
        for (name, path, _) in &items {
            println!("  - {}: {}", name, path.display());
        }
        if let Some(ref dir) = backup_dir {
            println!("\nBackups would be created in: {}", dir.display());
        }
        return Ok(());
    }
    
    // Confirm with user
    if !yes {
        println!("About to reset the following items:");
        for (name, path, _) in &items {
            println!("  - {}: {}", name, path.display());
        }
        if let Some(ref dir) = backup_dir {
            println!("\nBackups will be created in: {}", dir.display());
        }
        print!("\nReset these items? [y/N] ");
        std::io::stdout().flush()?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Reset cancelled.");
            return Ok(());
        }
    }
    
    // Perform reset with optional backup
    let mut reset_count = 0;
    for (name, path, item_type) in &items {
        if let Some(ref backup_dir) = backup_dir {
            // Create backup
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let backup_path = backup_dir.join(format!("{}_{}.backup", path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown"), timestamp));
            
            if path.is_dir() {
                copy_dir_all(path, &backup_path)?;
            } else {
                fs::copy(path, &backup_path)?;
            }
            println!("Backed up {} to {}", name, backup_path.display());
        }
        
        // Remove item
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        println!("Reset: {}", name);
        reset_count += 1;
    }
    
    println!("\nReset {} items", reset_count);
    
    if reset_config {
        println!("\nTo reinitialize, run:");
        println!("  skm setup");
    }
    
    Ok(())
}

/// Show cache statistics
pub fn show_cache_stats(registry: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let base_config = BaseConfig::load()?;
    
    let registries: Vec<String> = if let Some(ref name) = registry {
        if !base_config.registries.contains_key(name) {
            return Err(format!("Registry '{}' not found", name).into());
        }
        vec![name.clone()]
    } else {
        base_config.registries.keys().cloned().collect()
    };
    
    let mut total_size = 0;
    let mut total_skills = 0;
    let mut total_versions = 0;
    
    println!("Cache Statistics:");
    println!("{}", "=".repeat(50));
    
    for reg_name in &registries {
        let cache_path = linker::resolve_registry_path(reg_name)
            .ok_or_else(|| format!("Could not resolve path for registry: {}", reg_name))?;
        
        let (size, skill_count, version_count) = if cache_path.exists() {
            let size = calculate_directory_size(&cache_path)?;
            let skills_dir = cache_path.join("skills");
            let skill_count = if skills_dir.exists() {
                fs::read_dir(&skills_dir)?.count()
            } else {
                0
            };
            
            let mut version_count = 0;
            if skills_dir.exists() {
                for skill_entry in fs::read_dir(&skills_dir)? {
                    let skill_entry = skill_entry?;
                    let skill_path = skill_entry.path();
                    if skill_path.is_dir() {
                        version_count += fs::read_dir(&skill_path)?.count();
                    }
                }
            }
            
            (size, skill_count, version_count)
        } else {
            (0, 0, 0)
        };
        
        total_size += size;
        total_skills += skill_count;
        total_versions += version_count;
        
        println!("\nRegistry: {}", reg_name);
        println!("  Path: {}", cache_path.display());
        println!("  Size: {} ({} MB)", format_size(size), format_size_mb(size));
        println!("  Cached: {}", if cache_path.exists() { "Yes" } else { "No" });
        if cache_path.exists() {
            println!("  Skills: {}", skill_count);
            println!("  Versions: {}", version_count);
        }
    }
    
    println!("\n{}", "-".repeat(50));
    println!("Total: {} registries, {} MB, {} skills, {} versions", 
        registries.len(), format_size_mb(total_size), total_skills, total_versions);
    
    Ok(())
}

/// Helper functions

/// Calculate total size of a directory
fn calculate_directory_size(path: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    let mut size = 0;
    
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        
        if metadata.is_dir() {
            size += calculate_directory_size(&entry.path())?;
        } else {
            size += metadata.len();
        }
    }
    
    Ok(size)
}

/// Format size in human-readable format
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Format size in MB
fn format_size_mb(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    format!("{:.2}", bytes as f64 / MB as f64)
}

/// Copy a directory recursively
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !src.exists() {
        return Ok(());
    }
    
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            copy_dir_all(&src_path, &dst_path)?;
        }
    } else {
        fs::copy(src, dst)?;
    }
    
    Ok(())
}
```

## Behavior Flow

### Clean Symlinks
```
1. User runs: skm clean --broken
2. SKM determines scope (project by default)
3. SKM finds all symlinks in agent directories
4. SKM identifies broken symlinks (target doesn't exist)
5. If --dry-run: Show what would be removed
6. If --yes: Skip confirmation
7. If not --yes: Prompt user for confirmation
8. SKM removes the broken symlinks
9. SKM prints summary
```

### Clean Cache
```
1. User runs: skm clean cache --all
2. SKM identifies all registry caches
3. If --dry-run: Show what would be removed
4. If --yes: Skip confirmation
5. If not --yes: Prompt user for confirmation
6. SKM removes cache directories
7. SKM prints summary with size freed
```

### Clean Old Versions
```
1. User runs: skm clean cache --old-versions --keep 3
2. SKM identifies all skill versions in caches
3. For each skill: Keep newest 3, mark rest for removal
4. If --dry-run: Show what would be removed
5. If --yes: Skip confirmation
6. If not --yes: Prompt user for confirmation
7. SKM removes old version directories
8. SKM prints summary with space freed
```

### Reset
```
1. User runs: skm clean reset --all --backup
2. SKM identifies all SKM files (config, cache, symlinks)
3. If --dry-run: Show what would be removed
4. If --yes: Skip confirmation
5. If not --yes: Prompt user for confirmation
6. If --backup: Create backups of all files
7. SKM removes all identified files
8. SKM prints summary
```

### Show Cache Stats
```
1. User runs: skm clean cache --stats
2. SKM calculates size for each registry cache
3. SKM counts skills and versions
4. SKM prints formatted statistics
```

## Error Handling

| Scenario | Behavior |
|----------|----------|
| No items to clean | Print message, exit successfully |
| File not found | Skip with warning |
| Permission denied | Clear error with path |
| Directory in use | Clear error with suggestion to close applications |
| Backup fails | Warning, continue with reset |
| No scope specified | Clear error with instructions |

## Testing Requirements

### Clean Symlinks
1. Test cleaning broken symlinks in project
2. Test cleaning broken symlinks globally
3. Test cleaning orphaned symlinks
4. Test --dry-run mode
5. Test --yes flag
6. Test --all flag
7. Test with no broken symlinks

### Clean Cache
1. Test cleaning all caches
2. Test cleaning specific registry
3. Test --dry-run mode
4. Test --stats flag
5. Test with non-existent cache

### Clean Old Versions
1. Test removing old versions with --keep flag
2. Test --dry-run mode
3. Test with skills that have fewer versions than keep

### Reset
1. Test resetting config only
2. Test resetting cache only
3. Test resetting symlinks only
4. Test resetting all
5. Test with --backup flag
6. Test --dry-run mode
7. Test confirmation prompts

## Migration Path

This feature is fully backward compatible:
- Existing configurations are unaffected
- No changes to existing commands
- New commands are additive
- Cleanup operations are destructive but require confirmation

## User Experience

### Clean Symlinks Output
```
$ skm clean --broken

Found 3 broken symlinks:
  [broken] ~/.claude/skills/old-skill -> /path/to/missing/target
  [broken] ~/.cursor/skills/old-skill -> /path/to/missing/target
  [broken] ~/.codex/skills/old-skill -> /path/to/missing/target

Clean these symlinks? [y/N] y
Cleaned 3 symlinks
```

### Clean Cache Output
```
$ skm clean cache --all

Would remove cache for 'default': ~/.cache/skm/registries/default (123.45 MB)
Would remove cache for 'company': ~/.cache/skm/registries/company (45.67 MB)

Clean these caches? [y/N] y
Cleaned 2 registry caches (169.12 MB freed)
```

### Clean Old Versions Output
```
$ skm clean cache --old-versions --keep 3

Would remove old versions:
  ~/.cache/skm/registries/default/skills/my-skill/v1.0.0 (2.34 MB)
  ~/.cache/skm/registries/default/skills/my-skill/v1.1.0 (2.45 MB)
  ~/.cache/skm/registries/default/skills/other-skill/v1.0.0 (1.23 MB)

Remove these old versions? [y/N] y
Removed 3 old versions (6.02 MB freed)
```

### Reset Output
```
$ skm clean reset --all --backup

About to reset the following items:
  - Global configuration: ~/.config/skm/config.yaml
  - Cache directory: ~/.cache/skm
  - Global claude symlinks: ~/.claude/skills
  - Global cursor symlinks: ~/.cursor/skills

Backups will be created in: ~/.skm-backups

Reset these items? [y/N] y
Backed up Global configuration to ~/.skm-backups/config.yaml_1234567890.backup
Backed up Cache directory to ~/.skm-backups/cache_1234567890.backup
Reset: Global configuration
Reset: Cache directory
Reset: Global claude symlinks
Reset: Global cursor symlinks

Reset 4 items

To reinitialize, run:
  skm setup
```

### Cache Stats Output
```
$ skm clean cache --stats

Cache Statistics:
==================================================

Registry: default
  Path: /home/user/.cache/skm/registries/default
  Size: 123.45 MB (123.45 MB)
  Cached: Yes
  Skills: 42
  Versions: 87

Registry: company
  Path: /home/user/.cache/skm/registries/company
  Size: 45.67 MB (45.67 MB)
  Cached: Yes
  Skills: 15
  Versions: 23

--------------------------------------------------
Total: 2 registries, 169.12 MB, 57 skills, 110 versions
```

## Related Features

- `skm list` - Lists skills (shows what's configured)
- `skm check` - Verifies skills (identifies broken symlinks)
- `skm cache-update` - Updates caches
- BaseConfig/SkillsConfig - Configuration structures
