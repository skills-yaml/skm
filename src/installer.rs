use crate::config::{SkillSpec, SkillsConfig};
use crate::linker;
use serde::{Deserialize, Serialize};
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    bundles: Vec<LockedBundle>,
    skills: Vec<LockedSkill>,
    outputs: Vec<LockedOutput>,
}

#[derive(Clone, Serialize, Deserialize)]
struct LockedBundle {
    name: String,
    version: String,
    source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resolved_version: Option<String>,
    members: Vec<LockedSkill>,
}

#[derive(Clone, Serialize, Deserialize)]
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

struct RemoveAction {
    path: PathBuf,
    previous: PathBuf,
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
    removals: Vec<RemoveAction>,
    lock: Option<(PathBuf, Option<Vec<u8>>, Vec<u8>)>,
    manifest: Option<(PathBuf, Vec<u8>, Vec<u8>)>,
    project: PathBuf,
    root: PathBuf,
}

fn locked_skill_spec(skill: &LockedSkill) -> SkillSpec {
    SkillSpec {
        name: skill.name.clone(),
        version: Some(skill.version.clone()),
        source: Some(skill.source.clone()),
        path: skill.path.clone(),
        kind: None,
    }
}

// Preserve the installed closure of every other entry when a single bundle is
// upgraded. The normal installer still owns validation, links and rollback.
fn resolve_named_bundle(
    config: &SkillsConfig,
    project: &Path,
    name: &str,
) -> Result<Vec<linker::ResolvedEntry>> {
    let bytes = read_regular_optional(&project.join("skills.lock.yaml"))?
        .ok_or("A named bundle upgrade requires an existing lock; run skm install first")?;
    let lock: serde_yaml::Value = serde_yaml::from_slice(&bytes)?;
    if lock.get("kind").and_then(|v| v.as_str()) != Some("skills") {
        return Err("skills.lock.yaml is not a skills-only lockfile".into());
    }
    let old_bundles: Vec<LockedBundle> = serde_yaml::from_value(
        lock.get("bundles")
            .cloned()
            .unwrap_or_else(|| serde_yaml::Value::Sequence(Vec::new())),
    )?;
    let old_skills: Vec<LockedSkill> = serde_yaml::from_value(
        lock.get("skills")
            .cloned()
            .ok_or("skills.lock.yaml has no skills")?,
    )?;
    let selected = config
        .skills
        .iter()
        .find(|s| s.is_bundle() && s.name == name)
        .ok_or("Selected bundle is missing")?;
    let old_selected = old_bundles
        .iter()
        .find(|b| b.name == name && Some(b.source.as_str()) == selected.source.as_deref())
        .ok_or("Selected bundle is not installed; run skm install first")?;
    let mut owned = std::collections::BTreeSet::new();
    let mut preserved_bundles = Vec::new();
    for bundle in config
        .skills
        .iter()
        .filter(|s| s.is_bundle() && s.name != name)
    {
        let old = old_bundles
            .iter()
            .find(|b| b.name == bundle.name && Some(b.source.as_str()) == bundle.source.as_deref())
            .ok_or("Another bundle is not installed; run skm install first")?;
        for member in &old.members {
            let matched = old_skills.iter().any(|skill| {
                skill.name == member.name
                    && skill.version == member.version
                    && skill.source == member.source
                    && skill.path == member.path
            });
            if !matched || member.source != old.source || member.path.is_some() {
                return Err("Invalid preserved bundle member in skills.lock.yaml".into());
            }
        }
        owned.extend(old.members.iter().map(|m| m.name.clone()));
        preserved_bundles.push(linker::ResolvedEntry::Bundle {
            spec: SkillSpec {
                name: old.name.clone(),
                version: Some(old.version.clone()),
                source: Some(old.source.clone()),
                path: None,
                kind: Some("bundle".into()),
            },
            members: old.members.iter().map(locked_skill_spec).collect(),
            resolved_version: old.resolved_version.clone(),
        });
    }
    for standalone in config.skills.iter().filter(|s| !s.is_bundle()) {
        let old = old_skills
            .iter()
            .find(|s| s.name == standalone.name)
            .ok_or("Another skill is not installed; run skm install first")?;
        let closure = linker::resolve_skill_dependency_closure(&[locked_skill_spec(old)], project)?;
        owned.extend(closure.into_iter().map(|s| s.name));
    }
    let selected_old: std::collections::BTreeSet<_> =
        old_selected.members.iter().map(|m| &m.name).collect();
    let mut requests = vec![selected.clone()];
    requests.extend(
        old_skills
            .iter()
            .filter(|s| !selected_old.contains(&s.name) || owned.contains(&s.name))
            .map(locked_skill_spec),
    );
    let mut entries = linker::resolve_entries(&requests, project)?;
    entries.extend(preserved_bundles);
    // The lock records bundles in manifest order, including preserved entries.
    entries.sort_by_key(|entry| match entry {
        linker::ResolvedEntry::Bundle { spec, .. } => config
            .skills
            .iter()
            .position(|skill| skill.is_bundle() && skill.name == spec.name)
            .unwrap_or(usize::MAX),
        linker::ResolvedEntry::Skill(_) => usize::MAX,
    });
    Ok(entries)
}

