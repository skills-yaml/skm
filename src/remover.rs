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
    remove_skill_with_confirmation(
        skill_name,
        project_root,
        global,
        yes,
        force,
        dry_run,
        verbose,
        prompt_for_removal,
    )
}

#[allow(clippy::too_many_arguments)]
fn remove_skill_with_confirmation<F>(
    skill_name: &str,
    project_root: &Path,
    global: bool,
    yes: bool,
    force: bool,
    dry_run: bool,
    verbose: bool,
    confirm: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&str) -> Result<bool, Box<dyn std::error::Error>>,
{
    linker::validate_skill_name(skill_name)?;
    let config_path = project_root.join("skills.yaml");

    if !config_path.exists() {
        return Err("skills.yaml file not found. Run 'skm init' to create one.".into());
    }

    let mut config = SkillsConfig::load_from_file(&config_path)?;

    let Some(skill) = config
        .skills
        .iter()
        .find(|skill| skill.name == skill_name)
        .cloned()
    else {
        if verbose {
            eprintln!("Skill '{}' not found in configuration", skill_name);
        }
        return Ok(()); // Idempotent - not an error
    };

    // Resolve and validate every target before confirmation or writes.
    let targets =
        linker::plan_skill_unlink(&skill, project_root, &config.agents, global, force, verbose)?;

    if dry_run {
        eprintln!("Would remove skill '{}' from skills.yaml", skill_name);
        for target in &targets {
            eprintln!("Would remove target: {}", target.path.display());
        }
        eprintln!(
            "Dry run complete. {} agent targets would be removed.",
            targets.len()
        );
        return Ok(());
    }

    // Confirm with user if not --yes
    if !yes && !confirm(skill_name)? {
        eprintln!("Removal cancelled.");
        return Ok(());
    }

    // Update configuration first, then attempt every preflighted target. If an
    // unlink fails, the configuration change remains and the partial outcome
    // is reported as required by the command contract.
    let removed = config.remove_skill(skill_name);

    if removed.is_some() {
        config.save_to_file(&config_path)?;
        eprintln!("Removed skill '{}' from configuration", skill_name);
        let unlink_result = linker::apply_skill_unlink(&targets);
        eprintln!(
            "Removed {} targets from agent directories:",
            unlink_result.removed.len()
        );
        for link in &unlink_result.removed {
            eprintln!("  - {}", link.display());
        }

        if !unlink_result.failures.is_empty() {
            eprintln!(
                "Failed to remove {} agent target(s):",
                unlink_result.failures.len()
            );
            for failure in &unlink_result.failures {
                eprintln!(
                    "  - {} (agent {}): {}",
                    failure.path.display(),
                    failure.agent,
                    failure.error
                );
            }
            return Err(format!(
                "Skill was removed from configuration, but {} agent target(s) could not be removed",
                unlink_result.failures.len()
            )
            .into());
        }
    }

    Ok(())
}

