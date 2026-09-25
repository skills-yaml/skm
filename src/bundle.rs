use crate::config::{SkillSpec, SkillsConfig};
use crate::{config_manager::BaseConfig, linker};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NamespaceManifest {
    schema_version: u32,
    namespace: String,
    #[serde(default)]
    toolkit_version: Option<String>,
    #[serde(default)]
    source_repository: Option<String>,
    #[serde(default)]
    source_revision: Option<String>,
    #[serde(default)]
    workspace_docs_compatibility: Option<String>,
    #[serde(default)]
    minimum_skm_version: Option<String>,
    #[serde(default)]
    skm_adapter_compatibility: Option<String>,
    packages: BTreeMap<String, String>,
    #[serde(default)]
    bundles: BTreeMap<String, BundleMembers>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleMembers {
    packages: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BundlePlan {
    schema_version: u32,
    bundle: String,
    source: String,
    toolkit_version: Option<String>,
    source_revision: Option<String>,
    members: Vec<String>,
    dependency_only: Vec<String>,
    additions: Vec<String>,
    unchanged: Vec<String>,
    pins: Vec<PlanPin>,
    links: Vec<String>,
    can_apply: bool,
}

#[derive(Debug, Serialize)]
struct PlanPin {
    name: String,
    version: String,
    source: String,
    requested_member: bool,
    action: &'static str,
}

#[derive(Debug)]
struct LinkAction {
    path: PathBuf,
    source: PathBuf,
    expected_canonical: PathBuf,
    previous: Option<PathBuf>,
}

#[derive(Debug)]
struct Prepared {
    public: BundlePlan,
    config_path: PathBuf,
    original: Vec<u8>,
    updated: Vec<u8>,
    links: Vec<LinkAction>,
    _registry_temp: Option<tempfile::TempDir>,
}

pub fn add(
    project: &Path,
    bundle: &str,
    source: &str,
    dry_run: bool,
    json: bool,
    yes: bool,
) -> Result<()> {
    add_with_confirmation(
        project,
        bundle,
        source,
        dry_run,
        json,
        yes,
        crate::confirmation::confirm,
    )
}

#[allow(clippy::too_many_arguments)]
fn add_with_confirmation<F>(
    project: &Path,
    bundle: &str,
    source: &str,
    dry_run: bool,
    json: bool,
    yes: bool,
    confirm: F,
) -> Result<()>
where
    F: FnOnce(&str) -> Result<bool>,
{
    let prepared = prepare(project, bundle, source, !dry_run && !json)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&prepared.public)?);
    } else {
        println!("Bundle: {}", prepared.public.bundle);
        println!("Registry: {source}");
        println!("Skills to install:");
        for pin in &prepared.public.pins {
            let role = if pin.requested_member {
                "bundle member"
            } else {
                "dependency"
            };
            println!("  {} {}@{} ({role})", pin.action, pin.name, pin.version);
        }
        println!("Agent links to create or repair: {}", prepared.links.len());
    }
    if !dry_run && !json {
        if !yes {
            io::stdout().flush()?;
            let question = format!(
                "Install {} skill{} and create or repair {} link{}?",
                prepared.public.pins.len(),
                if prepared.public.pins.len() == 1 {
                    ""
                } else {
                    "s"
                },
                prepared.links.len(),
                if prepared.links.len() == 1 { "" } else { "s" }
            );
            if !confirm(&question)? {
                eprintln!("Add cancelled.");
                return Ok(());
            }
        }
        apply(project, &prepared)?;
    }
    Ok(())
}

