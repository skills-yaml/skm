# Specification: Local Development Mode

## Overview

SKM needs a development mode that allows skill developers to work on skills locally and test them without needing to push to a registry. This enables rapid iteration and testing of skills.

## Problem Statement

Currently, to test a skill:
1. Developer must push changes to a registry
2. User must run `skm cache-update` to pull changes
3. User must run `skm install` to relink
4. This workflow is slow and cumbersome for development

Additionally:
- No way to test a skill before it's in a registry
- No way to make local modifications to a cached skill
- No way to override a registry skill with a local version

## Requirements

### Core Development Workflow

#### R1: Link Local Directory
SKM must allow linking a local directory as a skill, making it available to agents immediately.

#### R2: Development Symlinks
SKM must create symlinks that point directly to the local development directory (not copies).

#### R3: Hot Reloading
Changes to files in the development directory must be immediately visible to agents (via symlinks).

#### R4: Multiple Local Skills
SKM must support multiple local skills simultaneously.

### Development Lifecycle

#### R5: List Development Skills
SKM must display a list of currently linked development skills.

#### R6: Unlink Development Skill
SKM must allow unlinking a development skill, restoring the original registry version.

#### R7: Override Registry Skill
SKM must allow a local development skill to override a registry skill with the same name.

### Project & Global Scope

#### R8: Project-Level Development
SKM must support development skills in project scope.

#### R9: Global Development
SKM must support development skills in global scope.

#### R10: Mixed Development & Registry Skills
SKM must allow mixing development and registry skills in the same project.

### Safety & UX

#### R11: Clear Identification
SKM must clearly identify development skills in listings.

#### R12: Validation
SKM must validate that development directories contain valid skills (have SKILL.md).

#### R13: Confirmation for Overrides
SKM must warn when a development skill overrides an existing registry skill.

#### R14: Non-Destructive
Linking/unlinking development skills must not affect registry cache.

## Command Specifications

### 1. Link Local Skill

```
Command: skm dev link <PATH> [OPTIONS]

Description: Link a local directory as a development skill

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--name`, `-n` | Skill name (defaults to directory name) | String | Directory name |
| `--source`, `-s` | Registry source to override | String | None |
| `--global`, `-g` | Link globally instead of in current project | Flag | false |
| `--all-agents` | Link to all available agents | Flag | false |
| `--agent` | Link to specific agent(s) (comma-separated) | String | None |
| `--force`, `-f` | Override existing skill without warning | Flag | false |
| `--verbose`, `-v` | Show detailed output | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `PATH` | Path to local skill directory | Yes | Path |

Examples:
```bash
# Link current directory as a development skill
$ skm dev link .

# Link with custom name
$ skm dev link ~/projects/my-skill --name my-skill

# Link to specific agent
$ skm dev link ~/projects/my-skill --agent claude

# Link globally
$ skm dev link ~/projects/my-skill --global

# Override a registry skill
$ skm dev link ~/projects/my-skill --name existing-skill --force
```
```

### 2. Unlink Local Skill

```
Command: skm dev unlink <SKILL_NAME> [OPTIONS]

Description: Unlink a development skill

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Unlink from global scope | Flag | false |
| `--yes`, `-y` | Skip confirmation | Flag | false |
| `--verbose`, `-v` | Show detailed output | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `SKILL_NAME` | Name of the development skill to unlink | Yes | String |

Examples:
```bash
# Unlink a development skill
$ skm dev unlink my-skill

# Unlink from global scope
$ skm dev unlink my-skill --global

# Skip confirmation
$ skm dev unlink my-skill --yes
```
```

### 3. List Development Skills

```
Command: skm dev list [OPTIONS]

Description: List all linked development skills

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Show global development skills | Flag | false |
| `--all` | Show both project and global | Flag | false |
| `--json` | Output in JSON format | Flag | false |
| `--paths` | Show full paths | Flag | false |

Examples:
```bash
# List project development skills
$ skm dev list

# List global development skills
$ skm dev list --global

# List all with paths
$ skm dev list --all --paths

# JSON output
$ skm dev list --json
```
```

### 4. Show Development Skill Info

```
Command: skm dev show <SKILL_NAME> [OPTIONS]

Description: Show information about a development skill

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Show from global scope | Flag | false |
| `--json` | Output in JSON format | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `SKILL_NAME` | Name of the development skill | Yes | String |

