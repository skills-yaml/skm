# Specification: Skill Version Management

## Overview

SKM needs robust version management capabilities to allow users to work with different versions of skills, update to latest versions, and inspect available versions. This is critical for production environments where version control and reproducibility are essential.

## Problem Statement

Currently, SKM:
1. Uses `latest` symlink in registries to point to the current version
2. Has no way to list available versions for a skill
3. Has no way to pin a skill to a specific version
4. Has no way to update a skill to a newer version
5. Has no way to downgrade a skill to an older version

This makes it difficult to:
- Maintain reproducible builds
- Test with different skill versions
- Roll back to known-good versions
- Understand what versions are available

## Requirements

### Version Discovery

#### R1: List Available Versions
SKM must provide a command to list all available versions for a given skill.

#### R2: Show Current Version
SKM must display the currently installed version of a skill.

#### R3: Version Sorting
SKM must sort versions semantically (not lexicographically).

#### R4: Filter Versions
SKM must support filtering versions (e.g., only stable, only prerelease).

### Version Selection

#### R5: Pin to Specific Version
SKM must allow pinning a skill to a specific version in skills.yaml.

#### R6: Update to Latest
SKM must allow updating a skill to its latest version.

#### R7: Version Validation
SKM must validate that the requested version exists before linking.

### Version Metadata

#### R8: Version Information
SKM must display metadata for each version (date, description, etc. if available).

#### R9: Changelog Access
SKM must provide access to version changelogs if available in the registry.

### Caching & Performance

#### R10: Version Cache
SKM must cache the list of available versions to avoid repeated registry access.

#### R11: Cache Invalidation
SKM must invalidate the version cache when the registry is updated.

## Command Specifications

### 1. List Versions

```
Command: skm versions <SKILL_NAME> [OPTIONS]

Description: List all available versions for a skill

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--registry`, `-r` | Specific registry to query | String | default |
| `--json` | Output in JSON format | Flag | false |
| `--stable-only` | Only show stable versions | Flag | false |
| `--pre` | Include prerelease versions | Flag | false |
| `--limit`, `-n` | Limit number of versions shown | Number | 50 |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `SKILL_NAME` | Name of the skill | Yes | String |

Examples:
```bash
# List all versions for a skill
$ skm versions software-development/symphony-spec-writing

# Output:
software-development/symphony-spec-writing:
  v1.2.0 (latest, 2024-01-15)
  v1.1.0
  v1.0.0
  v0.9.0

# List only stable versions
$ skm versions my-skill --stable-only

# Output in JSON
$ skm versions my-skill --json
```
```

### 2. Use Specific Version

```
Command: skm use <SKILL_NAME>@<VERSION> [OPTIONS]

Description: Switch a skill to a specific version

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Apply to global configuration | Flag | false |
| `--yes`, `-y` | Skip confirmation | Flag | false |
| `--dry-run` | Preview changes | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `SKILL_VERSION` | Skill name with version (e.g., my-skill@v1.2.0) | Yes | String |

Examples:
```bash
# Switch to a specific version
$ skm use software-development/symphony-spec-writing@v1.1.0

# Switch globally
$ skm use my-skill@v2.0.0 --global

# Preview changes
$ skm use my-skill@v1.0.0 --dry-run
```
```

### 3. Update Skill

```
Command: skm update <SKILL_NAME> [OPTIONS]

Description: Update a skill to its latest version

Options:
| Option | Description | Type | Default |
|--------|-------------|------|---------|
| `--global`, `-g` | Update in global configuration | Flag | false |
| `--yes`, `-y` | Skip confirmation | Flag | false |
| `--dry-run` | Preview changes | Flag | false |
| `--pre` | Update to latest prerelease | Flag | false |

Arguments:
| Argument | Description | Required | Type |
|----------|-------------|----------|------|
| `SKILL_NAME` | Name of the skill to update | Yes | String |

Examples:
```bash
# Update to latest version
$ skm update software-development/symphony-spec-writing

# Update all skills in project
$ skm update --all

# Update to latest prerelease
$ skm update my-skill --pre
```
```

## Implementation

### Files to Modify

#### `src/main.rs`
Add new commands to the `Commands` enum:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// List all available versions for a skill
    Versions {
        /// Name of the skill
        skill_name: String,
        /// Specific registry to query
        #[arg(short, long)]
        registry: Option<String>,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Only show stable versions
        #[arg(long)]
        stable_only: bool,
        /// Include prerelease versions
        #[arg(long)]
        pre: bool,
        /// Limit number of versions shown
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    
    /// Switch a skill to a specific version
    Use {
        /// Skill and version (format: skill@v1.2.0)
        skill_version: String,
        /// Apply to global configuration
        #[arg(short, long)]
        global: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Preview changes
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Update a skill to its latest version
    UpdateSkill {
        /// Name of the skill to update
        skill_name: String,
        /// Update in global configuration
        #[arg(short, long)]
        global: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Preview changes
        #[arg(long)]
        dry_run: bool,
        /// Update to prerelease version
        #[arg(long)]
        pre: bool,
    },
}
```