fn prepare(project: &Path, bundle: &str, source: &str, write_cache: bool) -> Result<Prepared> {
    linker::validate_skill_name(bundle)?;
    if bundle.split('/').count() != 2 {
        return Err("bundle ID must be namespace/name".into());
    }
    let (namespace, id) = bundle
        .split_once('/')
        .ok_or("bundle ID must be namespace/name")?;
    if linker::resolve_registry_path(source).is_none() {
        return Err(format!("Invalid registry name: {source}").into());
    }
    let config_path = project.join("skills.yaml");
    let metadata = fs::symlink_metadata(&config_path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("skills.yaml must be a regular file".into());
    }
    let original = fs::read(&config_path)?;
    let config: SkillsConfig = serde_yaml::from_slice(&original)?;
    linker::validate_agents(&config.agents)?;
    linker::require_skill_targets(&config.agents, project, false)?;
    linker::validate_unique_skill_targets(&config.skills)?;

    let (root, registry_temp) = registry_root(project, &config, source, write_cache)?;
    let cache = linker::resolve_registry_path(source).ok_or("Invalid registry name")?;
    let compare_root = if !write_cache && cache.is_dir() {
        &cache
    } else {
        &root
    };
    let manifest_path = root.join("skills").join(namespace).join("manifest.yaml");
    for path in [root.join("skills"), root.join("skills").join(namespace)] {
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "registry namespace must be a real directory: {}",
                path.display()
            )
            .into());
        }
    }
    let metadata = fs::symlink_metadata(&manifest_path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 256 * 1024 {
        return Err("bundle manifest must be a regular file of at most 256 KiB".into());
    }
    let manifest: NamespaceManifest = serde_yaml::from_slice(&fs::read(&manifest_path)?)?;
    validate_manifest(&manifest, namespace)?;
    let members = manifest
        .bundles
        .get(id)
        .ok_or_else(|| format!("Bundle '{bundle}' is not published by registry '{source}'"))?;
    let mut requested = Vec::new();
    for member in &members.packages {
        requested.push(SkillSpec {
            name: format!("{namespace}/{member}"),
            version: Some(manifest.packages[member].clone()),
            source: Some(source.to_string()),
            path: None,
        });
    }
    let resolve = |skill: &SkillSpec, project_root: &Path| {
        if skill.path.is_none() && skill.source.as_deref().unwrap_or("default") == source {
            linker::resolve_registry_skill_source_dir(skill, &root)
        } else {
            linker::resolve_skill_source_dir(skill, project_root)
        }
    };
    let closure = linker::resolve_skill_dependency_closure_with(&requested, project, resolve)?;
    let mut combined = config.skills.clone();
    combined.extend(closure.clone());
    linker::validate_unique_skill_targets(&combined)?;
    let mut additions = Vec::new();
    let mut unchanged = Vec::new();
    for skill in &closure {
        match config
            .skills
            .iter()
            .find(|existing| existing.name == skill.name)
        {
            Some(existing) if same_pin(existing, skill) => unchanged.push(skill.name.clone()),
            Some(_) => {
                return Err(format!(
                    "Existing skill '{}' has a different source or version",
                    skill.name
                )
                .into())
            }
            None => additions.push(skill.name.clone()),
        }
    }
    linker::resolve_skill_dependency_closure_with(&combined, project, resolve)?;
    let member_set: BTreeSet<_> = requested.iter().map(|skill| skill.name.clone()).collect();
    let dependency_only = closure
        .iter()
        .filter(|skill| !member_set.contains(&skill.name))
        .map(|skill| skill.name.clone())
        .collect();
    let pins = closure
        .iter()
        .map(|skill| PlanPin {
            name: skill.name.clone(),
            version: skill.version.clone().unwrap_or_default(),
            source: source.to_string(),
            requested_member: member_set.contains(&skill.name),
            action: if additions.contains(&skill.name) {
                "add"
            } else {
                "keep"
            },
        })
        .collect();
    let mut links = Vec::new();
    for target in linker::resolve_agent_skill_targets(&config.agents, project, false)? {
        validate_target_root(project, &target.path)?;
        for skill in &closure {
            linker::validate_skill_target_parent(&target.path, &skill.name)?;
            let path = linker::get_skill_target_path(&target.path, &skill.name)?;
            let desired = resolve(skill, project)?;
            let compare_source = compare_root.join(desired.strip_prefix(&root)?);
            let previous = match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    let current = fs::read_link(&path)?;
                    if compare_source.exists() && linker::symlink_points_to(&path, &compare_source)?
                    {
                        continue;
                    }
                    let existing = config
                        .skills
                        .iter()
                        .find(|item| item.name == skill.name)
                        .ok_or_else(|| {
                            format!("Refusing unmanaged symlink collision: {}", path.display())
                        })?;
                    let old_source = resolve(existing, project)?;
                    let old_compare = compare_root.join(old_source.strip_prefix(&root)?);
                    if !old_compare.exists() || !linker::symlink_points_to(&path, &old_compare)? {
                        return Err(
                            format!("Refusing unexpected symlink: {}", path.display()).into()
                        );
                    }
                    Some(current)
                }
                Ok(_) => {
                    return Err(
                        format!("Refusing non-symlink collision: {}", path.display()).into(),
                    )
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            links.push(LinkAction {
                path,
                expected_canonical: fs::canonicalize(&desired)?,
                source: desired,
                previous,
            });
        }
    }
    let mut document: serde_yaml::Value = serde_yaml::from_slice(&original)?;
    if !additions.is_empty() {
        let entries = document
            .as_mapping_mut()
            .ok_or("skills.yaml must be a YAML mapping")?
            .entry(serde_yaml::Value::String("skills".into()))
            .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));
        let entries = entries
            .as_sequence_mut()
            .ok_or("skills must be a YAML sequence")?;
        for skill in &closure {
            if additions.contains(&skill.name) {
                entries.push(serde_yaml::to_value(skill)?);
            }
        }
    }
    let updated = if additions.is_empty() {
        original.clone()
    } else {
        serde_yaml::to_string(&document)?.into_bytes()
    };
    let mut links_display = links
        .iter()
        .map(|link| {
            link.path
                .strip_prefix(project)
                .unwrap_or(&link.path)
                .display()
                .to_string()
        })
        .collect::<Vec<_>>();
    links_display.sort();
    Ok(Prepared {
        public: BundlePlan {
            schema_version: 1,
            bundle: bundle.into(),
            source: source.into(),
            toolkit_version: manifest.toolkit_version,
            source_revision: manifest.source_revision,
            members: member_set.into_iter().collect(),
            dependency_only,
            additions,
            unchanged,
            pins,
            links: links_display,
            can_apply: true,
        },
        config_path,
        original,
        updated,
        links,
        _registry_temp: registry_temp,
    })
}