Examples:
```bash
# Show info about a development skill
$ skm dev show my-skill

# JSON output
$ skm dev show my-skill --json
```
```

### 5. Development Mode Toggle

```
Command: skm dev mode [on|off|status] [OPTIONS]

Description: Toggle development mode for automatic local skill detection

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Apply to global configuration | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `ACTION` | on, off, or status | Yes | String |

Examples:
```bash
# Enable development mode
$ skm dev mode on

# Check development mode status
$ skm dev mode status

# Disable development mode
$ skm dev mode off

# Enable globally
$ skm dev mode on --global
```
```

## Implementation

### Files to Modify/Create

#### `src/main.rs`
Add dev commands to the `Commands` enum:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Manage local development skills
    #[command(subcommand)]
    Dev(DevCommands),
}

#[derive(Subcommand)]
enum DevCommands {
    /// Link a local directory as a development skill
    Link {
        /// Path to local skill directory
        path: PathBuf,
        /// Skill name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,
        /// Registry source to override
        #[arg(short, long)]
        source: Option<String>,
        /// Link globally instead of in current project
        #[arg(short, long)]
        global: bool,
        /// Link to all available agents
        #[arg(long)]
        all_agents: bool,
        /// Link to specific agent(s)
        #[arg(long)]
        agent: Option<String>,
        /// Override existing skill without warning
        #[arg(short, long)]
        force: bool,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Unlink a development skill
    Unlink {
        /// Name of the development skill to unlink
        skill_name: String,
        /// Unlink from global scope
        #[arg(short, long)]
        global: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// List all linked development skills
    List {
        /// Show global development skills
        #[arg(short, long)]
        global: bool,
        /// Show both project and global
        #[arg(long)]
        all: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show full paths
        #[arg(long)]
        paths: bool,
    },
    
    /// Show information about a development skill
    Show {
        /// Name of the development skill
        skill_name: String,
        /// Show from global scope
        #[arg(short, long)]
        global: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    
    /// Toggle development mode
    Mode {
        /// Action: on, off, or status
        action: String,
        /// Apply to global configuration
        #[arg(short, long)]
        global: bool,
    },
}
```

Add handler in `run()` function:

```rust
Commands::Dev(cmd) => {
    match cmd {
        DevCommands::Link { path, name, source, global, all_agents, agent, force, verbose } => {
            dev::link_local_skill(path, name, source, global, all_agents, agent, force, verbose)?;
        }
        DevCommands::Unlink { skill_name, global, yes, verbose } => {
            dev::unlink_local_skill(&skill_name, global, yes, verbose)?;
        }
        DevCommands::List { global, all, json, paths } => {
            dev::list_local_skills(global, all, json, paths)?;
        }
        DevCommands::Show { skill_name, global, json } => {
            dev::show_local_skill(&skill_name, global, json)?;
        }
        DevCommands::Mode { action, global } => {
            dev::toggle_dev_mode(&action, global)?;
        }
    }
    Ok(())
}
```

#### `src/dev.rs` (New File)
Create a dedicated development module:

