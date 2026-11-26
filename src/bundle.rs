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

// Root publisher metadata is opaque; installation fields retain typed parsing.
#[derive(Debug, Deserialize)]
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
    bundle_entry_added: bool,
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
    linker::validate_configured_skill_targets(&config.skills)?;

    let (root, registry_temp) = registry_root(project, &config, source, write_cache)?;
    let cache = linker::resolve_registry_path(source).ok_or("Invalid registry name")?;
    let compare_root = if !write_cache && cache.is_dir() {
        &cache
    } else {
        &root
    };
    let manifest = read_manifest(&root, namespace)?;
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
            kind: None,
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
    // Expand existing skills for conflict checking. Bundle entries are expanded
    // to their direct members, then the full dependency closure is resolved so
    // transitive dependencies (e.g. acme/helper) are recognized as existing.
    let mut existing_direct: Vec<SkillSpec> = Vec::new();
    for existing in &config.skills {
        if existing.is_bundle() {
            existing_direct.extend(linker::resolve_bundle_members_pub(existing, project)?);
        } else {
            existing_direct.push(existing.clone());
        }
    }
    for existing in &existing_direct {
        if let Some(member) = closure.iter().find(|member| member.name == existing.name) {
            let alias = matches!(
                existing.version.as_deref(),
                None | Some("latest") | Some("default")
            );
            if existing.path.is_some()
                || existing.source.as_deref().unwrap_or("default")
                    != member.source.as_deref().unwrap_or("default")
                || (!alias && !same_pin(existing, member))
            {
                return Err(format!(
                    "Existing skill '{}' has a different source or version",
                    existing.name
                )
                .into());
            }
        }
    }
    let existing_expanded =
        linker::resolve_skill_dependency_closure_with(&existing_direct, project, resolve)?;
    let mut combined = existing_expanded.clone();
    combined.extend(closure.clone());
    linker::validate_unique_skill_targets(&combined)?;
    let mut additions = Vec::new();
    let mut unchanged = Vec::new();
    for skill in &closure {
        match existing_expanded
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
                    let existing = existing_expanded
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
    // Check whether the bundle entry already exists as a kind: bundle entry.
    let bundle_entry_exists = if let Some(existing) = config
        .skills
        .iter()
        .find(|existing| existing.name == bundle)
    {
        if !existing.is_bundle() || existing.source.as_deref() != Some(source) {
            return Err(format!("Existing entry '{bundle}' has a different kind or source").into());
        }
        true
    } else {
        false
    };
    // If the bundle entry already exists, this is an idempotent re-add as long
    // as no member pins conflict.
    let bundle_addition = !bundle_entry_exists;
    let mut document: serde_yaml::Value = serde_yaml::from_slice(&original)?;
    if bundle_addition {
        let entries = document
            .as_mapping_mut()
            .ok_or("skills.yaml must be a YAML mapping")?
            .entry(serde_yaml::Value::String("skills".into()))
            .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));
        let entries = entries
            .as_sequence_mut()
            .ok_or("skills must be a YAML sequence")?;
        entries.push(serde_yaml::to_value(SkillSpec {
            name: bundle.to_string(),
            version: Some("latest".to_string()),
            source: Some(source.to_string()),
            path: None,
            kind: Some("bundle".to_string()),
        })?);
    }
    let updated = if bundle_addition {
        serde_yaml::to_string(&document)?.into_bytes()
    } else {
        original.clone()
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
        bundle_entry_added: bundle_addition,
        _registry_temp: registry_temp,
    })
}

fn read_manifest(root: &Path, namespace: &str) -> Result<NamespaceManifest> {
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
    Ok(manifest)
}