fn validate_manifest(manifest: &NamespaceManifest, namespace: &str) -> Result<()> {
    if manifest.schema_version != 2 || manifest.namespace != namespace {
        return Err("bundle requires a schema-2 manifest matching its namespace".into());
    }
    if manifest.packages.is_empty() || manifest.bundles.is_empty() {
        return Err("bundle manifest must publish packages and bundles".into());
    }
    for (name, version) in &manifest.packages {
        valid_id(name)?;
        linker::validate_skill_name(&format!("{namespace}/{name}"))?;
        linker::validate_exact_version(version)?;
    }
    for (id, bundle) in &manifest.bundles {
        valid_id(id)?;
        if bundle.packages.is_empty() {
            return Err(format!("Bundle '{id}' is empty").into());
        }
        let members: BTreeSet<_> = bundle.packages.iter().collect();
        if members.len() != bundle.packages.len()
            || bundle
                .packages
                .iter()
                .any(|name| !manifest.packages.contains_key(name))
        {
            return Err(format!("Bundle '{id}' has duplicate or unknown members").into());
        }
        if namespace == "workspace"
            && id == "all-workspace-skills"
            && members != manifest.packages.keys().collect()
        {
            return Err("all-workspace-skills must contain every published package".into());
        }
    }
    if namespace == "workspace" {
        if !manifest.bundles.contains_key("all-workspace-skills") {
            return Err("Workspace manifest must publish all-workspace-skills".into());
        }
        for value in [
            &manifest.toolkit_version,
            &manifest.source_repository,
            &manifest.source_revision,
            &manifest.workspace_docs_compatibility,
            &manifest.minimum_skm_version,
            &manifest.skm_adapter_compatibility,
        ] {
            if value.as_deref().is_none_or(str::is_empty) {
                return Err(
                    "Workspace bundle manifest is missing provenance or compatibility metadata"
                        .into(),
                );
            }
        }
        linker::validate_exact_version(manifest.toolkit_version.as_deref().unwrap())?;
        let minimum = manifest.minimum_skm_version.as_deref().unwrap();
        linker::validate_exact_version(minimum)?;
        if semver_parts(minimum)? > semver_parts(env!("CARGO_PKG_VERSION"))? {
            return Err(format!("Workspace bundle requires SKM {minimum} or newer").into());
        }
        if manifest.source_repository.as_deref()
            != Some("https://github.com/skills-yaml/workspace.git")
            || !manifest
                .source_revision
                .as_deref()
                .unwrap()
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || manifest.source_revision.as_deref().unwrap().len() != 40
            || !matches!(
                manifest.workspace_docs_compatibility.as_deref(),
                Some("4.x" | "5.x" | "6.x")
            )
            || manifest.skm_adapter_compatibility.as_deref() != Some("2.x")
        {
            return Err("Workspace bundle provenance or compatibility is unsupported".into());
        }
    }
    Ok(())
}

fn semver_parts(version: &str) -> Result<(u64, u64, u64)> {
    let mut parts = version.split('.');
    Ok((
        parts.next().ok_or("missing major version")?.parse()?,
        parts.next().ok_or("missing minor version")?.parse()?,
        parts.next().ok_or("missing patch version")?.parse()?,
    ))
}