```rust
use crate::config::{SkillSpec, SkillsConfig};
use crate::config_manager::{BaseConfig, get_base_config_path};
use crate::linker;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Development skill information
#[derive(Debug, Clone, serde::Serialize)]
pub struct DevSkill {
    pub name: String,
    pub path: PathBuf,
    pub agents: Vec<String>,
    pub global: bool,
    pub overrides: Option<String>,  // Registry skill this overrides
}

/// Development configuration (stored in ~/.config/skm/dev.yaml)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DevConfig {
    pub skills: HashMap<String, DevSkill>,  // Keyed by skill name
    pub mode: bool,  // Development mode enabled/disabled
}

impl DevConfig {
    pub fn load(global: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let path = get_dev_config_path(global)?;
        
        if !path.exists() {
            return Ok(Self {
                skills: HashMap::new(),
                mode: false,
            });
        }
        
        let content = fs::read_to_string(&path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }
    
    pub fn save(&self, global: bool) -> Result<(), Box<dyn std::error::Error>> {
        let path = get_dev_config_path(global)?;
        
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_yaml::to_string(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}

/// Get development config path
fn get_dev_config_path(global: bool) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if global {
        let config_dir = dirs::config_dir().ok_or("Could not determine config directory")?;
        Ok(config_dir.join("skm").join("dev.yaml"))
    } else {
        let current_dir = std::env::current_dir()?;
        Ok(current_dir.join(".skm-dev.yaml"))
    }
}

/// Link a local directory as a development skill
pub fn link_local_skill(
    path: PathBuf,
    name: Option<String>,
    source: Option<String>,
    global: bool,
    all_agents: bool,
    agent: Option<String>,
    force: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate path
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()).into());
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path.display()).into());
    }
    
    // Validate skill structure
    let skill_md = path.join("SKILL.md");
    if !skill_md.exists() {
        return Err(format!(
            "Directory does not contain a valid skill (missing SKILL.md): {}",
            path.display()
        ).into());
    }
    
    // Determine skill name
    let skill_name = name.unwrap_or_else(|| {
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed-skill")
            .to_string()
    });
    
    // Validate skill name
    linker::validate_skill_name(&skill_name)?;
    
    // Load or create dev config
    let mut dev_config = DevConfig::load(global)?;
    
    // Check if skill already exists
    if dev_config.skills.contains_key(&skill_name) {
        if !force {
            return Err(format!(
                "Development skill '{}' already exists. Use --force to override.",
                skill_name
            ).into());
        }
    }
    
    // Determine agents to link to
    let agents = if all_agents {
        // All known agents
        vec!["claude", "cursor", "codex", "copilot", "grok", "hermes"]
            .iter().map(|s| s.to_string()).collect()
    } else if let Some(agent_list) = agent {
        // Parse comma-separated agent list
        agent_list.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        // Default: all available agents (from config or detected)
        if global {
            // For global, use all known agents
            vec!["claude", "cursor", "codex", "copilot", "grok", "hermes"]
                .iter().map(|s| s.to_string()).collect()
        } else {
            // For project, use agents from skills.yaml
            let current_dir = std::env::current_dir()?;
            let config_path = current_dir.join("skills.yaml");
            if config_path.exists() {
                let config = SkillsConfig::load_from_file(&config_path)?;
                config.agents
            } else {
                // Fall back to detected agents
                crate::wizard::detect_available_agents()
            }
        }
    };
    
    // Validate agents
    for agent in &agents {
        linker::validate_agents(&[agent.clone()])?;
    }
    
    // Check for override warning
    if let Some(ref source) = source {
        println!("Warning: This will override any registry skill named '{}' from source '{}'", 
            skill_name, source);
    } else {
        // Check if a skill with this name exists in registries
        if skill_exists_in_registries(&skill_name)? {
            if !force {
                return Err(format!(
                    "A skill named '{}' already exists in a registry. Use --force to override or --source to be explicit.",
                    skill_name
                ).into());
            }
        }
    }
    
    // Create dev skill
    let dev_skill = DevSkill {
        name: skill_name.clone(),
        path: path.clone(),
        agents: agents.clone(),
        global,
        overrides: source.clone(),
    };
    
    // Add to dev config
    dev_config.skills.insert(skill_name.clone(), dev_skill);
    dev_config.save(global)?;
    
    // Link to each agent directory
    let project_root = if global {
        // For global, we need a dummy project root - but global symlinks don't need it
        std::env::current_dir()?
    } else {
        std::env::current_dir()?
    };
    
    for agent_name in &agents {
        let base_dir = if global {
            linker::get_global_agent_skills_dir(agent_name)
        } else {
            linker::get_project_agent_skills_dir(agent_name, &project_root)
        };
        
        let Some(base_dir) = base_dir else {
            if verbose {
                println!("Skipping unknown agent: {}", agent_name);
            }
            continue;
        };
        
        // Create parent directory if needed
        if let Some(parent) = base_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let target_path = base_dir.join(linker::validated_skill_path(&skill_name)?);
        
        // Remove existing symlink or directory if it exists
        if target_path.exists() {
            if target_path.is_symlink() {
                fs::remove_file(&target_path)?;
            } else {
                return Err(format!(
                    "Refusing to replace existing directory: {}",
                    target_path.display()
                ).into());
            }
        }
        
        // Create symlink
        let canonical_path = fs::canonicalize(&path)?;
        linker::symlink_dir(&canonical_path, &target_path)?;
        
        if verbose {
            println!("Linked {} to {} for agent {}", 
                skill_name, path.display(), agent_name);
        }
    }
    
    println!("Linked development skill '{}' from {}", skill_name, path.display());
    println!("Agents: {}", agents.join(", "));
    if global {
        println!("Scope: global");
    } else {
        println!("Scope: project");
    }
    
    Ok(())
}

/// Unlink a development skill
pub fn unlink_local_skill(
    skill_name: &str,
    global: bool,
    yes: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut dev_config = DevConfig::load(global)?;
    
    // Check if skill exists
    if !dev_config.skills.contains_key(skill_name) {
        return Err(format!("Development skill '{}' not found", skill_name).into());
    }
    
    let dev_skill = dev_config.skills.get(skill_name).unwrap();
    
    // Confirm with user
    if !yes {
        print!("Are you sure you want to unlink development skill '{}'? [y/N] ", skill_name);
        std::io::stdout().flush()?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Unlink cancelled.");
            return Ok(());
        }
    }
    
    // Remove from dev config
    dev_config.skills.remove(skill_name);
    dev_config.save(global)?;
    
    // Remove symlinks from each agent
    let project_root = if global {
        std::env::current_dir()?
    } else {
        std::env::current_dir()?
    };
    
    for agent_name in &dev_skill.agents {
        let base_dir = if global {
            linker::get_global_agent_skills_dir(agent_name)
        } else {
            linker::get_project_agent_skills_dir(agent_name, &project_root)
        };
        
        let Some(base_dir) = base_dir else {
            continue;
        };
        
        let target_path = base_dir.join(linker::validated_skill_path(skill_name)?);
        
        if target_path.exists() && target_path.is_symlink() {
            fs::remove_file(&target_path)?;
            if verbose {
                println!("Removed symlink: {}", target_path.display());
            }
        }
    }
    
    println!("Unlinked development skill: {}", skill_name);
    
    Ok(())
}

/// List all linked development skills
pub fn list_local_skills(
    global: bool,
    all: bool,
    json_output: bool,
    show_paths: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut all_skills: Vec<(String, DevSkill, bool)> = Vec::new();  // (name, skill, is_global)
    
    if all || !global {
        // Load project dev config
        if let Ok(mut dev_config) = DevConfig::load(false) {
            for (name, skill) in dev_config.skills.drain() {
                all_skills.push((name, skill, false));
            }
        }
    }
    
    if all || global {
        // Load global dev config
        if let Ok(mut dev_config) = DevConfig::load(true) {
            for (name, skill) in dev_config.skills.drain() {
                all_skills.push((name, skill, true));
            }
        }
    }
    
    if all_skills.is_empty() {
        println!("No development skills found");
        return Ok(());
    }
    
    if json_output {
        let output: Vec<_> = all_skills.iter().map(|(name, skill, is_global)| {
            serde_json::json!({
                "name": name,
                "path": skill.path,
                "agents": skill.agents,
                "global": is_global,
                "overrides": skill.overrides
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Development Skills:");
        println!("{}", "=".repeat(50));
        
        for (name, skill, is_global) in &all_skills {
            let scope = if *is_global { "[global]" } else { "[project]" };
            if show_paths {
                println!("{} {} -> {}", scope, name, skill.path.display());
            } else {
                println!("{} {}", scope, name);
            }
            if let Some(ref override_source) = skill.overrides {
                println!("    Overrides: {}", override_source);
            }
            println!("    Agents: {}", skill.agents.join(", "));
            println!();
        }
    }
    
    Ok(())
}

/// Show information about a development skill
pub fn show_local_skill(
    skill_name: &str,
    global: bool,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let dev_config = DevConfig::load(global)?;
    
    let dev_skill = dev_config.skills.get(skill_name)
        .ok_or_else(|| format!("Development skill '{}' not found", skill_name).into())?;
    
    if json_output {
        let output = serde_json::json!({
            "name": dev_skill.name,
            "path": dev_skill.path,
            "path_absolute": fs::canonicalize(&dev_skill.path)?,
            "agents": dev_skill.agents,
            "global": global,
            "overrides": dev_skill.overrides,
            "has_skill_md": dev_skill.path.join("SKILL.md").exists()
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Development Skill: {}", dev_skill.name);
        println!("{}", "-".repeat(50));
        println!("Path: {}", dev_skill.path.display());
        if let Ok(canonical) = fs::canonicalize(&dev_skill.path) {
            println!("Absolute: {}", canonical.display());
        }
        println!("Global: {}", global);
        println!("Agents: {}", dev_skill.agents.join(", "));
        if let Some(ref override_source) = dev_skill.overrides {
            println!("Overrides: {}", override_source);
        }
        println!("SKILL.md exists: {}", dev_skill.path.join("SKILL.md").exists());
        
        // Show linked symlinks
        println!("\nLinked to:");
        let project_root = if global {
            std::env::current_dir()?
        } else {
            std::env::current_dir()?
        };
        
        for agent_name in &dev_skill.agents {
            let base_dir = if global {
                linker::get_global_agent_skills_dir(agent_name)
            } else {
                linker::get_project_agent_skills_dir(agent_name, &project_root)
            };
            
            if let Some(base_dir) = base_dir {
                let symlink_path = base_dir.join(linker::validated_skill_path(&dev_skill.name)?);
                if symlink_path.exists() {
                    println!("  {}: {}", agent_name, symlink_path.display());
                } else {
                    println!("  {}: NOT LINKED", agent_name);
                }
            }
        }
    }
    
    Ok(())
}

/// Toggle development mode
pub fn toggle_dev_mode(
    action: &str,
    global: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut dev_config = DevConfig::load(global)?;
    
    match action {
        "on" | "enable" | "true" => {
            dev_config.mode = true;
            dev_config.save(global)?;
            println!("Development mode enabled");
        }
        "off" | "disable" | "false" => {
            dev_config.mode = false;
            dev_config.save(global)?;
            println!("Development mode disabled");
        }
        "status" | "show" | "get" => {
            println!("Development mode: {}", if dev_config.mode { "enabled" } else { "disabled" });
        }
        _ => {
            return Err(format!("Unknown action: '{}'. Use on, off, or status.", action).into());
        }
    }
    
    Ok(())
}

/// Check if a skill exists in any registry
fn skill_exists_in_registries(skill_name: &str) -> bool {
    let base_config = BaseConfig::load().ok();
    if base_config.is_none() {
        return false;
    }
    
    let base_config = base_config.unwrap();
    
    for (_name, url) in &base_config.registries {
        let cache_path = linker::resolve_registry_path(_name);
        if cache_path.is_none() {
            continue;
        }
        let cache_path = cache_path.unwrap();
        if !cache_path.exists() {
            continue;
        }
        
        let skills_dir = cache_path.join("skills");
        if !skills_dir.exists() {
            continue;
        }
        
        let skill_path = skills_dir.join(linker::validated_skill_path(skill_name).unwrap());
        if skill_path.exists() {
            return true;
        }
    }
    
    false
}

/// Auto-discover and link local skills (when dev mode is enabled)
pub fn auto_discover_local_skills(global: bool) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let dev_config = DevConfig::load(global)?;
    
    if !dev_config.mode {
        return Ok(Vec::new());
    }
    
    // Look for .skm-local directory or other indicators
    // This is a placeholder for auto-discovery logic
    
    Ok(Vec::new())
}
```

