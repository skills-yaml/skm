use crate::config::SkillsConfig;
use crate::linker;
use std::io::Write;
use std::path::Path;

/// Remove a skill from configuration and unlink from agents
pub fn remove_skill(
    skill_name: &str,
    project_root: &Path,
    global: bool,
    yes: bool,
    force: bool,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = project_root.join("skills.yaml");

    if !config_path.exists() {
        return Err("skills.yaml file not found. Run 'skm init' to create one.".into());
    }

    let mut config = SkillsConfig::load_from_file(&config_path)?;

    // Check if skill exists
    if !config.skills.iter().any(|s| s.name == skill_name) {
        if verbose {
            eprintln!("Skill '{}' not found in configuration", skill_name);
        }
        return Ok(()); // Idempotent - not an error
    }

    if dry_run || verbose {
        eprintln!("Removing skill '{}' from configuration", skill_name);
    }

    // Unlink from agents
    let removed_links = linker::unlink_skill(
        skill_name,
        project_root,
        &config.agents,
        global,
        force,
        dry_run,
        verbose,
    )?;

    if dry_run {
        eprintln!(
            "Dry run complete. {} symlinks would be removed.",
            removed_links.len()
        );
        return Ok(());
    }

    // Confirm with user if not --yes
    if !yes {
        eprint!("Are you sure you want to remove '{}'? [y/N] ", skill_name);
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            eprintln!("Removal cancelled.");
            return Ok(());
        }
    }

    // Remove from configuration
    let removed = config.remove_skill(skill_name);

    if removed.is_some() {
        config.save_to_file(&config_path)?;
        eprintln!("Removed skill '{}' from configuration", skill_name);
        eprintln!(
            "Removed {} symlinks from agent directories:",
            removed_links.len()
        );
        for link in &removed_links {
            eprintln!("  - {}", link.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "skm-test-remover-{}-{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_remove_skill_success() {
        let project = temp_project();
        let config = SkillsConfig::default_init("test-proj");
        let config_path = project.join("skills.yaml");
        config.save_to_file(&config_path).unwrap();

        // Create the skill dir and link it
        let skill_dir = project
            .join("source")
            .join("software-development")
            .join("spec");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Test skill\n").unwrap();

        // Run install
        linker::link_skill(&config.skills[0], &project, &config.agents, false).unwrap();
        let target = project
            .join(".codex")
            .join("skills")
            .join("software-development")
            .join("spec");
        assert!(target.exists() || target.is_symlink());

        // Now run remove_skill (with yes=true)
        remove_skill(
            "software-development/spec",
            &project,
            false,
            true,
            false,
            false,
            false,
        )
        .unwrap();

        // Check symlinks are gone
        assert!(!target.exists() && !target.is_symlink());

        // Check configuration is updated
        let updated_config = SkillsConfig::load_from_file(&config_path).unwrap();
        assert!(updated_config.skills.is_empty());

        fs::remove_dir_all(project).unwrap();
    }
}