#### `src/config.rs`
Update `SkillSpec` to support version pinning:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillSpec {
    pub name: String,
    pub version: Option<String>,  // Can be "latest", "v1.2.0", etc.
    pub source: Option<String>,
    pub path: Option<String>,
}

impl SkillSpec {
    /// Parse skill spec with version (e.g., "my-skill@v1.2.0")
    pub fn parse_with_version(input: &str) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
        if let Some(at_pos) = input.rfind('@') {
            let name = &input[..at_pos];
            let version = Some(input[at_pos + 1..].to_string());
            Ok((name.to_string(), version))
        } else {
            Ok((input.to_string(), None))
        }
    }
}
```

#### `src/linker.rs`
Update `resolve_version_path` to handle pinned versions:

```rust
/// Resolves the version component of a skill path.
///
/// Version resolution order:
/// 1. If version is explicitly set (e.g., "v1.2.3") → use as-is
/// 2. If version is "latest" → use "latest" (follows symlink)
/// 3. If version is None → use "latest"
pub fn resolve_version_path(skill: &SkillSpec) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let version = skill.version.as_deref().unwrap_or("latest");

    // Normalize version to path component
    let version_path = if version == "latest" {
        // Use as-is, will follow symlink
        PathBuf::from(version)
    } else if version.starts_with('v') {
        // Already has v prefix
        PathBuf::from(version)
    } else {
        // Add v prefix for semantic versions
        PathBuf::from(format!("v{}", version))
    };

    // Validate version path is safe
    if !is_safe_version_path(&version_path) {
        return Err(format!("Invalid version path: {}", version_path.display()).into());
    }

    Ok(version_path)
}

/// List all available versions for a skill from a registry
pub fn list_skill_versions(
    skill_name: &str,
    registry_name: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let registry_path = resolve_registry_path(registry_name)
        .ok_or_else(|| format!("Could not resolve path for registry: {}", registry_name))?;
    
    let skill_path = registry_path.join(validated_skill_path(skill_name)?);
    
    if !skill_path.exists() {
        return Err(format!("Skill '{}' not found in registry '{}'", skill_name, registry_name).into());
    }
    
    let mut versions = Vec::new();
    
    // Read directory entries
    for entry in std::fs::read_dir(&skill_path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                // Skip non-version directories
                if name.starts_with('v') || name == "latest" {
                    versions.push(name.to_string());
                }
            }
        }
    }
    
    // Sort versions semantically
    versions.sort_by(|a, b| compare_versions(a, b));
    
    Ok(versions)
}

/// Compare two version strings semantically
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    // Simple semantic version comparison
    // In production, use semver crate
    if a == "latest" {
        std::cmp::Ordering::Greater
    } else if b == "latest" {
        std::cmp::Ordering::Less
    } else {
        // Compare as strings (for now)
        a.cmp(b)
    }
}
```

#### `src/version_manager.rs` (New File)
Create a dedicated version management module:

```rust
use crate::config::SkillSpec;
use crate::linker;
use std::path::Path;

/// List all versions available for a skill
pub fn list_versions(
    skill_name: &str,
    registry_name: Option<&str>,
    stable_only: bool,
    include_pre: bool,
    limit: usize,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let registry = registry_name.unwrap_or("default");
    let mut versions = linker::list_skill_versions(skill_name, registry)?;
    
    // Filter versions
    if stable_only {
        versions.retain(|v| !is_prerelease(v));
    }
    
    if !include_pre {
        versions.retain(|v| !is_prerelease(v));
    }
    
    // Apply limit
    if versions.len() > limit {
        versions = versions.into_iter().take(limit).collect();
    }
    
    Ok(versions)
}

/// Check if a version is a prerelease
fn is_prerelease(version: &str) -> bool {
    // Check for common prerelease indicators
    version.contains("-alpha") ||
    version.contains("-beta") ||
    version.contains("-rc") ||
    version.contains("-pre") ||
    version.contains("+dev")
}

/// Update a skill to a specific version
pub fn use_version(
    skill_name: &str,
    version: &str,
    config_path: &Path,
    global: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = SkillsConfig::load_from_file(config_path)?;
    
    // Find and update the skill
    let skill_index = config.skills.iter()
        .position(|s| s.name == skill_name)
        .ok_or_else(|| format!("Skill '{}' not found in configuration", skill_name))?;
    
    let skill = &mut config.skills[skill_index];
    skill.version = Some(version.to_string());
    
    if dry_run {
        println!("Would update '{}' to version '{}'", skill_name, version);
        // Also show what would be re-linked
        let project_root = config_path.parent().unwrap();
        for agent in &config.agents {
            let base = linker::get_agent_skills_dir(agent, project_root, global)?;
            let old_path = linker::get_skill_target_path(&base, skill_name)?;
            let new_path = linker::resolve_skill_source_dir(skill, project_root)?;
            println!("Would relink for agent '{}': {} -> {}", agent, old_path.display(), new_path.display());
        }
        return Ok(());
    }
    
    // Save updated configuration
    config.save_to_file(config_path)?;
    
    // Re-link the skill with new version
    let project_root = config_path.parent().unwrap();
    for agent in &config.agents {
        linker::link_skill(skill, &project_root, &config.agents, global)?;
    }
    
    println!("Updated '{}' to version '{}'", skill_name, version);
    
    Ok(())
}

