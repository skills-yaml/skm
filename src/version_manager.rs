use crate::config::SkillsConfig;
use crate::linker;
use std::cmp::Ordering;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Compare two version strings semantically
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    if a == "latest" {
        if b == "latest" {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    } else if b == "latest" {
        Ordering::Less
    } else {
        compare_semver_strings(a, b)
    }
}

fn split_pre(s: &str) -> (&str, Option<&str>) {
    if let Some(idx) = s.find('-') {
        (&s[..idx], Some(&s[idx + 1..]))
    } else {
        (s, None)
    }
}

/// Compare two semver-like strings
fn compare_semver_strings(a: &str, b: &str) -> Ordering {
    let a_clean = a.strip_prefix('v').unwrap_or(a);
    let b_clean = b.strip_prefix('v').unwrap_or(b);

    let (a_ver, a_pre) = split_pre(a_clean);
    let (b_ver, b_pre) = split_pre(b_clean);

    let a_parts: Vec<&str> = a_ver.split('.').collect();
    let b_parts: Vec<&str> = b_ver.split('.').collect();

    for i in 0..std::cmp::max(a_parts.len(), b_parts.len()) {
        let a_part = a_parts.get(i).copied().unwrap_or("0");
        let b_part = b_parts.get(i).copied().unwrap_or("0");

        let a_num = a_part.parse::<u64>().ok();
        let b_num = b_part.parse::<u64>().ok();

        match (a_num, b_num) {
            (Some(an), Some(bn)) => {
                if an != bn {
                    return an.cmp(&bn);
                }
            }
            _ => {
                if a_part != b_part {
                    return a_part.cmp(b_part);
                }
            }
        }
    }

    match (a_pre, b_pre) {
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(ap), Some(bp)) => ap.cmp(bp),
        (None, None) => Ordering::Equal,
    }
}

/// Check if a version is a prerelease
pub fn is_prerelease(version: &str) -> bool {
    version.contains("-alpha")
        || version.contains("-beta")
        || version.contains("-rc")
        || version.contains("-pre")
        || version.contains("+dev")
}

/// List all available versions for a skill from a registry
pub fn list_skill_versions(
    skill_name: &str,
    registry_name: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let registry_path = linker::resolve_registry_path(registry_name)
        .ok_or_else(|| format!("Could not resolve path for registry: {}", registry_name))?;

    let skill_path = registry_path
        .join("skills")
        .join(linker::validated_skill_path(skill_name)?);

    if !skill_path.exists() {
        return Err(format!(
            "Skill '{}' not found in registry '{}'",
            skill_name, registry_name
        )
        .into());
    }

    let mut versions = Vec::new();

    for entry in fs::read_dir(&skill_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with('v') || name == "latest" {
                    versions.push(name.to_string());
                }
            }
        }
    }

    // Sort versions descending (newest first)
    versions.sort_by(|a, b| compare_versions(b, a));

    Ok(versions)
}