pub(crate) fn published_members(
    root: &Path,
    namespace: &str,
    id: &str,
    source: &str,
) -> Result<Vec<SkillSpec>> {
    let manifest = read_manifest(root, namespace)?;
    let bundle = manifest.bundles.get(id).ok_or_else(|| {
        format!("Bundle '{namespace}/{id}' is not published by registry '{source}'")
    })?;
    Ok(bundle
        .packages
        .iter()
        .map(|member| SkillSpec {
            name: format!("{namespace}/{member}"),
            version: Some(manifest.packages[member].clone()),
            source: Some(source.to_string()),
            path: None,
            kind: None,
        })
        .collect())
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
    }
    if let Some(version) = &manifest.toolkit_version {
        linker::validate_exact_version(version)?;
    }
    if let Some(minimum) = &manifest.minimum_skm_version {
        linker::validate_exact_version(minimum)?;
        if semver_parts(minimum)? > semver_parts(env!("CARGO_PKG_VERSION"))? {
            return Err(format!("Bundle requires SKM {minimum} or newer").into());
        }
    }
    if let Some(adapter) = &manifest.skm_adapter_compatibility {
        if adapter != "2.x" {
            return Err(format!("unsupported SKM adapter compatibility: {adapter}").into());
        }
    }
    if manifest
        .source_repository
        .as_ref()
        .is_some_and(|source| source.trim().is_empty())
    {
        return Err("source_repository cannot be empty".into());
    }
    if let Some(revision) = &manifest.source_revision {
        if revision.len() != 40
            || !revision
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(
                "source_revision must be a 40-character lowercase hexadecimal commit".into(),
            );
        }
    }
    Ok(())
}