/// Update a skill to its latest version
pub fn update_to_latest(
    skill_name: &str,
    config_path: &Path,
    global: bool,
    pre: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get current config
    let config = SkillsConfig::load_from_file(config_path)?;
    
    // Find the skill
    let skill = config.skills.iter()
        .find(|s| s.name == skill_name)
        .ok_or_else(|| format!("Skill '{}' not found in configuration", skill_name))?;
    
    let registry = skill.source.as_deref().unwrap_or("default");
    
    // List versions and find latest
    let versions = list_versions(skill_name, Some(registry), false, pre, 100)?;
    let latest = versions.first()
        .ok_or_else(|| format!("No versions found for skill '{}'", skill_name))?;
    
    // Use the latest version
    use_version(skill_name, latest, config_path, global, dry_run)
}
```

#### `src/config_manager.rs`
Update `first_time_setup` to include version info in default config:

```rust
// No changes needed - version is already supported in SkillSpec
```

## Behavior Flow

### List Versions
```
1. User runs: skm versions my-skill
2. SKM resolves registry for my-skill (default or specified)
3. SKM lists directory contents in registry/skills/my-skill/
4. SKM filters by version pattern (v*, latest)
5. SKM sorts versions semantically
6. SKM applies filters (stable-only, pre, limit)
7. SKM displays versions
```

### Use Specific Version
```
1. User runs: skm use my-skill@v1.2.0
2. SKM parses skill name and version
3. SKM validates version exists in registry
4. If --dry-run: Show what would change
5. SKM updates skills.yaml with new version
6. SKM re-links skill with new version
7. SKM prints confirmation
```

### Update to Latest
```
1. User runs: skm update my-skill
2. SKM finds current skill configuration
3. SKM queries registry for all versions
4. SKM identifies latest version
5. SKM updates skills.yaml with latest version
6. SKM re-links skill with latest version
7. SKM prints confirmation
```

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Skill not found | Clear error message |
| Version not found | Clear error message with available versions |
| Registry not found | Clear error message |
| Permission denied | Clear error with path |
| Invalid version format | Clear error with expected format |
| Network error | Clear error, suggest retry |

## Testing Requirements

### Version Listing
1. Test listing versions for a skill with multiple versions
2. Test listing versions with --json flag
3. Test filtering with --stable-only
4. Test filtering with --pre
5. Test limiting with --limit
6. Test error when skill not found
7. Test error when registry not found

### Version Selection
1. Test switching to a specific version
2. Test switching with --dry-run
3. Test switching with --global
4. Test error when version not found
5. Test that symlinks are updated to point to new version

### Version Update
1. Test updating to latest version
2. Test updating with --dry-run
3. Test updating with --pre flag
4. Test error when no versions available
5. Test that version in skills.yaml is updated

## Migration Path

This feature is backward compatible:
- Existing skills without explicit versions will continue to use "latest"
- Version field is already optional in SkillSpec
- New commands are additive

For existing users:
- Skills without version specified will continue to use latest
- No changes required to existing configurations

## User Experience

### List Versions Output
```
$ skm versions software-development/symphony-spec-writing

software-development/symphony-spec-writing (registry: default):
  v1.5.0 (latest, 2024-06-15) - Feature X, Bug fixes
  v1.4.0 (2024-05-01)
  v1.3.2 (2024-04-15) - Security patch
  v1.3.0 (2024-03-01)
  v1.2.0 (2024-02-01)

5 versions found. Use `skm use software-development/symphony-spec-writing@v1.5.0` to switch.
```

### JSON Output
```json
{
  "skill": "software-development/symphony-spec-writing",
  "registry": "default",
  "versions": ["v1.5.0", "v1.4.0", "v1.3.2", "v1.3.0", "v1.2.0"],
  "latest": "v1.5.0",
  "count": 5
}
```

### Use Version Output
```
$ skm use software-development/symphony-spec-writing@v1.3.2

Updating 'software-development/symphony-spec-writing' from v1.5.0 to v1.3.2
Relinking for agent 'claude': ~/.claude/skills/software-development/symphony-spec-writing
Relinking for agent 'cursor': ~/.cursor/skills/software-development/symphony-spec-writing
Done. Skill now using v1.3.2
```

### Update Output
```
$ skm update software-development/symphony-spec-writing

Current version: v1.3.2
Latest version: v1.5.0
Updating 'software-development/symphony-spec-writing' to v1.5.0
Relinking for agent 'claude': ~/.claude/skills/software-development/symphony-spec-writing
Done. Skill now using v1.5.0
```

## Related Features

- `skm add` - Add skills (supports version specification)
- `skm list` - List skills (should show version)
- `skm check` - Verify skills (should verify correct version)
- Registry structure - Must support versioned directories