#### `src/linker.rs`
Add function to create symlinks (platform-specific):

```rust
// Already exists in the current code
#[cfg(unix)]
fn symlink_dir(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(windows)]
fn symlink_dir(source: &Path, target: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(source, target)
}

// Make it public so dev.rs can use it
pub use symlink_dir;
```

#### `src/config_manager.rs`
No changes needed.

#### `Cargo.toml`
No changes needed - serde already handles YAML serialization.

## Behavior Flow

### Link Local Skill
```
1. User runs: skm dev link ~/projects/my-skill
2. SKM validates the directory exists and contains SKILL.md
3. SKM determines skill name (from --name or directory)
4. SKM validates skill name
5. SKM checks if skill already exists in dev config
6. If override: Warn user (unless --force)
7. SKM determines agents to link to
8. SKM creates dev config entry
9. SKM creates symlinks from local dir to each agent directory
10. SKM prints confirmation
```

### Unlink Local Skill
```
1. User runs: skm dev unlink my-skill
2. SKM checks if skill exists in dev config
3. SKM prompts for confirmation (unless --yes)
4. SKM removes from dev config
5. SKM removes symlinks from each agent directory
6. SKM prints confirmation
```

### List Development Skills
```
1. User runs: skm dev list
2. SKM loads dev config (project and/or global)
3. SKM formats output (text or JSON)
4. SKM prints list
```