pub fn prepare(
    config: &SkillsConfig,
    project: &Path,
    global: bool,
    manifest: Option<(Vec<u8>, Vec<u8>)>,
) -> Result<Prepared> {
    prepare_inner(config, project, global, manifest, None)
}

pub fn prepare_named_bundle(config: &SkillsConfig, project: &Path, name: &str) -> Result<Prepared> {
    prepare_inner(config, project, false, None, Some(name))
}

fn prepare_inner(
    config: &SkillsConfig,
    project: &Path,
    global: bool,
    manifest: Option<(Vec<u8>, Vec<u8>)>,
    selected_bundle: Option<&str>,
) -> Result<Prepared> {
    linker::validate_agents(&config.agents)?;
    if global && config.skills.iter().any(|skill| skill.is_bundle()) {
        return Err("published bundles are project-only".into());
    }
    let entries = match selected_bundle {
        Some(name) => resolve_named_bundle(config, project, name)?,
        None => linker::resolve_entries(&config.skills, project)?,
    };
    let resolved = linker::linkable_entries(&entries);
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
    let mut removals = Vec::new();
    let mut outputs = Vec::new();
    let mut locked_skills = Vec::new();
    let mut locked_bundles = Vec::new();

    for entry in &entries {
        if let linker::ResolvedEntry::Bundle {
            spec,
            members,
            resolved_version,
        } = entry
        {
            locked_bundles.push(LockedBundle {
                name: spec.name.clone(),
                version: spec.version.clone().unwrap_or_else(|| "latest".into()),
                source: spec.source.clone().unwrap_or_else(|| "default".into()),
                resolved_version: resolved_version.clone(),
                members: members
                    .iter()
                    .map(|member| LockedSkill {
                        name: member.name.clone(),
                        version: member.version.clone().unwrap_or_else(|| "latest".into()),
                        source: member.source.clone().unwrap_or_else(|| "default".into()),
                        path: member.path.clone(),
                    })
                    .collect(),
            });
        }
    }

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
        // Only remove previously locked bundle members no longer required by
        // any current entry. Derive targets and require the old registry link.
        if let Some(bytes) = &existing {
            let previous: serde_yaml::Value = serde_yaml::from_slice(bytes)?;
            if let Some(old_bundles) = previous.get("bundles") {
                let old_bundles: Vec<LockedBundle> = serde_yaml::from_value(old_bundles.clone())?;
                let mut planned = std::collections::BTreeSet::new();
                for old in old_bundles {
                    if !locked_bundles
                        .iter()
                        .any(|current| current.name == old.name && current.source == old.source)
                    {
                        continue;
                    }
                    for member in old.members {
                        if resolved.iter().any(|skill| skill.name == member.name) {
                            continue;
                        }
                        if member.source != old.source {
                            return Err("Invalid source for bundle member in lockfile".into());
                        }
                        if member.path.is_some() {
                            return Err("Invalid local bundle member in lockfile".into());
                        }
                        linker::validate_exact_version(&member.version)?;
                        let registry = linker::effective_registry_path(&member.source, project)?;
                        let expected = registry
                            .join("skills")
                            .join(linker::validated_skill_path(&member.name)?)
                            .join(format!("v{}", member.version.trim_start_matches('v')));
                        for target in &targets {
                            let path = linker::get_skill_target_path(&target.path, &member.name)?;
                            if outputs
                                .iter()
                                .any(|output: &LockedOutput| root.join(&output.path) == path)
                                || !planned.insert(path.clone())
                            {
                                continue;
                            }
                            validate_parent(&root, &path)?;
                            let metadata = match fs::symlink_metadata(&path) {
                                Ok(metadata) => metadata,
                                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                                Err(error) => return Err(error.into()),
                            };
                            if !metadata.file_type().is_symlink() {
                                return Err(format!(
                                    "Refusing to remove real path: {}",
                                    path.display()
                                )
                                .into());
                            }
                            let previous = fs::read_link(&path)?;
                            let matches = if expected.exists() {
                                linker::symlink_points_to(&path, &expected)?
                            } else {
                                previous == expected
                            };
                            if !matches {
                                return Err(format!(
                                    "Refusing to remove unexpected bundle link: {}",
                                    path.display()
                                )
                                .into());
                            }
                            public.actions.push(PlanAction {
                                action: "remove",
                                path: path.display().to_string(),
                                target: None,
                            });
                            removals.push(RemoveAction { path, previous });
                        }
                    }
                }
            }
        }
        let mut desired = serde_yaml::to_value(SkillsLock {
            schema_version: 1,
            kind: "skills",
            project: config.name.clone(),
            agents: config.agents.clone(),
            bundles: locked_bundles,
            skills: locked_skills,
            outputs,
        })?;
        if let Some(bytes) = &existing {
            let previous: serde_yaml::Value = serde_yaml::from_slice(bytes)?;
            if let Some(metadata) = previous.as_mapping() {
                let fields = desired.as_mapping_mut().unwrap();
                for (key, value) in metadata {
                    if key.as_str() != Some("bundles") && !fields.contains_key(key) {
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
        removals,
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

    pub(crate) fn apply_with_failure(&self, fail_after_link: Option<usize>) -> Result<()> {
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
        let mut removed: Vec<&RemoveAction> = Vec::new();
        let mut created_directories = Vec::new();
        let mut manifest_moved = false;
        let mut lock_moved = false;
        let result = (|| -> Result<()> {
            for removal in &self.removals {
                validate_parent(&self.root, &removal.path)?;
                let metadata = fs::symlink_metadata(&removal.path)?;
                if !metadata.file_type().is_symlink()
                    || fs::read_link(&removal.path)? != removal.previous
                {
                    return Err(format!(
                        "Bundle link changed after planning: {}",
                        removal.path.display()
                    )
                    .into());
                }
                fs::remove_file(&removal.path)?;
                removed.push(removal);
            }
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
            for removal in removed.into_iter().rev() {
                if let Err(restore) = linker::symlink_dir(&removal.previous, &removal.path) {
                    rollback_errors.push(format!(
                        "could not restore link {}: {restore}",
                        removal.path.display()
                    ));
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
                    kind: None,
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

    #[test]
    #[serial_test::serial]
    fn bundle_member_replacement_rolls_back_removed_links_and_lock_on_failure() {
        struct HomeGuard(Option<std::ffi::OsString>, Option<std::ffi::OsString>);
        impl Drop for HomeGuard {
            fn drop(&mut self) {
                if let Some(value) = &self.0 {
                    std::env::set_var("HOME", value);
                } else {
                    std::env::remove_var("HOME");
                }
                if let Some(value) = &self.1 {
                    std::env::set_var("XDG_CACHE_HOME", value);
                } else {
                    std::env::remove_var("XDG_CACHE_HOME");
                }
            }
        }
        let home = tempfile::tempdir().unwrap();
        let _guard = HomeGuard(std::env::var_os("HOME"), std::env::var_os("XDG_CACHE_HOME"));
        std::env::set_var("HOME", home.path());
        std::env::set_var("XDG_CACHE_HOME", home.path().join(".cache"));
        let project = tempfile::tempdir().unwrap();
        let registry = home.path().join(".cache/skm/registries/local");
        for name in ["alpha", "beta"] {
            let path = registry.join(format!("skills/acme/{name}/v1.0.0"));
            fs::create_dir_all(&path).unwrap();
            fs::write(
                path.join("SKILL.md"),
                format!("---\nname: {name}\nmetadata:\n  skm-version: \"1.0.0\"\n---\n# {name}\n"),
            )
            .unwrap();
        }
        crate::config_manager::BaseConfig {
            default_registry: "local".into(),
            registries: std::collections::HashMap::from([(
                "local".into(),
                registry.to_string_lossy().into_owned(),
            )]),
            check_for_updates: false,
        }
        .save()
        .unwrap();
        let manifest = registry.join("skills/acme/manifest.yaml");
        fs::write(&manifest, "schema_version: 2\nnamespace: acme\npackages: {alpha: 1.0.0, beta: 1.0.0}\nbundles: {starter: {packages: [alpha]}}\n").unwrap();
        let config = SkillsConfig {
            name: "fixture".into(),
            version: None,
            registries: None,
            agents: vec!["codex".into()],
            skills: vec![SkillSpec {
                name: "acme/starter".into(),
                version: None,
                source: Some("local".into()),
                path: None,
                kind: Some("bundle".into()),
            }],
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            metadata: Default::default(),
        };
        let root = project.path();
        prepare(&config, root, false, None)
            .unwrap()
            .apply()
            .unwrap();
        let original = fs::read(root.join("skills.lock.yaml")).unwrap();
        let alpha = root.join(".agents/skills/alpha");
        let old_target = fs::read_link(&alpha).unwrap();
        fs::write(&manifest, "schema_version: 2\nnamespace: acme\npackages: {alpha: 1.0.0, beta: 1.0.0}\nbundles: {starter: {packages: [beta]}}\n").unwrap();
        let plan = prepare(&config, root, false, None).unwrap();
        assert!(plan.apply_with_failure(Some(1)).is_err());
        assert_eq!(fs::read_link(alpha).unwrap(), old_target);
        assert!(!root.join(".agents/skills/beta").exists());
        assert_eq!(fs::read(root.join("skills.lock.yaml")).unwrap(), original);
        prepare(&config, root, false, None)
            .unwrap()
            .apply()
            .unwrap();
        assert!(!root.join(".agents/skills/alpha").exists());
        assert!(root.join(".agents/skills/beta").is_symlink());
        verify_lock(&config, root).unwrap();
    }
}