fn valid_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.starts_with('-')
        || id.ends_with('-')
        || id.contains("--")
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(format!("Invalid bundle or package ID: {id}").into());
    }
    Ok(())
}

fn validate_target_root(project: &Path, root: &Path) -> Result<()> {
    let relative = root.strip_prefix(project)?;
    let mut current = project.to_path_buf();
    for part in relative.components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(format!(
                    "Agent skill parent must be a real directory: {}",
                    current.display()
                )
                .into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn same_pin(left: &SkillSpec, right: &SkillSpec) -> bool {
    left.path.is_none()
        && right.path.is_none()
        && left.source.as_deref().unwrap_or("default")
            == right.source.as_deref().unwrap_or("default")
        && left
            .version
            .as_deref()
            .map(|value| value.trim_start_matches('v'))
            == right
                .version
                .as_deref()
                .map(|value| value.trim_start_matches('v'))
}

fn registry_root(
    project: &Path,
    config: &SkillsConfig,
    source: &str,
    write_cache: bool,
) -> Result<(PathBuf, Option<tempfile::TempDir>)> {
    let mut registries = BaseConfig::load()?.registries;
    if let Some(local) = &config.registries {
        registries.extend(local.clone());
    }
    let location = registries
        .get(source)
        .ok_or_else(|| format!("Registry '{source}' is not configured"))?;
    let local = project.join(location);
    if local.is_dir() {
        let metadata = fs::symlink_metadata(&local)?;
        if metadata.file_type().is_symlink() {
            return Err("Registry root must be a real directory".into());
        }
        let absolute = fs::canonicalize(local)?;
        let git_location = absolute
            .to_str()
            .ok_or("Registry path is not valid UTF-8")?;
        if !write_cache {
            if let Some(cache) = linker::resolve_registry_path(source) {
                if cache.exists() && !local_cache_matches(&cache, &absolute, git_location) {
                    return Err(format!("Cached registry '{source}' has a different origin").into());
                }
            }
            return Ok((absolute, None));
        }
        return cached_local_registry(source, &absolute, git_location);
    }
    if !crate::search::is_git_url(location) {
        return Err(format!(
            "Registry '{source}' is neither a local directory nor a supported Git URL"
        )
        .into());
    }
    if !write_cache {
        if let Some(cache) = linker::resolve_registry_path(source) {
            if cache.exists() && !crate::search::cache_matches(&cache, location) {
                return Err(format!("Cached registry '{source}' has a different origin").into());
            }
        }
        let temporary = tempfile::tempdir()?;
        let clone_path = temporary.path().join("registry");
        clone_registry(location, &clone_path, source)?;
        return Ok((clone_path, Some(temporary)));
    }
    cached_registry(source, location)
}

fn local_cache_matches(cache: &Path, local: &Path, location: &str) -> bool {
    (fs::symlink_metadata(cache).is_ok_and(|metadata| metadata.file_type().is_symlink())
        && fs::canonicalize(cache).is_ok_and(|target| target == local))
        || crate::search::cache_matches(cache, location)
}

fn cached_local_registry(
    source: &str,
    local: &Path,
    location: &str,
) -> Result<(PathBuf, Option<tempfile::TempDir>)> {
    let cache = linker::resolve_registry_path(source).ok_or("Invalid registry name")?;
    match fs::symlink_metadata(&cache) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            if fs::canonicalize(&cache)? != local {
                return Err(format!("Cached registry '{source}' has a different origin").into());
            }
            Ok((cache, None))
        }
        Ok(_) if crate::search::cache_matches(&cache, location) => {
            cached_registry(source, location)
        }
        Ok(_) => Err(format!("Cached registry '{source}' has a different origin").into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(cache.parent().ok_or("Invalid registry cache path")?)?;
            linker::symlink_dir(local, &cache)?;
            Ok((cache, None))
        }
        Err(error) => Err(error.into()),
    }
}