### Show Development Skill Info
```
1. User runs: skm dev show my-skill
2. SKM loads dev config
3. SKM finds the skill
4. SKM gathers information (path, agents, symlinks)
5. SKM formats output (text or JSON)
6. SKM prints info
```

### Toggle Dev Mode
```
1. User runs: skm dev mode on
2. SKM loads dev config
3. SKM updates mode setting
4. SKM saves dev config
5. SKM prints status
```

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Path doesn't exist | Clear error with path |
| Path is not a directory | Clear error with path |
| Missing SKILL.md | Clear error with path |
| Invalid skill name | Clear error with validation details |
| Skill already exists | Clear error with instructions to use --force |
| Agent doesn't exist | Warning, skip that agent |
| Existing non-symlink directory | Refuse to overwrite, clear error |
| Permission denied | Clear error with path |

## Testing Requirements

### Link Local Skill
1. Test linking a valid skill directory
2. Test linking with custom name
3. Test linking to specific agents
4. Test linking globally
5. Test override with --force
6. Test linking directory without SKILL.md
7. Test linking non-existent directory

### Unlink Local Skill
1. Test unlinking an existing dev skill
2. Test unlinking with --yes
3. Test unlinking non-existent skill
4. Test unlinking globally

### List Development Skills
1. Test listing project dev skills
2. Test listing global dev skills
3. Test listing with --all
4. Test JSON output
5. Test with --paths flag
6. Test with no dev skills