/// List all versions available for a skill and print them
pub fn list_versions_cmd(
    skill_name: &str,
    registry_name: Option<&str>,
    stable_only: bool,
    include_pre: bool,
    limit: usize,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = registry_name.unwrap_or("default");
    let mut versions = list_skill_versions(skill_name, registry)?;

    if stable_only {
        versions.retain(|v| !is_prerelease(v));
    }

    if !include_pre {
        versions.retain(|v| !is_prerelease(v));
    }

    if versions.len() > limit {
        versions.truncate(limit);
    }

    if json_output {
        let output = serde_json::json!({
            "skill": skill_name,
            "registry": registry,
            "versions": versions,
            "latest": versions.first().map(|s| s.as_str()).unwrap_or("latest"),
            "count": versions.len()
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}:", skill_name);
        if versions.is_empty() {
            println!("  No versions found.");
        } else {
            for (idx, version) in versions.iter().enumerate() {
                if idx == 0 {
                    println!("  {} (latest)", version);
                } else {
                    println!("  {}", version);
                }
            }
            if let Some(latest) = versions.first() {
                println!(
                    "\n{} versions found. Use `skm use {}@{}` to switch.",
                    versions.len(),
                    skill_name,
                    latest
                );
            }
        }
    }

    Ok(())
}

/// Update a skill to a specific version
pub fn use_version(
    skill_name: &str,
    version: &str,
    config_path: &Path,
    global: bool,
    yes: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = SkillsConfig::load_from_file(config_path)?;

    // Find and update the skill
    let skill_index = config
        .skills
        .iter()
        .position(|s| s.name == skill_name)
        .ok_or_else(|| format!("Skill '{}' not found in configuration", skill_name))?;

    // Verify version exists in the registry
    let registry = config.skills[skill_index]
        .source
        .as_deref()
        .unwrap_or("default");
    let available_versions = list_skill_versions(skill_name, registry)?;
    if !available_versions.contains(&version.to_string()) && version != "latest" {
        return Err(format!(
            "Version '{}' not found for skill '{}' in registry '{}'. Available versions: {}",
            version,
            skill_name,
            registry,
            available_versions.join(", ")
        )
        .into());
    }

    let old_version = config.skills[skill_index]
        .version
        .as_deref()
        .unwrap_or("latest");

    if dry_run {
        println!(
            "Would update '{}' from {} to version '{}'",
            skill_name, old_version, version
        );
        let project_root = config_path.parent().unwrap_or_else(|| Path::new("."));
        for agent in &config.agents {
            let base = linker::get_agent_skills_dir(agent, project_root, global)?;
            let old_path = linker::get_skill_target_path(&base, skill_name)?;
            // Temporarily set version to resolve the new path
            let mut skill_temp = config.skills[skill_index].clone();
            skill_temp.version = Some(version.to_string());
            let new_path = linker::resolve_skill_source_dir(&skill_temp, project_root)?;
            println!(
                "Would relink for agent '{}': {} -> {}",
                agent,
                old_path.display(),
                new_path.display()
            );
        }
        return Ok(());
    }

    if !yes {
        print!(
            "Are you sure you want to switch skill '{}' from {} to version '{}'? [y/N] ",
            skill_name, old_version, version
        );
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    println!(
        "Updating '{}' from {} to {}",
        skill_name, old_version, version
    );

    // Save updated configuration
    config.skills[skill_index].version = Some(version.to_string());
    config.save_to_file(config_path)?;

    // Re-link the skill with new version
    let project_root = config_path.parent().unwrap_or_else(|| Path::new("."));
    linker::link_skill(
        &config.skills[skill_index],
        project_root,
        &config.agents,
        global,
    )?;

    println!("Done. Skill now using {}", version);

    Ok(())
}

/// Update a skill to its latest version
pub fn update_to_latest(
    skill_name: &str,
    config_path: &Path,
    global: bool,
    pre: bool,
    yes: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get current config
    let config = SkillsConfig::load_from_file(config_path)?;

    // Find the skill
    let skill = config
        .skills
        .iter()
        .find(|s| s.name == skill_name)
        .ok_or_else(|| format!("Skill '{}' not found in configuration", skill_name))?;

    let registry = skill.source.as_deref().unwrap_or("default");

    // List versions and find latest
    let mut versions = list_skill_versions(skill_name, registry)?;
    if !pre {
        versions.retain(|v| !is_prerelease(v));
    }

    let latest = versions
        .iter()
        .find(|v| *v != "latest")
        .or_else(|| versions.first())
        .ok_or_else(|| format!("No versions found for skill '{}'", skill_name))?;

    // Check if we are already at the latest version
    let current_version = skill.version.as_deref().unwrap_or("latest");
    if current_version == latest {
        println!(
            "Skill '{}' is already up to date (version: {}).",
            skill_name, current_version
        );
        return Ok(());
    }

    // Use the latest version
    use_version(skill_name, latest, config_path, global, yes, dry_run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SkillSpec;

    #[test]
    fn test_compare_semver_strings() {
        assert_eq!(compare_semver_strings("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_semver_strings("v1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_semver_strings("1.1.0", "1.0.0"), Ordering::Greater);
        assert_eq!(compare_semver_strings("1.0.0", "1.1.0"), Ordering::Less);
        assert_eq!(
            compare_semver_strings("1.0.0-alpha", "1.0.0"),
            Ordering::Less
        );
        assert_eq!(
            compare_semver_strings("1.0.0", "1.0.0-alpha"),
            Ordering::Greater
        );
        assert_eq!(
            compare_semver_strings("1.0.0-alpha", "1.0.0-beta"),
            Ordering::Less
        );
        assert_eq!(compare_semver_strings("1.2", "1.2.0"), Ordering::Equal);
    }

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions("latest", "v1.0.0"), Ordering::Greater);
        assert_eq!(compare_versions("v1.0.0", "latest"), Ordering::Less);
        assert_eq!(compare_versions("latest", "latest"), Ordering::Equal);
    }

    #[test]
    fn test_is_prerelease() {
        assert!(is_prerelease("1.0.0-alpha"));
        assert!(is_prerelease("v1.0.0-beta.1"));
        assert!(is_prerelease("2.0.0-rc1"));
        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("v2.1.3"));
    }

    fn unique_name() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("skm-version-test-{}-{}", std::process::id(), unique)
    }

    #[test]
    fn test_list_use_update_integration() {
        let unique = unique_name();
        let temp_dir = std::env::temp_dir().join(&unique);
        fs::create_dir_all(&temp_dir).unwrap();

        // Save original env vars
        let original_home = std::env::var("HOME").ok();
        let original_dir = std::env::current_dir().unwrap();

        // Set mock home to temp_dir
        std::env::set_var("HOME", &temp_dir);
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a mock registry structure
        let registry_path = temp_dir
            .join(".cache")
            .join("skm")
            .join("registries")
            .join("default");
        let skill_reg_path = registry_path.join("skills").join("test-skill");
        fs::create_dir_all(&skill_reg_path.join("v1.0.0")).unwrap();
        fs::create_dir_all(&skill_reg_path.join("v1.1.0")).unwrap();
        fs::create_dir_all(&skill_reg_path.join("latest")).unwrap();
        fs::write(skill_reg_path.join("v1.0.0").join("SKILL.md"), "# v1.0.0").unwrap();
        fs::write(skill_reg_path.join("v1.1.0").join("SKILL.md"), "# v1.1.0").unwrap();
        fs::write(skill_reg_path.join("latest").join("SKILL.md"), "# latest").unwrap();

        // 1. Test list_skill_versions
        let versions = list_skill_versions("test-skill", "default").unwrap();
        assert_eq!(versions, vec!["latest", "v1.1.0", "v1.0.0"]);

        // Create mock skills.yaml
        let config_path = temp_dir.join("skills.yaml");
        let mock_config = SkillsConfig {
            name: "test-proj".to_string(),
            version: Some("0.1.0".to_string()),
            registries: None,
            agents: vec!["claude".to_string()],
            skills: vec![SkillSpec {
                name: "test-skill".to_string(),
                version: Some("v1.0.0".to_string()),
                source: Some("default".to_string()),
                path: None,
            }],
        };
        mock_config.save_to_file(&config_path).unwrap();

        // Mock BaseConfig
        let config_dir = temp_dir.join(".config").join("skm");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(
            config_dir.join("config.yaml"),
            "default_registry: default\nregistries:\n  default: git@github.com:skills-yaml/skills-registry.git\n",
        )
        .unwrap();

        // 2. Test use_version
        use_version("test-skill", "v1.1.0", &config_path, false, true, false).unwrap();

        let updated_config = SkillsConfig::load_from_file(&config_path).unwrap();
        assert_eq!(updated_config.skills[0].version.as_deref(), Some("v1.1.0"));

        // Check symlinks
        let symlink_path = temp_dir.join(".claude").join("skills").join("test-skill");
        assert!(symlink_path.is_symlink());

        // 3. Test update_to_latest
        update_to_latest("test-skill", &config_path, false, false, true, false).unwrap();
        let latest_config = SkillsConfig::load_from_file(&config_path).unwrap();
        assert_eq!(latest_config.skills[0].version.as_deref(), Some("v1.1.0"));

        // Restore env vars
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
        std::env::set_current_dir(original_dir).unwrap();
        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