fn cached_registry(source: &str, location: &str) -> Result<(PathBuf, Option<tempfile::TempDir>)> {
    let cache = linker::resolve_registry_path(source).ok_or("Invalid registry name")?;
    if cache.exists() {
        if !crate::search::cache_matches(&cache, location) {
            return Err(format!(
                "Cached registry '{source}' has a different origin; refresh its cache explicitly"
            )
            .into());
        }
        let mut pull = Command::new("git");
        pull.args(["-c", "protocol.ext.allow=never"])
            .arg("-C")
            .arg(&cache)
            .args(["pull", "--ff-only"]);
        crate::search::run_git(pull, Duration::from_secs(30))
            .map_err(|error| format!("Could not refresh registry '{source}': {error}"))?;
        return Ok((cache, None));
    }
    let parent = cache.parent().ok_or("Invalid registry cache path")?;
    fs::create_dir_all(parent)?;
    let temporary = tempfile::tempdir_in(parent)?;
    let clone_path = temporary.path().join("registry");
    clone_registry(location, &clone_path, source)?;
    fs::rename(clone_path, &cache)?;
    Ok((cache, None))
}

fn clone_registry(location: &str, destination: &Path, source: &str) -> Result<()> {
    let mut clone = Command::new("git");
    clone
        .args([
            "-c",
            "protocol.ext.allow=never",
            "clone",
            "--depth",
            "1",
            "--",
            location,
        ])
        .arg(destination);
    crate::search::run_git(clone, Duration::from_secs(30))
        .map_err(|error| format!("Could not clone registry '{source}': {error}"))?;
    Ok(())
}

fn apply(project: &Path, prepared: &Prepared) -> Result<()> {
    apply_with_failure(project, prepared, None)
}

fn apply_with_failure(
    project: &Path,
    prepared: &Prepared,
    fail_after_link: Option<usize>,
) -> Result<()> {
    if prepared.public.additions.is_empty() && prepared.links.is_empty() {
        return Ok(());
    }
    if fs::read(&prepared.config_path)? != prepared.original {
        return Err("skills.yaml changed after bundle planning; retry".into());
    }
    let transaction = tempfile::Builder::new()
        .prefix(".skm-bundle-")
        .tempdir_in(project)?;
    let staged = transaction.path().join("skills.yaml.new");
    fs::write(&staged, &prepared.updated)?;
    fs::set_permissions(&staged, fs::metadata(&prepared.config_path)?.permissions())?;
    let mut moved = Vec::new();
    let mut created = Vec::new();
    let mut directories = Vec::new();
    let result = (|| -> Result<()> {
        for (index, link) in prepared.links.iter().enumerate() {
            validate_target_root(project, link.path.parent().ok_or("Link has no parent")?)?;
            if fs::canonicalize(&link.source)? != link.expected_canonical {
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
                        "Link target changed after planning: {}",
                        link.path.display()
                    )
                    .into())
                }
            }
            if link.previous.is_some() {
                let backup = transaction.path().join(format!("link-{index}"));
                fs::rename(&link.path, &backup)?;
                moved.push((link.path.clone(), backup));
            }
            ensure_parents(&link.path, project, &mut directories)?;
            linker::symlink_dir(&link.source, &link.path)?;
            created.push(link.path.clone());
            if fail_after_link == Some(index + 1) {
                return Err("injected bundle transaction failure".into());
            }
        }
        if fs::read(&prepared.config_path)? != prepared.original {
            return Err("skills.yaml changed during bundle application; retry".into());
        }
        if prepared.updated != prepared.original {
            let backup = transaction.path().join("skills.yaml.old");
            fs::rename(&prepared.config_path, &backup)?;
            moved.push((prepared.config_path.clone(), backup));
            fs::rename(&staged, &prepared.config_path)?;
            created.push(prepared.config_path.clone());
        }
        Ok(())
    })();
    if let Err(error) = result {
        let rollback = rollback(&created, &moved, &directories);
        return match rollback {
            Ok(()) => {
                Err(format!("Bundle application failed and rollback succeeded: {error}").into())
            }
            Err(rollback_error) => {
                let recovery = transaction.keep();
                Err(format!("Bundle application failed: {error}; rollback failed: {rollback_error}; backups retained at {}", recovery.display()).into())
            }
        };
    }
    Ok(())
}