### Show Development Skill Info
1. Test showing info for existing skill
2. Test showing info for non-existent skill
3. Test JSON output

### Toggle Dev Mode
1. Test enabling dev mode
2. Test disabling dev mode
3. Test checking status
4. Test invalid action

## Migration Path

This feature is fully backward compatible:
- Existing configurations are unaffected
- No changes to existing commands
- New commands are additive
- Development config is stored separately from main config

## User Experience

### Link Output
```
$ skm dev link ~/projects/my-skill

Linked development skill 'my-skill' from /home/user/projects/my-skill
Agents: claude, cursor, codex
Scope: project

The skill is now available to your agents. Changes to files in
/home/user/projects/my-skill will be immediately visible.
```

### Link with Override Warning
```
$ skm dev link ~/projects/my-skill --name existing-skill

Warning: This will override any registry skill named 'existing-skill'
Linked development skill 'existing-skill' from /home/user/projects/my-skill
Agents: claude, cursor
Scope: project
```

### List Output
```
$ skm dev list

Development Skills:
==================================================

[project] my-skill
    -> /home/user/projects/my-skill
    Agents: claude, cursor, codex

[global] shared-skill
    -> /home/user/shared-skills/shared-skill
    Agents: claude, cursor
    Overrides: default
```

### JSON Output
```json
[
  {
    "name": "my-skill",
    "path": "/home/user/projects/my-skill",
    "agents": ["claude", "cursor", "codex"],
    "global": false,
    "overrides": null
  },
  {
    "name": "shared-skill",
    "path": "/home/user/shared-skills/shared-skill",
    "agents": ["claude", "cursor"],
    "global": true,
    "overrides": "default"
  }
]
```

### Show Output
```
$ skm dev show my-skill

Development Skill: my-skill
--------------------------------------------------
Path: /home/user/projects/my-skill
Absolute: /home/user/projects/my-skill
Global: false
Agents: claude, cursor, codex
SKILL.md exists: true

Linked to:
  claude: /home/user/.claude/skills/my-skill
  cursor: /home/user/.cursor/skills/my-skill
  codex: /home/user/.codex/skills/my-skill
```

### Dev Mode Output
```
$ skm dev mode on
Development mode enabled

$ skm dev mode status
Development mode: enabled

$ skm dev mode off
Development mode disabled
```

## Related Features

- `skm add --path` - Add local skill (but creates copies, not symlinks)
- `skm link` - Install skills (could be enhanced to support dev mode)
- `skm list` - List skills (should show dev skills with indicator)
- `skm check` - Verify skills (should verify dev skills)
- Symlink creation - Already implemented in linker.rs

## Future Enhancements

1. **Auto-discovery**: Automatically detect and link directories with SKILL.md files
2. **Watch mode**: Watch for changes in dev skills and report them
3. **Dev skill validation**: Validate dev skills match registry schema
4. **Sync to registry**: Command to sync a dev skill to a registry
5. **Dev skill dependencies**: Support dependencies between dev skills
