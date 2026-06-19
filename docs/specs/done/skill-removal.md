# Specification: Skill Removal Feature

## Overview

SKM needs the ability to remove skills from a project configuration and unlink them from agent directories. This is essential for managing skill lifecycles and keeping projects clean.

## Problem Statement

Currently, once a skill is added to a project via `skm add`, there is no built-in way to remove it. Users must:
1. Manually edit `skills.yaml` to remove the skill definition
2. Manually remove symlinks from each agent's skills directory
3. Risk leaving orphaned symlinks if not done correctly

This creates a poor user experience and potential for configuration inconsistencies.

## Requirements

### R1: Remove from Configuration
The command must remove the skill definition from the project's `skills.yaml` file.

### R2: Unlink from Agent Directories
The command must remove symlinks for the skill from all configured agent directories.

### R3: Support Both Scopes
The command must support both project-local and global removal modes.

### R4: Safety Checks
Before removing, the command must verify that:
- The skill exists in the configuration
- The symlinks exist and are valid (point to cached skills)
- No data will be lost (refuse to remove if target is not a symlink)

### R5: Confirmation Prompt
The command must prompt for confirmation before performing destructive actions.

### R6: Force Mode
The command must support a `--force` flag to skip confirmation for automation.

### R7: Dry Run
The command must support a `--dry-run` flag to preview actions without making changes.

### R8: Partial Success Handling
If removal succeeds for some agents but fails for others, the command must:
- Report partial success
- Indicate which agents failed
- Not roll back successful removals

### R9: Error Handling
The command must provide clear error messages for:
- Skill not found in configuration
- Symlink removal failures
- Permission issues
- Invalid skill names

### R10: Idempotent Operation
Running `skm remove` on an already-removed skill must succeed silently (not an error).

## Command Specification

### Primary Command
```
skm remove <SKILL_NAME> [OPTIONS]
```

### Options
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global` | Remove from global agent directories instead of project-local | Flag | false |
| `--yes`, `-y` | Skip confirmation prompt | Flag | false |
| `--force` | Remove even if target exists (non-symlink) | Flag | false |
| `--dry-run` | Preview actions without making changes | Flag | false |
| `--verbose`, `-v` | Show detailed output | Flag | false |

### Arguments
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `SKILL_NAME` | Name of the skill to remove | Yes | String |

## Examples

### Basic Removal
```bash
# Remove a skill from current project
$ skm remove software-development/symphony-spec-writing

# Prompts: "Are you sure you want to remove 'software-development/symphony-spec-writing'? [y/N]"
# Removes from skills.yaml
# Removes symlinks from ./.claude/skills/, ./.cursor/skills/, etc.
```

### Global Removal
```bash
# Remove a skill from global configuration
$ skm remove my-global-skill --global

# Removes symlinks from ~/.claude/skills/, ~/.cursor/skills/, etc.
```

### Force Removal
```bash
# Skip confirmation
$ skm remove old-skill --yes

# Force remove even if directory exists
$ skm remove problematic-skill --force
```

### Dry Run
```bash
# Preview what would be removed
$ skm remove test-skill --dry-run

# Output:
# Would remove skill 'test-skill' from skills.yaml
# Would remove symlink: ./.claude/skills/test-skill
# Would remove symlink: ./.cursor/skills/test-skill
```

## Implementation

### Files to Modify

#### `src/main.rs`
Add `Remove` command to the `Commands` enum:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Remove a skill from skills.yaml and unlink it from agent directories
    Remove {
        /// Name of the skill to remove
        skill_name: String,
        /// Remove from global agent directories instead of project-local
        #[arg(short, long)]
        global: bool,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
        /// Remove even if target is not a symlink (use with caution)
        #[arg(long)]
        force: bool,
        /// Preview actions without making changes
        #[arg(long)]
        dry_run: bool,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}
```

Add handler in `run()` function:

```rust
Commands::Remove {
    skill_name,
    global,
    yes,
    force,
    dry_run,
    verbose,
} => {
    let config_path = current_dir.join("skills.yaml");
    let config = load_config(&config_path)?;
    
    remove_skill(&config, &skill_name, global, yes, force, dry_run, verbose)?;
    Ok(())
}
```

#### `src/linker.rs`
Add new function to remove skill symlinks:

```rust
/// Remove a skill from all configured agent directories
pub fn unlink_skill(
    skill_name: &str,
    project_root: &Path,
    agents: &[String],
    global: bool,
    force: bool,
    dry_run: bool,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut removed = Vec::new();
    
    for agent in agents {
        let base_dir = get_agent_skills_dir(agent, project_root, global)?;
        let skill_path = get_skill_target_path(&base_dir, skill_name)?;
        
        if !skill_path.exists() {
            if verbose {
                println!("Symlink already missing for agent '{}': {}", agent, skill_path.display());
            }
            continue;
        }
        
        if !skill_path.is_symlink() {
            if force {
                if dry_run {
                    println!("Would remove non-symlink directory: {}", skill_path.display());
                    removed.push(skill_path);
                    continue;
                }
                fs::remove_dir_all(&skill_path)?;
                removed.push(skill_path);
            } else {
                return Err(format!(
                    "Refusing to remove non-symlink directory for agent '{}': {}",
                    agent,
                    skill_path.display()
                ).into());
            }
            continue;
        }
        
        // It's a symlink, safe to remove
        if dry_run {
            println!("Would remove symlink: {}", skill_path.display());
            removed.push(skill_path);
        } else {
            fs::remove_file(&skill_path)?;
            removed.push(skill_path);
        }
    }
    
    Ok(removed)
}
```

