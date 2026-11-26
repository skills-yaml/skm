use crate::config::{SkillSpec, SkillsConfig};
use crate::linker;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
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

    let original = fs::read(&config_path)?;
    let mut config: SkillsConfig = serde_yaml::from_slice(&original)?;

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

    // The installed lock describes links even after registry membership drifts.
    let bundle_scope = !global && config.skills.iter().any(SkillSpec::is_bundle);
    let bundle_lock = if bundle_scope {
        read_install_lock(project_root)?
    } else {
        None
    };
    let (targets, removed_names) = if bundle_scope {
        if let Some(lock) = &bundle_lock {
            match plan_locked_removal(&config, &skill, lock, project_root, force, verbose)? {
                Some((targets, removed_names)) => (targets, Some(removed_names)),
                None => (Vec::new(), None),
            }
        } else {
            (
                plan_unlocked_removal(&config, &skill, project_root, false, force, verbose)?,
                None,
            )
        }
    } else if global {
        (
            plan_unlocked_removal(&config, &skill, project_root, true, force, verbose)?,
            None,
        )
    } else {
        (
            linker::plan_skill_unlink(
                &skill,
                project_root,
                &config.agents,
                global,
                force,
                verbose,
            )?,
            None,
        )
    };

    let snapshots: Vec<_> = targets
        .iter()
        .map(|target| {
            if target.kind == linker::UnlinkTargetKind::Symlink {
                fs::read_link(&target.path).map(Some)
            } else {
                Ok(None)
            }
        })
        .collect::<std::io::Result<_>>()?;

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

    if global {
        return remove_global_transaction(
            &config_path,
            &original,
            skill_name,
            &skill,
            project_root,
            &config,
            force,
            verbose,
            &targets,
            &snapshots,
            None,
        );
    }

    if bundle_scope {
        regular_manifest_metadata(&config_path)?;
    }
    if fs::read(&config_path)? != original {
        return Err("Manifest changed during removal; retry".into());
    }

    if bundle_scope {
        let current_lock = read_install_lock(project_root)?;
        if current_lock.as_ref().map(|lock| &lock.original)
            != bundle_lock.as_ref().map(|lock| &lock.original)
        {
            return Err("Lockfile changed during removal; retry".into());
        }
        let current_targets = if let Some(lock) = &bundle_lock {
            plan_locked_removal(&config, &skill, lock, project_root, force, verbose)?
                .map(|(targets, _)| targets)
                .unwrap_or_default()
        } else {
            plan_unlocked_removal(&config, &skill, project_root, false, force, verbose)?
        };
        if current_targets != targets {
            return Err("Targets changed during removal; retry".into());
        }
        for (target, snapshot) in targets.iter().zip(&snapshots) {
            crate::installer::validate_parent(project_root, &target.path)?;
            if let Some(snapshot) = snapshot {
                if fs::read_link(&target.path)? != *snapshot {
                    return Err("Target changed during removal; retry".into());
                }
            }
        }
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
        if let (Some(lock), Some(removed_names)) = (&bundle_lock, &removed_names) {
            update_installed_lock(project_root, lock, &skill, removed_names)?;
        }
    }

    Ok(())
}

fn regular_manifest_metadata(path: &Path) -> Result<fs::Metadata, Box<dyn std::error::Error>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("Manifest must remain a regular file during removal; retry".into());
    }
    Ok(metadata)
}