fn rollback(
    created: &[PathBuf],
    moved: &[(PathBuf, PathBuf)],
    directories: &[PathBuf],
) -> Result<()> {
    for path in created.iter().rev() {
        fs::remove_file(path)?;
    }
    for (path, backup) in moved.iter().rev() {
        fs::rename(backup, path)?;
    }
    for path in directories.iter().rev() {
        match fs::remove_dir(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn ensure_parents(path: &Path, project: &Path, created: &mut Vec<PathBuf>) -> Result<()> {
    let mut missing = Vec::new();
    let mut current = path.parent();
    while let Some(directory) = current {
        if directory == project {
            break;
        }
        if directory.exists() {
            break;
        }
        missing.push(directory.to_path_buf());
        current = directory.parent();
    }
    for directory in missing.into_iter().rev() {
        fs::create_dir(&directory)?;
        created.push(directory);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use serial_test::serial;
    #[cfg(unix)]
    use std::env;

    #[cfg(unix)]
    struct HomeGuard(Option<std::ffi::OsString>);

    #[cfg(unix)]
    impl HomeGuard {
        fn set(path: &Path) -> Self {
            let prior = env::var_os("HOME");
            env::set_var("HOME", path);
            Self(prior)
        }
    }

    #[cfg(unix)]
    impl Drop for HomeGuard {
        fn drop(&mut self) {
            if let Some(prior) = &self.0 {
                env::set_var("HOME", prior);
            } else {
                env::remove_var("HOME");
            }
        }
    }

    struct Fixture {
        project: tempfile::TempDir,
        registry: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            let project = tempfile::tempdir().unwrap();
            let registry = tempfile::tempdir().unwrap();
            let fixture = Self { project, registry };
            fixture.write_config("skills: []\n");
            fixture.skill("alpha", "1.0.0", Some("acme/helper@2.0.0"));
            fixture.skill("helper", "2.0.0", None);
            fixture.manifest("schema_version: 2\nnamespace: acme\npackages:\n  alpha: 1.0.0\n  helper: 2.0.0\nbundles:\n  starter:\n    packages: [alpha]\n");
            fixture
        }

        fn write_config(&self, skills: &str) {
            let yaml = format!(
                "name: fixture\nagents: [codex]\nregistries:\n  local: {}\n{skills}",
                self.registry.path().display()
            );
            fs::write(self.project.path().join("skills.yaml"), yaml).unwrap();
        }

        fn skill(&self, name: &str, version: &str, dependencies: Option<&str>) {
            let path = self
                .registry
                .path()
                .join(format!("skills/acme/{name}/v{version}"));
            fs::create_dir_all(&path).unwrap();
            let dependency = dependencies
                .map(|value| format!("  skm-dependencies: \"{value}\"\n"))
                .unwrap_or_default();
            fs::write(path.join("SKILL.md"), format!("---\nname: {name}\nmetadata:\n  skm-version: \"{version}\"\n{dependency}---\n\n# {name}\n")).unwrap();
        }

        fn manifest(&self, content: &str) {
            fs::write(
                self.registry.path().join("skills/acme/manifest.yaml"),
                content,
            )
            .unwrap();
        }

        fn plan(&self) -> Result<Prepared> {
            prepare(self.project.path(), "acme/starter", "local", false)
        }
    }

    #[test]
    fn expands_members_and_dependencies_and_repeats_without_writes() {
        let fixture = Fixture::new();
        let before = fs::read(fixture.project.path().join("skills.yaml")).unwrap();
        let first = fixture.plan().unwrap();
        assert_eq!(first.public.members, ["acme/alpha"]);
        assert_eq!(first.public.dependency_only, ["acme/helper"]);
        assert_eq!(first.public.additions, ["acme/alpha", "acme/helper"]);
        assert_eq!(
            fs::read(fixture.project.path().join("skills.yaml")).unwrap(),
            before
        );
        assert!(!fixture.project.path().join(".agents/skills/alpha").exists());

        apply(fixture.project.path(), &first).unwrap();
        let after = fs::read(fixture.project.path().join("skills.yaml")).unwrap();
        let config =
            SkillsConfig::load_from_file(fixture.project.path().join("skills.yaml")).unwrap();
        assert_eq!(config.skills.len(), 2);
        assert!(config
            .skills
            .iter()
            .all(|skill| skill.source.as_deref() == Some("local")));
        assert!(linker::symlink_points_to(
            &fixture.project.path().join(".agents/skills/alpha"),
            &fixture.registry.path().join("skills/acme/alpha/v1.0.0")
        )
        .unwrap());
        let second = fixture.plan().unwrap();
        assert!(second.public.additions.is_empty());
        assert!(second.links.is_empty());
        apply(fixture.project.path(), &second).unwrap();
        assert_eq!(
            fs::read(fixture.project.path().join("skills.yaml")).unwrap(),
            after
        );
    }

    #[test]
    #[cfg(unix)]
    #[serial]
    fn confirmation_controls_bundle_application() {
        let fixture = Fixture::new();
        let home = tempfile::tempdir().unwrap();
        let _home = HomeGuard::set(home.path());
        let config_path = fixture.project.path().join("skills.yaml");
        let before = fs::read(&config_path).unwrap();

        add_with_confirmation(
            fixture.project.path(),
            "acme/starter",
            "local",
            false,
            false,
            false,
            |question| {
                assert!(question.contains("2 skills"));
                assert!(question.contains("2 links"));
                Ok(false)
            },
        )
        .unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), before);
        assert!(!fixture.project.path().join(".agents/skills/alpha").exists());

        add_with_confirmation(
            fixture.project.path(),
            "acme/starter",
            "local",
            false,
            false,
            false,
            |_| Ok(true),
        )
        .unwrap();
        let config = SkillsConfig::load_from_file(&config_path).unwrap();
        assert_eq!(config.skills.len(), 2);
        assert!(fixture
            .project
            .path()
            .join(".agents/skills/alpha")
            .is_symlink());
        assert!(fixture
            .project
            .path()
            .join(".agents/skills/helper")
            .is_symlink());

        add_with_confirmation(
            fixture.project.path(),
            "acme/starter",
            "local",
            false,
            false,
            true,
            |_| panic!("--yes must skip confirmation"),
        )
        .unwrap();
    }

    #[test]
    fn rejects_conflicts_and_non_symlink_targets_before_writes() {
        let fixture = Fixture::new();
        let target = fixture.project.path().join(".agents/skills/alpha");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "owned").unwrap();
        let before = fs::read(fixture.project.path().join("skills.yaml")).unwrap();
        assert!(fixture
            .plan()
            .unwrap_err()
            .to_string()
            .contains("non-symlink collision"));
        assert_eq!(fs::read(&target).unwrap(), b"owned");
        assert_eq!(
            fs::read(fixture.project.path().join("skills.yaml")).unwrap(),
            before
        );
        fs::remove_file(target).unwrap();
        fixture
            .write_config("skills:\n  - name: acme/alpha\n    version: 9.0.0\n    source: local\n");
        assert!(fixture
            .plan()
            .unwrap_err()
            .to_string()
            .contains("different source or version"));
    }

    #[test]
    fn rejects_invalid_manifest_and_detects_config_change() {
        let fixture = Fixture::new();
        fixture.manifest("schema_version: 2\nnamespace: acme\npackages:\n  alpha: 1.0.0\nbundles:\n  starter:\n    packages: [alpha, alpha]\n");
        assert!(fixture
            .plan()
            .unwrap_err()
            .to_string()
            .contains("duplicate or unknown"));
        fixture.manifest("schema_version: 2\nnamespace: acme\npackages:\n  alpha: 1.0.0\n  helper: 2.0.0\nbundles:\n  starter:\n    packages: [alpha]\n");
        let prepared = fixture.plan().unwrap();
        fs::write(
            fixture.project.path().join("skills.yaml"),
            "name: changed\n",
        )
        .unwrap();
        assert!(apply(fixture.project.path(), &prepared)
            .unwrap_err()
            .to_string()
            .contains("changed after bundle planning"));
        assert!(!fixture.project.path().join(".agents/skills/alpha").exists());
    }

    #[test]
    fn workspace_manifest_requires_complete_bundle_and_supported_release() {
        let raw = format!(
            "schema_version: 2\nnamespace: workspace\ntoolkit_version: 0.3.0\nsource_repository: https://github.com/skills-yaml/workspace.git\nsource_revision: {}\nworkspace_docs_compatibility: 5.x\nminimum_skm_version: 0.4.0\nskm_adapter_compatibility: 2.x\npackages:\n  alpha: 1.0.0\n  beta: 1.0.0\nbundles:\n  all-workspace-skills:\n    packages: [alpha, beta]\n",
            "a".repeat(40)
        );
        let manifest: NamespaceManifest = serde_yaml::from_str(&raw).unwrap();
        validate_manifest(&manifest, "workspace").unwrap();
        let incomplete = raw.replace("[alpha, beta]", "[alpha]");
        let manifest: NamespaceManifest = serde_yaml::from_str(&incomplete).unwrap();
        assert!(validate_manifest(&manifest, "workspace")
            .unwrap_err()
            .to_string()
            .contains("every published package"));
        let future = raw.replace("minimum_skm_version: 0.4.0", "minimum_skm_version: 99.0.0");
        let manifest: NamespaceManifest = serde_yaml::from_str(&future).unwrap();
        assert!(validate_manifest(&manifest, "workspace")
            .unwrap_err()
            .to_string()
            .contains("or newer"));
    }

    #[test]
    fn transaction_rolls_back_after_a_link_failure() {
        let fixture = Fixture::new();
        let before = fs::read(fixture.project.path().join("skills.yaml")).unwrap();
        let prepared = fixture.plan().unwrap();
        let error = apply_with_failure(fixture.project.path(), &prepared, Some(1))
            .unwrap_err()
            .to_string();
        assert!(error.contains("rollback succeeded"));
        assert_eq!(
            fs::read(fixture.project.path().join("skills.yaml")).unwrap(),
            before
        );
        assert!(!fixture.project.path().join(".agents/skills/alpha").exists());
        assert!(!fixture.project.path().join(".agents").exists());
    }

    #[test]
    fn keeps_matching_existing_pin_and_extension_fields() {
        let fixture = Fixture::new();
        fixture.write_config("extra: keep\nskills:\n  - name: acme/helper\n    version: 2.0.0\n    source: local\n    custom: preserve\n");
        let prepared = fixture.plan().unwrap();
        assert_eq!(prepared.public.additions, ["acme/alpha"]);
        assert_eq!(prepared.public.unchanged, ["acme/helper"]);
        apply(fixture.project.path(), &prepared).unwrap();
        let value: serde_yaml::Value =
            serde_yaml::from_slice(&fs::read(fixture.project.path().join("skills.yaml")).unwrap())
                .unwrap();
        assert_eq!(value["extra"].as_str(), Some("keep"));
        assert_eq!(value["skills"][0]["custom"].as_str(), Some("preserve"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_agent_parent_before_writes() {
        let fixture = Fixture::new();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), fixture.project.path().join(".agents")).unwrap();
        let error = fixture.plan().unwrap_err().to_string();
        assert!(error.contains("Agent skill parent must be a real directory"));
        assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn git_registry_preview_stays_read_only_and_apply_supports_normal_link_checks() {
        let fixture = Fixture::new();
        let home = tempfile::tempdir().unwrap();
        let _guard = HomeGuard::set(home.path());
        let commands: &[&[&str]] = &[
            &["init", "-q"],
            &["add", "."],
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
        ];
        for args in commands {
            assert!(Command::new("git")
                .args(*args)
                .current_dir(fixture.registry.path())
                .status()
                .unwrap()
                .success());
        }
        fs::write(
            fixture.project.path().join("skills.yaml"),
            format!(
                "name: fixture\nagents: [codex]\nregistries:\n  local: file://{}\nskills: []\n",
                fixture.registry.path().display()
            ),
        )
        .unwrap();
        let cache = linker::resolve_registry_path("local").unwrap();
        let preview = fixture.plan().unwrap();
        assert!(!cache.exists());
        drop(preview);

        let prepared = prepare(fixture.project.path(), "acme/starter", "local", true).unwrap();
        assert!(cache.is_dir());
        apply(fixture.project.path(), &prepared).unwrap();
        let config =
            SkillsConfig::load_from_file(fixture.project.path().join("skills.yaml")).unwrap();
        for skill in &config.skills {
            let expected = linker::resolve_skill_source_dir(skill, fixture.project.path()).unwrap();
            assert!(linker::symlink_points_to(
                &fixture
                    .project
                    .path()
                    .join(".agents/skills")
                    .join(skill.name.rsplit('/').next().unwrap()),
                &expected,
            )
            .unwrap());
        }
        let second = fixture.plan().unwrap();
        assert!(second.public.additions.is_empty());
        assert!(second.links.is_empty());
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn plain_local_registry_applies_and_resolves_through_cache() {
        let fixture = Fixture::new();
        let home = tempfile::tempdir().unwrap();
        let _guard = HomeGuard::set(home.path());
        let cache = linker::resolve_registry_path("local").unwrap();
        assert!(!cache.exists());
        let preview = fixture.plan().unwrap();
        assert!(!cache.exists());
        drop(preview);
        let prepared = prepare(fixture.project.path(), "acme/starter", "local", true).unwrap();
        assert!(fs::symlink_metadata(&cache)
            .unwrap()
            .file_type()
            .is_symlink());
        apply(fixture.project.path(), &prepared).unwrap();
        let config =
            SkillsConfig::load_from_file(fixture.project.path().join("skills.yaml")).unwrap();
        for skill in &config.skills {
            let source = linker::resolve_skill_source_dir(skill, fixture.project.path()).unwrap();
            assert!(linker::symlink_points_to(
                &fixture
                    .project
                    .path()
                    .join(".agents/skills")
                    .join(skill.name.rsplit('/').next().unwrap()),
                &source,
            )
            .unwrap());
        }
        assert!(fixture.plan().unwrap().links.is_empty());
    }
}