#### `src/config.rs`
Add function to remove skill from configuration:

```rust
impl SkillsConfig {
    // ... existing methods ...
    
    /// Remove a skill from the configuration
    pub fn remove_skill(&mut self, skill_name: &str) -> Option<SkillSpec> {
        let index = self.skills.iter()
            .position(|s| s.name == skill_name)?;
        self.skills.remove(index)
    }
}
```

#### `src/remover.rs` (New File)
Create a dedicated module for removal operations:

```rust
use crate::config::{SkillSpec, SkillsConfig};
use crate::linker;
use std::fs;
use std::path::Path;

/// Remove a skill from configuration and unlink from agents
pub fn remove_skill(
    config: &SkillsConfig,
    skill_name: &str,
    global: bool,
    yes: bool,
    force: bool,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if skill exists
    if !config.skills.iter().any(|s| s.name == skill_name) {
        if verbose {
            println!("Skill '{}' not found in configuration", skill_name);
        }
        return Ok(()); // Idempotent - not an error
    }
    
    // Show what will be removed
    if dry_run || verbose {
        println!("Removing skill '{}' from configuration", skill_name);
    }
    
    // Unlink from agents
    let project_root = std::env::current_dir()?;
    let removed_links = linker::unlink_skill(
        skill_name,
        &project_root,
        &config.agents,
        global,
        force,
        dry_run,
        verbose,
    )?;
    
    if dry_run {
        println!("Dry run complete. {} symlinks would be removed.", removed_links.len());
        return Ok(());
    }
    
    // Confirm with user if not --yes
    if !yes {
        print!("Are you sure you want to remove '{}'? [y/N] ", skill_name);
        std::io::stdout().flush()?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Removal cancelled.");
            return Ok(());
        }
    }
    
    // Remove from configuration
    let mut config_path = project_root.join("skills.yaml");
    if global {
        // For global removal, use the global config path
        config_path = crate::config_manager::get_base_config_path()
            .ok_or("Could not determine config path for global removal")?;
    }
    
    let mut config = SkillsConfig::load_from_file(&config_path)?;
    let removed = config.remove_skill(skill_name);
    
    if removed.is_some() {
        config.save_to_file(&config_path)?;
        println!("Removed skill '{}' from configuration", skill_name);
        println!("Removed {} symlinks from agent directories", removed_links.len());
    }
    
    Ok(())
}
```

## Behavior Flow

```
1. User runs: skm remove my-skill
2. SKM checks if my-skill exists in skills.yaml
   - If not found: Print message and exit (idempotent)
3. SKM identifies all agent directories with my-skill symlinks
4. If --dry-run: Show what would be removed and exit
5. If --yes: Skip confirmation
6. If not --yes: Prompt user for confirmation
   - If user declines: Exit without changes
7. Remove skill definition from skills.yaml
8. Remove symlinks from all agent directories
9. Print summary of removed items
```

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Skill not in configuration | Print message, exit successfully (idempotent) |
| Symlink is not a symlink (directory/file) | Error unless --force is used |
| Permission denied | Clear error message with path |
| Config file not writable | Clear error message |
| Agent directory doesn't exist | Skip that agent, continue with others |

## Testing Requirements

1. **Basic Removal**
   - Test removing a skill from project configuration
   - Verify skill removed from skills.yaml
   - Verify symlinks removed from all agent directories

2. **Global Removal**
   - Test removing a skill with --global flag
   - Verify symlinks removed from global agent directories

3. **Confirmation Prompt**
   - Test that prompt appears when not using --yes
   - Test that removal is cancelled when user declines

4. **Force Mode**
   - Test removing a non-symlink directory with --force
   - Test that removal fails without --force

5. **Dry Run**
   - Test that --dry-run shows what would be removed
   - Test that no changes are made with --dry-run

6. **Idempotent Operation**
   - Test removing an already-removed skill
   - Verify no error is returned

7. **Partial Success**
   - Test when removal succeeds for some agents but fails for others
   - Verify partial success is reported

8. **Safety Checks**
   - Test that non-symlink directories are not removed without --force
   - Test that permission errors are handled gracefully

## Migration Path

This feature is fully backward compatible:
- Existing configurations are unaffected
- No changes to existing commands
- New command is additive

## User Experience

### Success Output
```
Removed skill 'software-development/symphony-spec-writing' from configuration
Removed 3 symlinks from agent directories:
  - ~/.claude/skills/software-development/symphony-spec-writing
  - ~/.cursor/skills/software-development/symphony-spec-writing
  - ~/.codex/skills/software-development/symphony-spec-writing
```

### Dry Run Output
```
Would remove skill 'software-development/symphony-spec-writing' from configuration
Would remove symlink: ~/.claude/skills/software-development/symphony-spec-writing
Would remove symlink: ~/.cursor/skills/software-development/symphony-spec-writing
Dry run complete. 2 symlinks would be removed.
```

### Error Output
```
Error: Refusing to remove non-symlink directory for agent 'claude': /home/user/.claude/skills/my-skill
The directory contains files that were not created by SKM.
Use --force to override this safety check.
```

## Related Features

- `skm add` - Add a skill (opposite of remove)
- `skm list` - List skills (should show removed skills are gone)
- `skm check` - Verify skills (should not check removed skills)