#[allow(clippy::too_many_arguments)]
fn remove_global_transaction(
    config_path: &Path,
    original: &[u8],
    name: &str,
    skill: &SkillSpec,
    root: &Path,
    config: &SkillsConfig,
    force: bool,
    verbose: bool,
    targets: &[linker::UnlinkTarget],
    snapshots: &[Option<std::path::PathBuf>],
    fail_after_move: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut document: serde_yaml::Value = serde_yaml::from_slice(original)?;
    let entries = document
        .get_mut("skills")
        .and_then(|v| v.as_sequence_mut())
        .ok_or("skills must be a YAML sequence")?;
    entries.retain(|entry| entry.get("name").and_then(|v| v.as_str()) != Some(name));
    let permissions = regular_manifest_metadata(config_path)?.permissions();
    let mut staged = tempfile::NamedTempFile::new_in(root)?;
    staged.as_file().set_permissions(permissions)?;
    staged.write_all(serde_yaml::to_string(&document)?.as_bytes())?;
    staged.as_file().sync_all()?;
    regular_manifest_metadata(config_path)?;
    if fs::read(config_path)? != original
        || plan_unlocked_removal(config, skill, root, true, force, verbose)? != targets
    {
        return Err("Manifest or targets changed during removal; retry".into());
    }
    for (target, snapshot) in targets.iter().zip(snapshots) {
        if let Some(snapshot) = snapshot {
            if fs::read_link(&target.path)? != *snapshot {
                return Err("Target changed during removal; retry".into());
            }
        }
    }
    let mut moved: Vec<(tempfile::TempDir, std::path::PathBuf)> = Vec::new();
    let result: Result<(), Box<dyn std::error::Error>> = (|| {
        for (target, snapshot) in targets.iter().zip(snapshots) {
            let home = dirs::home_dir().ok_or("Cannot resolve home")?;
            crate::installer::validate_parent(&home, &target.path)?;
            let metadata = fs::symlink_metadata(&target.path)?;
            let valid = match target.kind {
                linker::UnlinkTargetKind::Symlink => {
                    metadata.file_type().is_symlink()
                        && snapshot.as_ref().is_some_and(|expected| {
                            fs::read_link(&target.path).ok().as_ref() == Some(expected)
                        })
                }
                linker::UnlinkTargetKind::Directory => {
                    metadata.is_dir() && !metadata.file_type().is_symlink()
                }
                linker::UnlinkTargetKind::File => {
                    metadata.is_file() && !metadata.file_type().is_symlink()
                }
            };
            if !valid {
                return Err("Target changed during removal; retry".into());
            }
            let temporary = tempfile::Builder::new()
                .prefix(".skm-remove-")
                .tempdir_in(target.path.parent().ok_or("Invalid target")?)?;
            fs::rename(&target.path, temporary.path().join("entry"))?;
            moved.push((temporary, target.path.clone()));
            if fail_after_move == Some(moved.len()) {
                return Err("injected removal failure".into());
            }
        }
        regular_manifest_metadata(config_path)?;
        if fs::read(config_path)? != original {
            return Err("Manifest changed during removal; retry".into());
        }
        staged.persist(config_path)?;
        Ok(())
    })();
    if let Err(error) = result {
        let mut failures = Vec::new();
        for (temporary, path) in moved.into_iter().rev() {
            let restore_result = match fs::symlink_metadata(&path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    fs::rename(temporary.path().join("entry"), &path)
                }
                _ => Err(std::io::Error::other("target occupied during rollback")),
            };
            if let Err(restore) = restore_result {
                let retained = temporary.keep();
                failures.push(format!(
                    "{}: {restore}; retained at {}",
                    path.display(),
                    retained.display()
                ));
            }
        }
        if !failures.is_empty() {
            return Err(format!("{error}; rollback failed: {}", failures.join("; ")).into());
        }
        return Err(error);
    }
    eprintln!(
        "Removed skill '{}' from global manifest and {} target(s)",
        name,
        targets.len()
    );
    Ok(())
}

#[derive(Deserialize)]
struct InstallLock {
    kind: String,
    agents: Vec<String>,
    #[serde(skip)]
    original: Vec<u8>,
    #[serde(default)]
    bundles: Vec<LockedBundle>,
    skills: Vec<LockedSkill>,
}

#[derive(Deserialize)]
struct LockedBundle {
    name: String,
    source: String,
    members: Vec<LockedSkill>,
}

#[derive(Clone, Deserialize)]
struct LockedSkill {
    name: String,
    version: String,
    source: String,
    path: Option<String>,
}

impl LockedSkill {
    fn spec(&self) -> SkillSpec {
        SkillSpec {
            name: self.name.clone(),
            version: Some(self.version.clone()),
            source: Some(self.source.clone()),
            path: self.path.clone(),
            kind: None,
        }
    }
}