fn semver_parts(version: &str) -> Result<(u64, u64, u64)> {
    let mut parts = version.strip_prefix('v').unwrap_or(version).split('.');
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
    if !prepared.bundle_entry_added
        && prepared.public.additions.is_empty()
        && prepared.links.is_empty()
    {
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

        fn publisher(namespace: &str, metadata: &str) -> Self {
            let fixture = Self::new();
            let directory = fixture.registry.path().join(format!("skills/{namespace}"));
            if namespace != "acme" {
                fs::rename(fixture.registry.path().join("skills/acme"), &directory).unwrap();
            }
            let skill_path = directory.join("alpha/v1.0.0/SKILL.md");
            let skill = fs::read_to_string(&skill_path).unwrap();
            fs::write(
                skill_path,
                skill.replace("acme/helper", &format!("{namespace}/helper")),
            )
            .unwrap();
            fs::write(
                directory.join("manifest.yaml"),
                format!(
                    "schema_version: 2\nnamespace: {namespace}\n{metadata}\npackages:\n  alpha: 1.0.0\n  helper: 2.0.0\nbundles:\n  starter:\n    packages: [alpha]\n  all-workspace-skills:\n    packages: [alpha]\n",
                ),
            )
            .unwrap();
            fixture
        }

        fn publisher_plan(&self, namespace: &str, bundle: &str) -> Result<Prepared> {
            prepare(
                self.project.path(),
                &format!("{namespace}/{bundle}"),
                "local",
                false,
            )
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
        // Bundle is written as a single kind: bundle entry, not unpacked.
        assert_eq!(config.skills.len(), 1);
        assert_eq!(config.skills[0].name, "acme/starter");
        assert_eq!(config.skills[0].kind.as_deref(), Some("bundle"));
        assert_eq!(config.skills[0].source.as_deref(), Some("local"));
        assert!(linker::symlink_points_to(
            &fixture.project.path().join(".agents/skills/alpha"),
            &fixture.registry.path().join("skills/acme/alpha/v1.0.0")
        )
        .unwrap());
        let second = fixture.plan().unwrap();
        assert!(second.public.additions.is_empty());
        assert!(second.links.is_empty());
        assert!(!second.bundle_entry_added);
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
        assert_eq!(config.skills.len(), 1);
        assert_eq!(config.skills[0].kind.as_deref(), Some("bundle"));
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
    fn publishers_install_subsets_and_metadata_under_identical_format_rules() {
        for namespace in ["workspace", "acme", "company"] {
            for bundle in ["starter", "all-workspace-skills"] {
                for metadata in [
                    "",
                    "workspace_docs_compatibility: 99.x",
                    "publisher_policy: {anything: [future, 99]}",
                    "minimum_skm_version: v0.4.0",
                    "workspace_docs_compatibility: {managed_by: skills}",
                    "toolkit_version: 0.3.0\nminimum_skm_version: 0.4.0\nskm_adapter_compatibility: 2.x\nsource_repository: https://example.com/publisher.git\nsource_revision: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                ] {
                    let fixture = Fixture::publisher(namespace, metadata);
                    let config_path = fixture.project.path().join("skills.yaml");
                    let before = fs::read(&config_path).unwrap();
                    let first = fixture.publisher_plan(namespace, bundle).unwrap();
                    assert_eq!(first.public.members, [format!("{namespace}/alpha")]);
                    assert_eq!(first.public.dependency_only, [format!("{namespace}/helper")]);
                    assert_eq!(fs::read(&config_path).unwrap(), before);
                    assert!(!fixture.project.path().join(".agents").exists());

                    apply(fixture.project.path(), &first).unwrap();
                    let after = fs::read(&config_path).unwrap();
                    let config = SkillsConfig::load_from_file(&config_path).unwrap();
                    // Bundle written as a single kind: bundle entry.
                    assert_eq!(config.skills.len(), 1);
                    assert_eq!(config.skills[0].name, format!("{namespace}/{bundle}"));
                    assert_eq!(config.skills[0].kind.as_deref(), Some("bundle"));
                    for (name, version) in [("alpha", "1.0.0"), ("helper", "2.0.0")] {
                        assert!(linker::symlink_points_to(
                            &fixture.project.path().join(format!(".agents/skills/{name}")),
                            &fixture.registry.path().join(format!("skills/{namespace}/{name}/v{version}")),
                        ).unwrap());
                    }
                    let second = fixture.publisher_plan(namespace, bundle).unwrap();
                    assert!(second.public.additions.is_empty());
                    assert!(second.links.is_empty());
                    assert!(!second.bundle_entry_added);
                    apply(fixture.project.path(), &second).unwrap();
                    assert_eq!(fs::read(config_path).unwrap(), after);
                }
            }
        }
    }

    #[test]
    fn declared_installation_constraints_apply_to_every_publisher_before_writes() {
        for namespace in ["workspace", "acme", "company"] {
            for (metadata, expected) in [
                ("minimum_skm_version: 99.0.0", "requires SKM 99.0.0"),
                ("minimum_skm_version: later", "exact semantic versioning"),
                (
                    "skm_adapter_compatibility: 99.x",
                    "unsupported SKM adapter compatibility",
                ),
                ("toolkit_version: later", "exact semantic versioning"),
                ("source_repository: ''", "source_repository cannot be empty"),
                (
                    "source_revision: invalid",
                    "source_revision must be a 40-character lowercase hexadecimal commit",
                ),
            ] {
                let fixture = Fixture::publisher(namespace, metadata);
                let config_path = fixture.project.path().join("skills.yaml");
                let before = fs::read(&config_path).unwrap();
                let error = fixture
                    .publisher_plan(namespace, "starter")
                    .unwrap_err()
                    .to_string();
                assert!(error.contains(expected), "{error}");
                assert_eq!(fs::read(config_path).unwrap(), before);
                assert!(!fixture.project.path().join(".agents").exists());
            }
            for (from, to, expected) in [
                (
                    "schema_version: 2",
                    "schema_version: 99",
                    "schema-2 manifest",
                ),
                (
                    "packages: [alpha]",
                    "packages: [alpha, alpha]",
                    "duplicate or unknown members",
                ),
                (
                    "packages: [alpha]",
                    "packages: [missing]",
                    "duplicate or unknown members",
                ),
                ("alpha: 1.0.0", "alpha: latest", "exact semantic versioning"),
            ] {
                let fixture = Fixture::publisher(namespace, "");
                let manifest_path = fixture
                    .registry
                    .path()
                    .join(format!("skills/{namespace}/manifest.yaml"));
                let manifest = fs::read_to_string(&manifest_path).unwrap();
                fs::write(manifest_path, manifest.replace(from, to)).unwrap();
                let config_path = fixture.project.path().join("skills.yaml");
                let before = fs::read(&config_path).unwrap();
                let error = fixture
                    .publisher_plan(namespace, "starter")
                    .unwrap_err()
                    .to_string();
                assert!(error.contains(expected), "{error}");
                assert_eq!(fs::read(config_path).unwrap(), before);
                assert!(!fixture.project.path().join(".agents").exists());
            }
        }
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
        // The manifest has a single kind: bundle entry; resolve it to members.
        let entries = linker::resolve_entries(&config.skills, fixture.project.path()).unwrap();
        for entry in &entries {
            for skill in entry.linkable_skills() {
                let expected =
                    linker::resolve_skill_source_dir(&skill, fixture.project.path()).unwrap();
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
        let entries = linker::resolve_entries(&config.skills, fixture.project.path()).unwrap();
        for entry in &entries {
            for skill in entry.linkable_skills() {
                let source =
                    linker::resolve_skill_source_dir(&skill, fixture.project.path()).unwrap();
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
        }
        assert!(fixture.plan().unwrap().links.is_empty());
    }
    #[cfg(unix)]
    #[test]
    #[serial]
    fn bundle_lifecycle_tracks_drift_and_preserves_shared_members() {
        let fixture = Fixture::new();
        let home = tempfile::tempdir().unwrap();
        let _guard = HomeGuard::set(home.path());
        let project = fixture.project.path();
        let config_path = project.join("skills.yaml");
        let prepared = prepare(project, "acme/starter", "local", true).unwrap();
        apply(project, &prepared).unwrap();
        let config = SkillsConfig::load_from_file(&config_path).unwrap();
        crate::installer::prepare(&config, project, false, None)
            .unwrap()
            .apply()
            .unwrap();
        crate::installer::verify_lock(&config, project).unwrap();
        let original = fs::read(&config_path).unwrap();
        let lock: serde_yaml::Value =
            serde_yaml::from_slice(&fs::read(project.join("skills.lock.yaml")).unwrap()).unwrap();
        assert_eq!(
            lock["bundles"][0]["members"].as_sequence().unwrap().len(),
            2
        );
        fixture.skill("beta", "1.0.0", None);
        fixture.manifest("schema_version: 2\nnamespace: acme\npackages:\n  alpha: 1.0.0\n  helper: 2.0.0\n  beta: 1.0.0\nbundles:\n  starter:\n    packages: [alpha, beta]\n");
        assert!(crate::installer::verify_lock(&config, project).is_err());
        let changes = crate::version_manager::bundle_changes(&config, project).unwrap();
        assert!(changes[0]
            .1
            .iter()
            .any(|s| s.contains("added acme/beta@1.0.0")));
        crate::version_manager::upgrade_all(&config_path, false, false, true, false).unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), original);
        assert!(project.join(".agents/skills/beta").is_symlink());
        crate::installer::verify_lock(&config, project).unwrap();
        fixture.skill("beta", "1.1.0", None);
        fixture.manifest("schema_version: 2\nnamespace: acme\npackages:\n  alpha: 1.0.0\n  helper: 2.0.0\n  beta: 1.1.0\nbundles:\n  starter:\n    packages: [alpha, beta]\n");
        assert!(crate::installer::verify_lock(&config, project).is_err());
        assert!(
            crate::version_manager::bundle_changes(&config, project).unwrap()[0]
                .1
                .iter()
                .any(|s| s.contains("acme/beta: 1.0.0 -> 1.1.0"))
        );
        crate::version_manager::upgrade_all(&config_path, false, false, true, false).unwrap();
        crate::installer::verify_lock(&config, project).unwrap();
        assert!(crate::installer::prepare(&config, project, true, None).is_err());
        let mut shared = config.clone();
        shared.skills.push(SkillSpec {
            name: "acme/helper".into(),
            version: Some("2.0.0".into()),
            source: Some("local".into()),
            path: None,
            kind: None,
        });
        shared.save_to_file(&config_path).unwrap();
        crate::remover::remove_skill("acme/starter", project, false, true, false, false, false)
            .unwrap();
        assert!(project.join(".agents/skills/helper").is_symlink());
        assert!(!project.join(".agents/skills/alpha").exists());
        assert!(!project.join(".agents/skills/beta").exists());
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn bundles_keep_independent_closures_and_reject_invalid_manifests() {
        let fixture = Fixture::new();
        let home = tempfile::tempdir().unwrap();
        let _guard = HomeGuard::set(home.path());
        fixture.skill("beta", "1.0.0", None);
        fixture.manifest("schema_version: 2\nnamespace: acme\npackages:\n  alpha: 1.0.0\n  helper: 2.0.0\n  beta: 1.0.0\nbundles:\n  starter:\n    packages: [alpha]\n  other:\n    packages: [beta]\n");
        fixture.write_config("skills:\n  - {name: acme/starter, kind: bundle, source: local}\n  - {name: acme/other, kind: bundle, source: local}\n  - {name: acme/helper, version: 2.0.0, source: local}\n");
        cached_local_registry(
            "local",
            &fs::canonicalize(fixture.registry.path()).unwrap(),
            fixture.registry.path().to_str().unwrap(),
        )
        .unwrap();
        let config =
            SkillsConfig::load_from_file(fixture.project.path().join("skills.yaml")).unwrap();
        let entries = linker::resolve_entries(&config.skills, fixture.project.path()).unwrap();
        let members = |index| match &entries[index] {
            linker::ResolvedEntry::Bundle { members, .. } => {
                members.iter().map(|s| s.name.as_str()).collect::<Vec<_>>()
            }
            _ => panic!("expected bundle"),
        };
        assert_eq!(members(0), ["acme/alpha", "acme/helper"]);
        assert_eq!(members(1), ["acme/beta"]);
        assert_eq!(linker::linkable_entries(&entries).len(), 3);
        let alpha = SkillSpec {
            name: "acme/alpha".into(),
            version: Some("1.0.0".into()),
            source: Some("local".into()),
            path: None,
            kind: None,
        };
        let skills_only =
            linker::resolve_skill_dependency_closure(&[alpha], fixture.project.path()).unwrap();
        assert_eq!(
            skills_only
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            ["acme/alpha", "acme/helper"]
        );
        fixture.skill("helper", "3.0.0", None);
        let mut conflicting = config.skills.clone();
        conflicting[2].version = Some("3.0.0".into());
        assert!(linker::resolve_entries(&conflicting, fixture.project.path()).is_err());
        let mut pinned = config.skills.clone();
        pinned[0].version = Some("1.0.0".into());
        assert!(linker::resolve_entries(&pinned, fixture.project.path())
            .unwrap_err()
            .to_string()
            .contains("no version history"));
        fixture.manifest("schema_version: 1\nnamespace: acme\npackages: {alpha: 1.0.0}\nbundles: {starter: {packages: [alpha]}}\n");
        assert!(
            linker::resolve_entries(&config.skills, fixture.project.path())
                .unwrap_err()
                .to_string()
                .contains("schema-2")
        );
        fixture.manifest("schema_version: 2\nnamespace: acme\npackages: {alpha: 1.0.0}\nbundles: {starter: {packages: [alpha, alpha]}}\n");
        assert!(
            linker::resolve_entries(&config.skills, fixture.project.path())
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );
    }
    #[cfg(unix)]
    #[test]
    #[serial]
    fn upgrade_removes_obsolete_links_and_rolls_back_on_failure() {
        let fixture = Fixture::new();
        let home = tempfile::tempdir().unwrap();
        let _guard = HomeGuard::set(home.path());
        let project = fixture.project.path();
        let config_path = project.join("skills.yaml");
        let prepared = prepare(project, "acme/starter", "local", true).unwrap();
        apply(project, &prepared).unwrap();
        let mut config = SkillsConfig::load_from_file(&config_path).unwrap();
        config.skills.push(SkillSpec {
            name: "acme/helper".into(),
            version: Some("2.0.0".into()),
            source: Some("local".into()),
            path: None,
            kind: None,
        });
        config.save_to_file(&config_path).unwrap();
        crate::installer::prepare(&config, project, false, None)
            .unwrap()
            .apply()
            .unwrap();
        let old_lock = fs::read(project.join("skills.lock.yaml")).unwrap();
        fixture.skill("beta", "1.0.0", None);
        fixture.manifest("schema_version: 2\nnamespace: acme\npackages:\n  alpha: 1.0.0\n  helper: 2.0.0\n  beta: 1.0.0\nbundles:\n  starter:\n    packages: [beta]\n");
        let plan = crate::installer::prepare(&config, project, false, None).unwrap();
        assert!(plan.apply_with_failure(Some(1)).is_err());
        assert!(project.join(".agents/skills/alpha").is_symlink());
        assert!(!project.join(".agents/skills/beta").exists());
        assert_eq!(
            fs::read(project.join("skills.lock.yaml")).unwrap(),
            old_lock
        );
        crate::version_manager::upgrade_all(&config_path, false, false, true, false).unwrap();
        assert!(!project.join(".agents/skills/alpha").exists());
        assert!(project.join(".agents/skills/beta").is_symlink());
        assert!(project.join(".agents/skills/helper").is_symlink());
        crate::installer::verify_lock(&config, project).unwrap();
    }
    #[cfg(unix)]
    #[test]
    #[serial]
    fn removal_uses_installed_members_after_registry_drift() {
        let fixture = Fixture::new();
        let home = tempfile::tempdir().unwrap();
        let _guard = HomeGuard::set(home.path());
        let project = fixture.project.path();
        let config_path = project.join("skills.yaml");
        let prepared = prepare(project, "acme/starter", "local", true).unwrap();
        apply(project, &prepared).unwrap();
        let config = SkillsConfig::load_from_file(&config_path).unwrap();
        crate::installer::prepare(&config, project, false, None)
            .unwrap()
            .apply()
            .unwrap();
        fixture.skill("alpha", "1.1.0", None);
        fixture.skill("beta", "1.0.0", None);
        fixture.manifest("schema_version: 2\nnamespace: acme\npackages: {alpha: 1.1.0, beta: 1.0.0}\nbundles: {starter: {packages: [beta]}}\n");
        fs::remove_dir_all(fixture.registry.path().join("skills/acme/alpha/v1.0.0")).unwrap();
        crate::remover::remove_skill("acme/starter", project, false, true, false, false, false)
            .unwrap();
        assert!(fs::symlink_metadata(project.join(".agents/skills/alpha")).is_err());
        assert!(fs::symlink_metadata(project.join(".agents/skills/helper")).is_err());
        let lock: serde_yaml::Value =
            serde_yaml::from_slice(&fs::read(project.join("skills.lock.yaml")).unwrap()).unwrap();
        assert!(lock.get("bundles").is_none());
        crate::check_skills(&config_path, project, false).unwrap();
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn removing_standalone_entry_preserves_bundle_membership() {
        let fixture = Fixture::new();
        let home = tempfile::tempdir().unwrap();
        let _guard = HomeGuard::set(home.path());
        let project = fixture.project.path();
        let config_path = project.join("skills.yaml");
        let prepared = prepare(project, "acme/starter", "local", true).unwrap();
        apply(project, &prepared).unwrap();
        let mut config = SkillsConfig::load_from_file(&config_path).unwrap();
        config.skills.push(SkillSpec {
            name: "acme/alpha".into(),
            version: Some("1.0.0".into()),
            source: Some("local".into()),
            path: None,
            kind: None,
        });
        config.save_to_file(&config_path).unwrap();
        crate::installer::prepare(&config, project, false, None)
            .unwrap()
            .apply()
            .unwrap();
        fixture.manifest("schema_version: 2\nnamespace: acme\npackages: {alpha: 1.0.0, helper: 2.0.0}\nbundles: {starter: {packages: [helper]}}\n");
        crate::remover::remove_skill("acme/alpha", project, false, true, false, false, false)
            .unwrap();
        assert!(project.join(".agents/skills/alpha").is_symlink());
        assert!(project.join(".agents/skills/helper").is_symlink());
        let lock: serde_yaml::Value =
            serde_yaml::from_slice(&fs::read(project.join("skills.lock.yaml")).unwrap()).unwrap();
        assert_eq!(
            lock["bundles"][0]["members"].as_sequence().unwrap().len(),
            2
        );
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn bundle_ids_do_not_claim_output_names() {
        let fixture = Fixture::new();
        let home = tempfile::tempdir().unwrap();
        let _guard = HomeGuard::set(home.path());
        let other = fixture.registry.path().join("skills/other");
        fs::create_dir_all(other.join("beta/v1.0.0")).unwrap();
        fs::write(
            other.join("beta/v1.0.0/SKILL.md"),
            "---\nname: beta\nmetadata: {skm-version: '1.0.0'}\n---\n# beta\n",
        )
        .unwrap();
        fs::write(other.join("manifest.yaml"), "schema_version: 2\nnamespace: other\npackages: {beta: 1.0.0}\nbundles: {starter: {packages: [beta]}}\n").unwrap();
        fixture.write_config("skills:\n  - {name: acme/starter, kind: bundle, source: local}\n  - {name: other/starter, kind: bundle, source: local}\n");
        cached_local_registry(
            "local",
            &fs::canonicalize(fixture.registry.path()).unwrap(),
            fixture.registry.path().to_str().unwrap(),
        )
        .unwrap();
        let config =
            SkillsConfig::load_from_file(fixture.project.path().join("skills.yaml")).unwrap();
        crate::validate_config(&config).unwrap();
        crate::installer::prepare(&config, fixture.project.path(), false, None)
            .unwrap()
            .apply()
            .unwrap();
        assert!(fixture
            .project
            .path()
            .join(".agents/skills/alpha")
            .is_symlink());
        assert!(fixture
            .project
            .path()
            .join(".agents/skills/beta")
            .is_symlink());
        fs::create_dir_all(other.join("alpha/v1.0.0")).unwrap();
        fs::write(
            other.join("alpha/v1.0.0/SKILL.md"),
            "---\nname: alpha\nmetadata: {skm-version: '1.0.0'}\n---\n# alpha\n",
        )
        .unwrap();
        fs::write(other.join("manifest.yaml"), "schema_version: 2\nnamespace: other\npackages: {alpha: 1.0.0}\nbundles: {starter: {packages: [alpha]}}\n").unwrap();
        assert!(crate::installer::prepare(&config, fixture.project.path(), false, None).is_err());
    }
    #[cfg(unix)]
    #[test]
    #[serial]
    fn removing_unversioned_alias_skill_preserves_bundle() {
        let fixture = Fixture::new();
        let home = tempfile::tempdir().unwrap();
        let _guard = HomeGuard::set(home.path());
        let project = fixture.project.path();
        let config_path = project.join("skills.yaml");
        fixture.skill("beta", "1.0.0", None);
        let beta = fixture.registry.path().join("skills/acme/beta");
        fs::write(
            beta.join("v1.0.0/SKILL.md"),
            "---\nname: beta\ndescription: Unversioned fixture.\n---\n# beta\n",
        )
        .unwrap();
        std::os::unix::fs::symlink("v1.0.0", beta.join("latest")).unwrap();
        std::os::unix::fs::symlink("v1.0.0", beta.join("default")).unwrap();
        let prepared = prepare(project, "acme/starter", "local", true).unwrap();
        apply(project, &prepared).unwrap();
        for alias in ["latest", "default"] {
            let mut config = SkillsConfig::load_from_file(&config_path).unwrap();
            config.skills.push(SkillSpec {
                name: "acme/beta".into(),
                version: Some(alias.into()),
                source: Some("local".into()),
                path: None,
                kind: None,
            });
            config.save_to_file(&config_path).unwrap();
            crate::installer::prepare(&config, project, false, None)
                .unwrap()
                .apply()
                .unwrap();
            crate::check_skills(&config_path, project, false).unwrap();
            crate::remover::remove_skill("acme/beta", project, false, true, false, false, false)
                .unwrap();
            assert!(!project.join(".agents/skills/beta").exists());
            assert!(project.join(".agents/skills/alpha").is_symlink());
            crate::check_skills(&config_path, project, false).unwrap();
        }
        let lock_path = project.join("skills.lock.yaml");
        let mut lock: serde_yaml::Value =
            serde_yaml::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
        lock["bundles"][0]["members"][0]["version"] = "latest".into();
        fs::write(&lock_path, serde_yaml::to_string(&lock).unwrap()).unwrap();
        assert!(crate::remover::remove_skill(
            "acme/starter",
            project,
            false,
            true,
            false,
            false,
            false
        )
        .is_err());
        assert!(project.join(".agents/skills/alpha").is_symlink());
    }
}
