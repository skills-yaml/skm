use crate::config::SkillsConfig;
use crate::config_manager::BaseConfig;
use crate::linker;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Development skill information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DevSkill {
    pub name: String,
    pub path: PathBuf,
    pub agents: Vec<String>,
    pub global: bool,
    pub overrides: Option<String>, // Registry skill this overrides
}

/// Development configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DevConfig {
    pub skills: HashMap<String, DevSkill>, // Keyed by skill name
    pub mode: bool,                        // Development mode enabled/disabled
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
#[allow(clippy::too_many_arguments)]
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
    let canonical_path = fs::canonicalize(&path)?;
    if !canonical_path.is_dir() {
        return Err(format!("Path is not a directory: {}", path.display()).into());
    }

    // Validate skill structure
    let skill_md = canonical_path.join("SKILL.md");
    if !skill_md.exists() {
        return Err(format!(
            "Directory does not contain a valid skill (missing SKILL.md): {}",
            path.display()
        )
        .into());
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

    // Check if skill already exists in dev config
    if dev_config.skills.contains_key(&skill_name) && !force {
        return Err(format!(
            "Development skill '{}' already exists. Use --force to override.",
            skill_name
        )
        .into());
    }

    // Determine agents to link to
    let agents = if all_agents {
        // All known agents
        ["claude", "cursor", "codex", "copilot", "grok", "hermes"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else if let Some(agent_list) = agent {
        // Parse comma-separated agent list
        agent_list
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        // Default: all available agents (from config or detected)
        if global {
            // For global, use all known agents
            ["claude", "cursor", "codex", "copilot", "grok", "hermes"]
                .iter()
                .map(|s| s.to_string())
                .collect()
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
        linker::validate_agents(std::slice::from_ref(agent))?;
    }

    // Check for override warning and determine override source
    let override_source = if let Some(ref src) = source {
        println!(
            "Warning: This will override any registry skill named '{}' from source '{}'",
            skill_name, src
        );
        Some(src.clone())
    } else if skill_exists_in_registries(&skill_name) {
        if !force {
            return Err(format!(
                "A skill named '{}' already exists in a registry. Use --force to override or --source to be explicit.",
                skill_name
            ).into());
        }
        let reg = find_skill_registry(&skill_name).unwrap_or_else(|| "default".to_string());
        println!(
            "Warning: This will override any registry skill named '{}'",
            skill_name
        );
        Some(reg)
    } else {
        None
    };

    // Create dev skill
    let dev_skill = DevSkill {
        name: skill_name.clone(),
        path: canonical_path.clone(),
        agents: agents.clone(),
        global,
        overrides: override_source,
    };

    // Add to dev config
    dev_config.skills.insert(skill_name.clone(), dev_skill);
    dev_config.save(global)?;

    // Link to each agent directory
    let project_root = std::env::current_dir()?;

    for agent_name in &agents {
        let base_dir = match linker::get_agent_skills_dir(agent_name, &project_root, global) {
            Ok(dir) => dir,
            Err(e) => {
                if verbose {
                    println!("Skipping unknown agent {}: {}", agent_name, e);
                }
                continue;
            }
        };

        if let Some(parent) = base_dir.parent() {
            fs::create_dir_all(parent)?;
        }

        let target_path = base_dir.join(linker::validated_skill_path(&skill_name)?);

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        match fs::symlink_metadata(&target_path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                fs::remove_file(&target_path)?;
            }
            Ok(_) => {
                return Err(format!(
                    "Refusing to replace existing non-symlink path: {}",
                    target_path.display()
                )
                .into());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }

        // Create symlink
        linker::symlink_dir(&canonical_path, &target_path)?;

        if verbose {
            println!(
                "Linked {} to {} for agent {}",
                skill_name,
                canonical_path.display(),
                agent_name
            );
        }
    }

    println!(
        "Linked development skill '{}' from {}",
        skill_name,
        canonical_path.display()
    );
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

    let dev_skill = dev_config.skills.get(skill_name).unwrap().clone();

    // Confirm with user
    if !yes {
        print!(
            "Are you sure you want to unlink development skill '{}'? [y/N] ",
            skill_name
        );
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
    let project_root = std::env::current_dir()?;

    for agent_name in &dev_skill.agents {
        let base_dir = match linker::get_agent_skills_dir(agent_name, &project_root, global) {
            Ok(dir) => dir,
            Err(_) => continue,
        };

        let target_path = base_dir.join(linker::validated_skill_path(skill_name)?);

        if fs::symlink_metadata(&target_path).is_ok() {
            let metadata = fs::symlink_metadata(&target_path)?;
            if metadata.file_type().is_symlink() {
                fs::remove_file(&target_path)?;
                if verbose {
                    println!("Removed symlink: {}", target_path.display());
                }
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
    let mut all_skills: Vec<(String, DevSkill, bool)> = Vec::new(); // (name, skill, is_global)

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

    // Sort by name for deterministic output
    all_skills.sort_by(|a, b| a.0.cmp(&b.0));

    if json_output {
        let output: Vec<_> = all_skills
            .iter()
            .map(|(name, skill, is_global)| {
                serde_json::json!({
                    "name": name,
                    "path": skill.path,
                    "agents": skill.agents,
                    "global": is_global,
                    "overrides": skill.overrides
                })
            })
            .collect();
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

    let dev_skill = dev_config
        .skills
        .get(skill_name)
        .ok_or_else(|| format!("Development skill '{}' not found", skill_name))?;

    let project_root = std::env::current_dir()?;
    let canonical = fs::canonicalize(&dev_skill.path)?;

    if json_output {
        let output = serde_json::json!({
            "name": dev_skill.name,
            "path": dev_skill.path,
            "path_absolute": canonical,
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
        println!("Absolute: {}", canonical.display());
        println!("Global: {}", global);
        println!("Agents: {}", dev_skill.agents.join(", "));
        if let Some(ref override_source) = dev_skill.overrides {
            println!("Overrides: {}", override_source);
        }
        println!(
            "SKILL.md exists: {}",
            dev_skill.path.join("SKILL.md").exists()
        );

        println!("\nLinked to:");
        for agent_name in &dev_skill.agents {
            let base_dir = match linker::get_agent_skills_dir(agent_name, &project_root, global) {
                Ok(dir) => dir,
                Err(_) => continue,
            };

            let symlink_path = base_dir.join(linker::validated_skill_path(&dev_skill.name)?);
            if symlink_path.exists() || symlink_path.is_symlink() {
                println!("  {}: {}", agent_name, symlink_path.display());
            } else {
                println!("  {}: NOT LINKED", agent_name);
            }
        }
    }

    Ok(())
}

/// Toggle development mode
pub fn toggle_dev_mode(action: &str, global: bool) -> Result<(), Box<dyn std::error::Error>> {
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
            println!(
                "Development mode: {}",
                if dev_config.mode {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
        _ => {
            return Err(format!("Unknown action: '{}'. Use on, off, or status.", action).into());
        }
    }

    Ok(())
}

/// Check if a skill exists in any registry
fn skill_exists_in_registries(skill_name: &str) -> bool {
    find_skill_registry(skill_name).is_some()
}

/// Find the registry name for a skill
fn find_skill_registry(skill_name: &str) -> Option<String> {
    let base_config = BaseConfig::load().ok()?;

    for name in base_config.registries.keys() {
        let cache_path = linker::resolve_registry_path(name)?;
        if !cache_path.exists() {
            continue;
        }

        let skills_dir = cache_path.join("skills");
        if !skills_dir.exists() {
            continue;
        }

        if let Ok(validated_path) = linker::validated_skill_path(skill_name) {
            let skill_path = skills_dir.join(validated_path);
            if skill_path.exists() {
                return Some(name.clone());
            }
        }
    }

    None
}

/// Auto-discover and link local skills (when dev mode is enabled)
#[allow(dead_code)]
pub fn auto_discover_local_skills(global: bool) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let dev_config = DevConfig::load(global)?;

    if !dev_config.mode {
        return Ok(Vec::new());
    }

    // Look for .skm-local directory or other indicators
    // This is a placeholder for auto-discovery logic

    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_name() -> String {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("skm-dev-test-{}-{}", std::process::id(), unique)
    }

    fn setup_temp_project() -> (PathBuf, PathBuf) {
        let name = unique_name();
        let project_dir = std::env::temp_dir().join(&name);
        fs::create_dir_all(&project_dir).unwrap();

        let skill_dir = project_dir.join("test-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Test Skill\n").unwrap();

        (project_dir, skill_dir)
    }

    #[test]
    fn test_dev_mode_toggle() {
        let name = unique_name();
        let temp_dir = std::env::temp_dir().join(&name);
        fs::create_dir_all(&temp_dir).unwrap();

        // Run in temp dir
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        toggle_dev_mode("on", false).unwrap();
        let config = DevConfig::load(false).unwrap();
        assert!(config.mode);

        toggle_dev_mode("off", false).unwrap();
        let config = DevConfig::load(false).unwrap();
        assert!(!config.mode);

        std::env::set_current_dir(original_dir).unwrap();
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_link_unlink_local_skill() {
        let (project_dir, skill_dir) = setup_temp_project();

        // Mock HOME directory for testing
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_dir).unwrap();

        // Mock skills.yaml to use a specific set of agents
        let mock_config = SkillsConfig {
            name: "test-project".to_string(),
            version: Some("0.1.0".to_string()),
            registries: None,
            agents: vec!["claude".to_string(), "cursor".to_string()],
            skills: vec![],
        };
        mock_config
            .save_to_file(project_dir.join("skills.yaml"))
            .unwrap();

        link_local_skill(
            skill_dir.clone(),
            Some("test-skill".to_string()),
            None,
            false,
            false,
            None,
            false,
            false,
        )
        .unwrap();

        let config = DevConfig::load(false).unwrap();
        assert!(config.skills.contains_key("test-skill"));
        let dev_skill = config.skills.get("test-skill").unwrap();
        assert_eq!(dev_skill.name, "test-skill");
        assert_eq!(dev_skill.agents, vec!["claude", "cursor"]);

        // Check symlinks
        let claude_link = project_dir
            .join(".claude")
            .join("skills")
            .join("test-skill");
        let cursor_link = project_dir
            .join(".cursor")
            .join("skills")
            .join("test-skill");
        assert!(claude_link.is_symlink());
        assert!(cursor_link.is_symlink());

        // Unlink
        unlink_local_skill("test-skill", false, true, false).unwrap();
        let config = DevConfig::load(false).unwrap();
        assert!(!config.skills.contains_key("test-skill"));
        assert!(!claude_link.exists());
        assert!(!cursor_link.exists());

        std::env::set_current_dir(original_dir).unwrap();
        fs::remove_dir_all(&project_dir).unwrap();
    }
}
