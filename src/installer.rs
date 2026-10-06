use crate::config::SkillsConfig;
use crate::linker;
use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Serialize)]
struct SkillsLock {
    schema_version: u32,
    kind: &'static str,
    project: String,
    agents: Vec<String>,
    skills: Vec<LockedSkill>,
    outputs: Vec<LockedOutput>,
}

#[derive(Serialize)]
struct LockedSkill {
    name: String,
    version: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

#[derive(Serialize)]
struct LockedOutput {
    path: String,
    skill: String,
    agents: Vec<String>,
}

#[derive(Serialize)]
pub struct InstallPlan {
    scope: &'static str,
    actions: Vec<PlanAction>,
}

#[derive(Serialize)]
struct PlanAction {
    action: &'static str,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
}

struct LinkAction {
    path: PathBuf,
    source: PathBuf,
    canonical_source: PathBuf,
    previous: Option<PathBuf>,
}

pub struct Prepared {
    public: InstallPlan,
    links: Vec<LinkAction>,
    lock: Option<(PathBuf, Option<Vec<u8>>, Vec<u8>)>,
    manifest: Option<(PathBuf, Vec<u8>, Vec<u8>)>,
    project: PathBuf,
    root: PathBuf,
}

pub fn prepare(
    config: &SkillsConfig,
    project: &Path,
    global: bool,
    manifest: Option<(Vec<u8>, Vec<u8>)>,
) -> Result<Prepared> {
    linker::validate_agents(&config.agents)?;
    let resolved = linker::resolve_skill_dependency_closure(&config.skills, project)?;
    linker::validate_unique_skill_targets(&resolved)?;
    if !resolved.is_empty() {
        linker::require_skill_targets(&config.agents, project, global)?;
    }
    let root = if global {
        dirs::home_dir().ok_or("Could not determine home directory")?
    } else {
        project.to_path_buf()
    };
    let targets = linker::resolve_agent_skill_targets(&config.agents, project, global)?;
    let mut public = InstallPlan {
        scope: if global { "global" } else { "project" },
        actions: Vec::new(),
    };
    let mut links = Vec::new();
    let mut outputs = Vec::new();
    let mut locked_skills = Vec::new();

    for skill in &resolved {
        let source = linker::resolve_skill_source_dir(skill, project)?;
        if !source.is_dir() || !source.join("SKILL.md").is_file() {
            return Err(format!("Skill '{}' source or SKILL.md is missing", skill.name).into());
        }
        let canonical_source = fs::canonicalize(&source)?;
        locked_skills.push(LockedSkill {
            name: skill.name.clone(),
            version: skill.version.clone().unwrap_or_else(|| "latest".into()),
            source: skill.source.clone().unwrap_or_else(|| {
                if skill.path.is_some() {
                    "local"
                } else {
                    "default"
                }
                .into()
            }),
            path: skill.path.clone(),
        });
        for target in &targets {
            let path = linker::get_skill_target_path(&target.path, &skill.name)?;
            validate_parent(&root, &path)?;
            let relative = path.strip_prefix(&root)?;
            outputs.push(LockedOutput {
                path: relative.to_string_lossy().into_owned(),
                skill: skill.name.clone(),
                agents: target.agents.clone(),
            });
            let previous = match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_type().is_symlink() => Some(fs::read_link(&path)?),
                Ok(_) => {
                    return Err(format!("Refusing to replace real path: {}", path.display()).into())
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            if previous.is_some() && linker::symlink_points_to(&path, &source)? {
                continue;
            }
            public.actions.push(PlanAction {
                action: if previous.is_some() {
                    "replace"
                } else {
                    "create"
                },
                path: path.display().to_string(),
                target: Some(source.display().to_string()),
            });
            links.push(LinkAction {
                path,
                source: source.clone(),
                canonical_source: canonical_source.clone(),
                previous,
            });
        }
    }
    outputs.sort_by(|a, b| a.path.cmp(&b.path));
    let lock = if global {
        None
    } else {
        let lock_path = project.join("skills.lock.yaml");
        let existing = read_regular_optional(&lock_path)?;
        if let Some(bytes) = &existing {
            let value: serde_yaml::Value = serde_yaml::from_slice(bytes)?;
            if value.get("kind").and_then(serde_yaml::Value::as_str) != Some("skills") {
                return Err("skills.lock.yaml is not a skills-only lockfile".into());
            }
        }
        let mut desired = serde_yaml::to_value(SkillsLock {
            schema_version: 1,
            kind: "skills",
            project: config.name.clone(),
            agents: config.agents.clone(),
            skills: locked_skills,
            outputs,
        })?;
        if let Some(bytes) = &existing {
            let previous: serde_yaml::Value = serde_yaml::from_slice(bytes)?;
            if let Some(metadata) = previous.as_mapping() {
                let fields = desired.as_mapping_mut().unwrap();
                for (key, value) in metadata {
                    if !fields.contains_key(key) {
                        fields.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        let desired = serde_yaml::to_string(&desired)?.into_bytes();
        if existing.as_deref() != Some(desired.as_slice()) {
            public.actions.push(PlanAction {
                action: if existing.is_some() {
                    "replace"
                } else {
                    "create"
                },
                path: lock_path.display().to_string(),
                target: None,
            });
        }
        Some((lock_path, existing, desired))
    };
    if let Some((original, _)) = &manifest {
        if read_regular_optional(&project.join("skills.yaml"))?.as_deref()
            != Some(original.as_slice())
        {
            return Err("skills.yaml changed before planning; retry".into());
        }
    }
    let manifest = manifest.map(|(original, updated)| {
        let path = project.join("skills.yaml");
        if original != updated {
            public.actions.push(PlanAction {
                action: "replace",
                path: path.display().to_string(),
                target: None,
            });
        }
        (path, original, updated)
    });
    Ok(Prepared {
        public,
        links,
        lock,
        manifest,
        project: project.to_path_buf(),
        root,
    })
}

pub fn verify_lock(config: &SkillsConfig, project: &Path) -> Result<()> {
    let path = project.join("skills.lock.yaml");
    if read_regular_optional(&path)?.is_none() {
        return Ok(());
    }
    let prepared = prepare(config, project, false, None)?;
    if let Some((_, existing, desired)) = prepared.lock {
        if existing.as_deref() != Some(desired.as_slice()) {
            return Err("skills.lock.yaml is stale; run skm install".into());
        }
    }
    Ok(())
}

/// Keep an existing skills-only lock current after an independent manifest command.
pub fn sync_existing_lock(config_path: &Path, project: &Path) -> Result<()> {
    let lock_path = project.join("skills.lock.yaml");
    if read_regular_optional(&lock_path)?.is_none() {
        return Ok(());
    }
    let config = SkillsConfig::load_from_file(config_path)?;
    if config.toolkit.is_some() {
        return Ok(());
    }
    let prepared = prepare(&config, project, false, None)?;
    if !prepared.links.is_empty() {
        return Err("Skill links are incomplete; run skm install to refresh the lockfile".into());
    }
    prepared.apply()
}

fn read_regular_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            Ok(Some(fs::read(path)?))
        }
        Ok(_) => Err(format!("Expected a regular file: {}", path.display()).into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn validate_parent(root: &Path, path: &Path) -> Result<()> {
    let parent = path.parent().ok_or("Skill target has no parent")?;
    let relative = parent.strip_prefix(root)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(format!("Unsafe skill target parent: {}", current.display()).into())
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

impl Prepared {
    pub fn print(&self, json: bool) -> Result<()> {
        if json {
            println!("{}", serde_json::to_string_pretty(&self.public)?);
        } else if self.public.actions.is_empty() {
            println!("Skills are already installed; no changes needed.");
        } else {
            println!("Install plan ({}):", self.public.scope);
            for action in &self.public.actions {
                if let Some(target) = &action.target {
                    println!("  {} {} -> {}", action.action, action.path, target);
                } else {
                    println!("  {} {}", action.action, action.path);
                }
            }
        }
        Ok(())
    }

    pub fn apply(&self) -> Result<()> {
        self.apply_with_failure(None)
    }

    fn apply_with_failure(&self, fail_after_link: Option<usize>) -> Result<()> {
        if self.public.actions.is_empty() {
            return Ok(());
        }
        if let Some((path, original, _)) = &self.manifest {
            if read_regular_optional(path)?.as_deref() != Some(original.as_slice()) {
                return Err("skills.yaml changed after planning; retry".into());
            }
        }
        if let Some((path, previous, _)) = &self.lock {
            if read_regular_optional(path)?.as_ref() != previous.as_ref() {
                return Err("skills.lock.yaml changed after planning; retry".into());
            }
        }
        let transaction = tempfile::Builder::new()
            .prefix(".skm-install-")
            .tempdir_in(&self.project)?;
        let mut completed: Vec<&LinkAction> = Vec::new();
        let mut created_directories = Vec::new();
        let mut manifest_moved = false;
        let mut lock_moved = false;
        let result = (|| -> Result<()> {
            for (index, link) in self.links.iter().enumerate() {
                validate_parent(&self.root, &link.path)?;
                if fs::canonicalize(&link.source)? != link.canonical_source {
                    return Err(format!(
                        "Skill source changed after planning: {}",
                        link.source.display()
                    )
                    .into());
                }
                match (&link.previous, fs::symlink_metadata(&link.path)) {
                    (None, Err(error)) if error.kind() == io::ErrorKind::NotFound => {}
                    (Some(previous), Ok(metadata))
                        if metadata.file_type().is_symlink()
                            && fs::read_link(&link.path)? == *previous => {}
                    _ => {
                        return Err(format!(
                            "Skill target changed after planning: {}",
                            link.path.display()
                        )
                        .into())
                    }
                }
                ensure_parents(&link.path, &self.root, &mut created_directories)?;
                completed.push(link);
                if link.previous.is_some() {
                    fs::remove_file(&link.path)?;
                }
                linker::symlink_dir(&link.source, &link.path)?;
                if fail_after_link == Some(index + 1) {
                    return Err("injected install failure".into());
                }
            }
            if let Some((path, original, updated)) = &self.manifest {
                if original != updated {
                    if read_regular_optional(path)?.as_deref() != Some(original.as_slice()) {
                        return Err("skills.yaml changed during installation; retry".into());
                    }
                    let staged = transaction.path().join("skills.yaml.new");
                    fs::write(&staged, updated)?;
                    fs::set_permissions(&staged, fs::metadata(path)?.permissions())?;
                    fs::rename(path, transaction.path().join("skills.yaml.old"))?;
                    manifest_moved = true;
                    fs::rename(staged, path)?;
                }
            }
            if let Some((path, previous, desired)) = &self.lock {
                if previous.as_deref() != Some(desired.as_slice()) {
                    if read_regular_optional(path)?.as_ref() != previous.as_ref() {
                        return Err("skills.lock.yaml changed during installation; retry".into());
                    }
                    let staged = transaction.path().join("skills.lock.yaml.new");
                    fs::write(&staged, desired)?;
                    if previous.is_some() {
                        fs::set_permissions(&staged, fs::metadata(path)?.permissions())?;
                        fs::rename(path, transaction.path().join("skills.lock.yaml.old"))?;
                        lock_moved = true;
                    }
                    fs::rename(staged, path)?;
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            let mut rollback_errors = Vec::new();
            if let Some((path, _, _)) = &self.lock {
                if lock_moved {
                    if path.exists() && fs::remove_file(path).is_err() {
                        rollback_errors
                            .push(format!("could not remove new lockfile: {}", path.display()));
                    }
                    if let Err(restore) =
                        fs::rename(transaction.path().join("skills.lock.yaml.old"), path)
                    {
                        rollback_errors.push(format!("could not restore lockfile: {restore}"));
                    }
                }
            }
            if let Some((path, _, _)) = &self.manifest {
                if manifest_moved {
                    if path.exists() && fs::remove_file(path).is_err() {
                        rollback_errors
                            .push(format!("could not remove new manifest: {}", path.display()));
                    }
                    if let Err(restore) =
                        fs::rename(transaction.path().join("skills.yaml.old"), path)
                    {
                        rollback_errors.push(format!("could not restore manifest: {restore}"));
                    }
                }
            }
            for link in completed.into_iter().rev() {
                if link.path.is_symlink() {
                    if let Err(restore) = fs::remove_file(&link.path) {
                        rollback_errors.push(format!(
                            "could not remove link {}: {restore}",
                            link.path.display()
                        ));
                        continue;
                    }
                }
                if let Some(previous) = &link.previous {
                    if let Err(restore) = linker::symlink_dir(previous, &link.path) {
                        rollback_errors.push(format!(
                            "could not restore link {}: {restore}",
                            link.path.display()
                        ));
                    }
                }
            }
            for directory in created_directories.into_iter().rev() {
                let _ = fs::remove_dir(directory);
            }
            if rollback_errors.is_empty() {
                return Err(
                    format!("Installation failed and changes were restored: {error}").into(),
                );
            }
            let recovery = transaction.keep();
            return Err(format!(
                "Installation failed: {error}; rollback incomplete: {}; backups retained at {}",
                rollback_errors.join("; "),
                recovery.display()
            )
            .into());
        }
        Ok(())
    }
}

fn ensure_parents(path: &Path, root: &Path, created: &mut Vec<PathBuf>) -> Result<()> {
    let mut missing = Vec::new();
    let mut current = path.parent();
    while let Some(parent) = current {
        if parent == root || parent.exists() {
            break;
        }
        missing.push(parent.to_path_buf());
        current = parent.parent();
    }
    for parent in missing.into_iter().rev() {
        fs::create_dir(&parent)?;
        created.push(parent);
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::config::SkillSpec;
    use std::os::unix::fs::symlink;

    fn fixture() -> (tempfile::TempDir, SkillsConfig) {
        let root = tempfile::tempdir().unwrap();
        for name in ["alpha", "beta"] {
            let source = root.path().join(name);
            fs::create_dir(&source).unwrap();
            fs::write(
                source.join("SKILL.md"),
                format!("---\nname: {name}\ndescription: Fixture skill.\n---\n# {name}\n"),
            )
            .unwrap();
        }
        let config = SkillsConfig {
            name: "fixture".into(),
            version: None,
            registries: None,
            agents: vec!["codex".into(), "cursor".into()],
            skills: ["alpha", "beta"]
                .iter()
                .map(|name| SkillSpec {
                    name: (*name).into(),
                    version: Some("latest".into()),
                    source: None,
                    path: Some((*name).into()),
                })
                .collect(),
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            metadata: Default::default(),
        };
        (root, config)
    }

    #[test]
    fn preview_apply_and_repeat_are_deterministic() {
        let (root, config) = fixture();
        let project = root.path();
        let plan = prepare(&config, project, false, None).unwrap();
        assert_eq!(plan.public.actions.len(), 5);
        assert!(!project.join("skills.lock.yaml").exists());
        assert!(!project.join(".agents/skills/alpha").exists());
        plan.apply().unwrap();
        assert!(project.join(".agents/skills/alpha").is_symlink());
        assert!(project.join(".cursor/skills/beta").is_symlink());
        let lock = fs::read(project.join("skills.lock.yaml")).unwrap();
        verify_lock(&config, project).unwrap();
        let repeat = prepare(&config, project, false, None).unwrap();
        assert!(repeat.public.actions.is_empty());
        repeat.apply().unwrap();
        assert_eq!(fs::read(project.join("skills.lock.yaml")).unwrap(), lock);
    }

    #[test]
    fn reinstall_preserves_opaque_lock_metadata_and_regenerates_installation_fields() {
        let (root, config) = fixture();
        let project = root.path();
        prepare(&config, project, false, None)
            .unwrap()
            .apply()
            .unwrap();
        let lock_path = project.join("skills.lock.yaml");
        let mut lock: serde_yaml::Value =
            serde_yaml::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
        lock["workspace"] = serde_yaml::from_str("[opaque, domain]").unwrap();
        lock["publisher_policy"] = serde_yaml::from_str("active: true").unwrap();
        lock["project"] = "stale-project".into();
        fs::write(&lock_path, serde_yaml::to_string(&lock).unwrap()).unwrap();

        prepare(&config, project, false, None)
            .unwrap()
            .apply()
            .unwrap();
        let after = fs::read(&lock_path).unwrap();
        let actual: serde_yaml::Value = serde_yaml::from_slice(&after).unwrap();
        assert_eq!(actual["workspace"], lock["workspace"]);
        assert_eq!(actual["publisher_policy"], lock["publisher_policy"]);
        assert_eq!(actual["project"], "fixture");
        verify_lock(&config, project).unwrap();
        let repeat = prepare(&config, project, false, None).unwrap();
        assert!(repeat.public.actions.is_empty());
        repeat.apply().unwrap();
        assert_eq!(fs::read(lock_path).unwrap(), after);
    }

    #[test]
    fn collision_preflight_and_injected_failure_preserve_existing_state() {
        let (root, config) = fixture();
        let project = root.path();
        fs::create_dir_all(project.join(".cursor/skills")).unwrap();
        fs::write(project.join(".cursor/skills/beta"), "owned").unwrap();
        assert!(prepare(&config, project, false, None).is_err());
        assert!(!project.join(".agents/skills/alpha").exists());
        assert!(!project.join("skills.lock.yaml").exists());
        fs::remove_file(project.join(".cursor/skills/beta")).unwrap();

        let old = project.join("old");
        fs::create_dir(&old).unwrap();
        fs::create_dir_all(project.join(".agents/skills")).unwrap();
        symlink(&old, project.join(".agents/skills/alpha")).unwrap();
        let plan = prepare(&config, project, false, None).unwrap();
        assert!(plan.apply_with_failure(Some(1)).is_err());
        assert_eq!(
            fs::read_link(project.join(".agents/skills/alpha")).unwrap(),
            old
        );
        assert!(!project.join(".agents/skills/beta").exists());
        assert!(!project.join("skills.lock.yaml").exists());
    }

    #[test]
    fn rejects_symlinked_parent_and_manifest_changed_after_planning() {
        let (root, config) = fixture();
        let project = root.path();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), project.join(".agents")).unwrap();
        assert!(prepare(&config, project, false, None).is_err());
        assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
        fs::remove_file(project.join(".agents")).unwrap();

        let manifest_path = project.join("skills.yaml");
        let original = b"name: fixture\n".to_vec();
        fs::write(&manifest_path, &original).unwrap();
        let plan = prepare(
            &config,
            project,
            false,
            Some((original, b"name: changed\n".to_vec())),
        )
        .unwrap();
        fs::write(&manifest_path, "name: external\n").unwrap();
        assert!(plan.apply().is_err());
        assert_eq!(fs::read(&manifest_path).unwrap(), b"name: external\n");
        assert!(!project.join(".agents/skills/alpha").exists());
        assert!(!project.join("skills.lock.yaml").exists());
    }
}