fn read_install_lock(project: &Path) -> Result<Option<InstallLock>, Box<dyn std::error::Error>> {
    let path = project.join("skills.lock.yaml");
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return Err("skills.lock.yaml must be a regular file".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let original = fs::read(path)?;
    let mut lock: InstallLock = serde_yaml::from_slice(&original)?;
    lock.original = original;
    if lock.kind != "skills" {
        return Err("skills.lock.yaml is not a skills-only lockfile".into());
    }
    Ok(Some(lock))
}

type LockedRemoval = (Vec<linker::UnlinkTarget>, BTreeSet<String>);

fn plan_locked_removal(
    config: &SkillsConfig,
    selected: &SkillSpec,
    lock: &InstallLock,
    project: &Path,
    force: bool,
    verbose: bool,
) -> Result<Option<LockedRemoval>, Box<dyn std::error::Error>> {
    let locked_skills: BTreeMap<_, _> = lock
        .skills
        .iter()
        .map(|skill| (skill.name.as_str(), skill))
        .collect();
    let get_bundle = |spec: &SkillSpec| {
        lock.bundles.iter().find(|bundle| {
            bundle.name == spec.name && Some(bundle.source.as_str()) == spec.source.as_deref()
        })
    };
    let candidates: Vec<SkillSpec> = if selected.is_bundle() {
        let Some(bundle) = get_bundle(selected) else {
            // Added since the last install: no links or lock entries belong to it.
            return Ok(None);
        };
        let mut result = Vec::new();
        for member in &bundle.members {
            if member.path.is_some() || member.source != bundle.source {
                return Err("Invalid bundle member in skills.lock.yaml".into());
            }
            linker::validate_exact_version(&member.version)?;
            result.push(member.spec());
        }
        result
    } else {
        let Some(locked) = locked_skills.get(selected.name.as_str()) else {
            return Ok(None);
        };
        vec![locked.spec()]
    };
    let mut retained = BTreeSet::new();
    for remaining in config
        .skills
        .iter()
        .filter(|skill| skill.name != selected.name)
    {
        if remaining.is_bundle() {
            if let Some(bundle) = get_bundle(remaining) {
                for member in &bundle.members {
                    retained.insert(member.name.clone());
                }
            }
        } else {
            if let Some(locked) = locked_skills.get(remaining.name.as_str()) {
                // Resolve the installed pin, not the currently published bundle or alias.
                let closure = linker::resolve_skill_dependency_closure(&[locked.spec()], project)?;
                retained.extend(closure.into_iter().map(|skill| skill.name));
            }
        }
    }
    let mut targets = Vec::new();
    let mut removed_names = BTreeSet::new();
    for candidate in candidates
        .iter()
        .filter(|skill| !retained.contains(&skill.name))
    {
        removed_names.insert(candidate.name.clone());
        targets.extend(linker::plan_locked_skill_unlink(
            candidate,
            project,
            &lock.agents,
            force,
            verbose,
        )?);
    }
    Ok(Some((targets, removed_names)))
}

fn update_installed_lock(
    project: &Path,
    lock: &InstallLock,
    selected: &SkillSpec,
    removed: &BTreeSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = project.join("skills.lock.yaml");
    if fs::read(&path)? != lock.original {
        return Err("skills.lock.yaml changed during removal; run skm install".into());
    }
    let mut value: serde_yaml::Value = serde_yaml::from_slice(&lock.original)?;
    if selected.is_bundle() {
        if let Some(bundles) = value.get_mut("bundles").and_then(|v| v.as_sequence_mut()) {
            bundles.retain(|bundle| {
                bundle.get("name").and_then(|v| v.as_str()) != Some(selected.name.as_str())
            });
            if bundles.is_empty() {
                value
                    .as_mapping_mut()
                    .ok_or("Invalid skills.lock.yaml")?
                    .shift_remove(serde_yaml::Value::String("bundles".into()));
            }
        }
    }
    for field in ["skills", "outputs"] {
        let entries = value
            .get_mut(field)
            .and_then(|v| v.as_sequence_mut())
            .ok_or("Invalid skills.lock.yaml entries")?;
        let key = if field == "skills" { "name" } else { "skill" };
        entries.retain(|entry| {
            entry
                .get(key)
                .and_then(|v| v.as_str())
                .is_none_or(|name| !removed.contains(name))
        });
    }
    let transaction = tempfile::Builder::new()
        .prefix(".skm-remove-")
        .tempdir_in(project)?;
    let staged = transaction.path().join("lock.new");
    fs::write(&staged, serde_yaml::to_string(&value)?)?;
    fs::set_permissions(&staged, fs::metadata(&path)?.permissions())?;
    if fs::read(&path)? != lock.original {
        return Err("skills.lock.yaml changed during removal; run skm install".into());
    }
    let backup = transaction.path().join("lock.old");
    fs::rename(&path, &backup)?;
    if let Err(error) = fs::rename(&staged, &path) {
        if let Err(restore) = fs::rename(&backup, &path) {
            let recovery = transaction.keep();
            return Err(format!(
                "Could not update lock: {error}; rollback failed: {restore}; backup retained at {}",
                recovery.display()
            )
            .into());
        }
        return Err(error.into());
    }
    Ok(())
}

fn plan_unlocked_removal(
    config: &SkillsConfig,
    selected: &SkillSpec,
    project: &Path,
    global: bool,
    force: bool,
    verbose: bool,
) -> Result<Vec<linker::UnlinkTarget>, Box<dyn std::error::Error>> {
    // With no installed targets at all, no declaration can own a link. Avoid
    // resolving sources (or requiring a cache) for the add-before-install state.
    if global && global_targets_are_empty(config, project)? {
        return Ok(Vec::new());
    }
    let before = linker::resolve_skill_dependency_closure(&config.skills, project)?;
    let remaining: Vec<_> = config
        .skills
        .iter()
        .filter(|skill| skill.name != selected.name)
        .cloned()
        .collect();
    let retained: BTreeSet<_> = linker::resolve_skill_dependency_closure(&remaining, project)?
        .into_iter()
        .map(|skill| skill.name)
        .collect();
    let candidates: Vec<_> = if global || selected.is_bundle() {
        before
            .iter()
            .filter(|skill| !retained.contains(&skill.name))
            .collect()
    } else {
        before
            .iter()
            .filter(|skill| skill.name == selected.name && !retained.contains(&skill.name))
            .collect()
    };
    let mut targets = Vec::new();
    for candidate in candidates {
        targets.extend(linker::plan_skill_unlink(
            candidate,
            project,
            &config.agents,
            global,
            force,
            verbose,
        )?);
    }
    Ok(targets)
}

fn global_targets_are_empty(
    config: &SkillsConfig,
    project: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    let home = dirs::home_dir().ok_or("Cannot resolve home")?;
    for target in linker::resolve_agent_skill_targets(&config.agents, project, true)? {
        // Validate all parent components, including the target directory itself.
        crate::installer::validate_parent(&home, &target.path.join("placeholder"))?;
        match fs::read_dir(&target.path) {
            Ok(mut entries) => {
                if entries.next().transpose()?.is_some() {
                    return Ok(false);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(true)
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
                kind: None,
            }],
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            metadata: Default::default(),
        };
        config.save_to_file(project.join("skills.yaml")).unwrap();
        linker::link_skill(&config.skills[0], &project, &config.agents, false).unwrap();
        (project, skill_dir)
    }

    #[test]
    #[cfg(unix)]
    #[serial_test::serial]
    fn global_removal_rolls_back_a_partial_failure() {
        let home = tempfile::tempdir().unwrap();
        let old_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home.path());
        let root = home.path().join(".skm");
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        fs::create_dir(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: test\ndescription: test\n---\n",
        )
        .unwrap();
        let original =
            b"name: user\nagents: [codex, cursor]\nskills:\n  - {name: test, path: source}\n";
        fs::write(root.join("skills.yaml"), original).unwrap();
        let config: SkillsConfig = serde_yaml::from_slice(original).unwrap();
        linker::link_skill(&config.skills[0], &root, &config.agents, true).unwrap();
        let targets =
            linker::plan_skill_unlink(&config.skills[0], &root, &config.agents, true, false, false)
                .unwrap();
        let snapshots: Vec<_> = targets
            .iter()
            .map(|target| Some(fs::read_link(&target.path).unwrap()))
            .collect();
        assert!(remove_global_transaction(
            &root.join("skills.yaml"),
            original,
            "test",
            &config.skills[0],
            &root,
            &config,
            false,
            false,
            &targets,
            &snapshots,
            Some(1)
        )
        .is_err());
        assert_eq!(fs::read(root.join("skills.yaml")).unwrap(), original);
        for (target, snapshot) in targets.iter().zip(snapshots) {
            assert_eq!(fs::read_link(&target.path).unwrap(), snapshot.unwrap());
        }
        match old_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }

    #[test]
    fn test_remove_skill_success() {
        let (project, _) = configured_project();
        let config_path = project.join("skills.yaml");
        let codex_target = project.join(".agents").join("skills").join("test-skill");
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
        let target = project.join(".agents").join("skills").join("test-skill");

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
        let codex_target = project.join(".agents/skills/test-skill");
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
        let codex_target = project.join(".agents/skills/test-skill");
        let cursor_target = project.join(".cursor/skills/test-skill");
        let sibling = project.join(".agents/skills/keep.txt");
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
        let codex_target = project.join(".agents/skills/test-skill");
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