fn prompt_for_removal(skill_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    eprint!("Are you sure you want to remove '{}'? [y/N] ", skill_name);
    std::io::stderr().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SkillSpec;
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

    fn configured_project() -> (std::path::PathBuf, std::path::PathBuf) {
        let project = temp_project();
        let skill_dir = project.join("source").join("test-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Test skill\n").unwrap();
        let config = SkillsConfig {
            name: "test-proj".to_string(),
            version: Some("0.1.0".to_string()),
            registries: None,
            agents: vec!["codex".to_string(), "cursor".to_string()],
            skills: vec![SkillSpec {
                name: "test-skill".to_string(),
                version: Some("latest".to_string()),
                source: None,
                path: Some("source/test-skill".to_string()),
            }],
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            workspace: None,
            trusted_sources: Vec::new(),
        };
        config.save_to_file(project.join("skills.yaml")).unwrap();
        linker::link_skill(&config.skills[0], &project, &config.agents, false).unwrap();
        (project, skill_dir)
    }

    #[test]
    fn test_remove_skill_success() {
        let (project, _) = configured_project();
        let config_path = project.join("skills.yaml");
        let codex_target = project.join(".codex").join("skills").join("test-skill");
        let cursor_target = project.join(".cursor").join("skills").join("test-skill");
        assert!(codex_target.is_symlink());
        assert!(cursor_target.is_symlink());

        // Now run remove_skill (with yes=true)
        remove_skill("test-skill", &project, false, true, false, false, false).unwrap();

        // Check symlinks are gone
        assert!(!codex_target.exists() && !codex_target.is_symlink());
        assert!(!cursor_target.exists() && !cursor_target.is_symlink());

        // Check configuration is updated
        let updated_config = SkillsConfig::load_from_file(&config_path).unwrap();
        assert!(updated_config.skills.is_empty());

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn declining_confirmation_leaves_config_and_links_unchanged() {
        let (project, _) = configured_project();
        let config_path = project.join("skills.yaml");
        let target = project.join(".codex").join("skills").join("test-skill");

        remove_skill_with_confirmation(
            "test-skill",
            &project,
            false,
            false,
            false,
            false,
            false,
            |_| Ok(false),
        )
        .unwrap();

        let config = SkillsConfig::load_from_file(&config_path).unwrap();
        assert_eq!(config.skills.len(), 1);
        assert!(target.is_symlink());

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn dry_run_leaves_config_and_links_unchanged() {
        let (project, _) = configured_project();
        let config_path = project.join("skills.yaml");
        let target = project.join(".cursor").join("skills").join("test-skill");

        remove_skill("test-skill", &project, false, true, false, true, false).unwrap();

        let config = SkillsConfig::load_from_file(&config_path).unwrap();
        assert_eq!(config.skills.len(), 1);
        assert!(target.is_symlink());

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn removal_is_idempotent() {
        let (project, _) = configured_project();

        remove_skill("test-skill", &project, false, true, false, false, false).unwrap();
        remove_skill("test-skill", &project, false, true, false, false, false).unwrap();

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn unexpected_symlink_fails_before_config_or_link_changes() {
        let (project, _) = configured_project();
        let config_path = project.join("skills.yaml");
        let codex_target = project.join(".codex/skills/test-skill");
        let cursor_target = project.join(".cursor/skills/test-skill");
        let unexpected = project.join("unexpected");
        fs::create_dir_all(&unexpected).unwrap();
        fs::remove_file(&codex_target).unwrap();
        linker::symlink_dir(&unexpected, &codex_target).unwrap();

        let error =
            remove_skill("test-skill", &project, false, true, false, false, false).unwrap_err();

        assert!(error.to_string().contains("unexpected symlink"));
        assert_eq!(
            SkillsConfig::load_from_file(&config_path)
                .unwrap()
                .skills
                .len(),
            1
        );
        assert!(codex_target.is_symlink());
        assert!(cursor_target.is_symlink());

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn non_symlink_fails_without_force_and_is_removed_with_force() {
        let (project, _) = configured_project();
        let config_path = project.join("skills.yaml");
        let codex_target = project.join(".codex/skills/test-skill");
        let cursor_target = project.join(".cursor/skills/test-skill");
        let sibling = project.join(".codex/skills/keep.txt");
        fs::remove_file(&codex_target).unwrap();
        fs::create_dir_all(&codex_target).unwrap();
        fs::write(codex_target.join("content.txt"), "content").unwrap();
        fs::write(&sibling, "keep").unwrap();

        let error =
            remove_skill("test-skill", &project, false, true, false, false, false).unwrap_err();
        assert!(error.to_string().contains("non-symlink"));
        assert_eq!(
            SkillsConfig::load_from_file(&config_path)
                .unwrap()
                .skills
                .len(),
            1
        );
        assert!(codex_target.join("content.txt").exists());
        assert!(cursor_target.is_symlink());

        remove_skill("test-skill", &project, false, true, true, false, false).unwrap();

        assert!(!codex_target.exists());
        assert!(!cursor_target.is_symlink());
        assert_eq!(fs::read_to_string(sibling).unwrap(), "keep");
        assert!(SkillsConfig::load_from_file(&config_path)
            .unwrap()
            .skills
            .is_empty());

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn unlink_failure_is_reported_after_later_targets_are_attempted() {
        let (project, _) = configured_project();
        let config_path = project.join("skills.yaml");
        let codex_target = project.join(".codex/skills/test-skill");
        let cursor_target = project.join(".cursor/skills/test-skill");
        let target_to_change = codex_target.clone();

        let error = remove_skill_with_confirmation(
            "test-skill",
            &project,
            false,
            false,
            false,
            false,
            false,
            move |_| {
                fs::remove_file(&target_to_change)?;
                fs::create_dir_all(&target_to_change)?;
                Ok(true)
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("1 agent target"));
        assert!(codex_target.is_dir());
        assert!(!cursor_target.exists() && !cursor_target.is_symlink());
        assert!(SkillsConfig::load_from_file(&config_path)
            .unwrap()
            .skills
            .is_empty());

        fs::remove_dir_all(project).unwrap();
    }
}
