use crate::config::SkillsConfig;
use crate::linker;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

const LOCKFILE_NAME: &str = "skills.lock.yaml";
const TRANSACTION_PATH: &str = ".skm/transactions/current";
const ADAPTER_VERSION: &str = "2.0.0";
const WORKSPACE_DOCS_COMPATIBILITY_ERROR: &str =
    "toolkit must declare a supported workspace_docs_compatibility: 4.x, 5.x, or 6.x";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolkitManifest {
    schema_version: u32,
    id: String,
    version: String,
    minimum_skm_version: String,
    #[serde(default, deserialize_with = "deserialize_workspace_docs_compatibility")]
    workspace_docs_compatibility: Option<String>,
    skills: Vec<ToolkitSkill>,
    profiles: Vec<ToolkitProfile>,
    bundles: Vec<ToolkitBundle>,
}

fn deserialize_workspace_docs_compatibility<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match serde_yaml::Value::deserialize(deserializer)? {
        serde_yaml::Value::Null => Ok(None),
        serde_yaml::Value::String(value) => Ok(Some(value)),
        _ => Err(serde::de::Error::custom(WORKSPACE_DOCS_COMPATIBILITY_ERROR)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolkitSkill {
    id: String,
    version: String,
    path: String,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolkitProfile {
    id: String,
    version: String,
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolkitBundle {
    id: String,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    profiles: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentProfile {
    id: String,
    version: String,
    description: String,
    read_only: bool,
    #[serde(default)]
    skills: Vec<String>,
    instructions: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SkillsLock {
    pub schema_version: u32,
    pub project: String,
    pub toolkit: LockedToolkit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<LockedWorkspace>,
    pub agents: Vec<LockedAgent>,
    pub bundles: Vec<String>,
    pub profiles: Vec<LockedProfile>,
    pub skills: Vec<LockedSkill>,
    pub outputs: Vec<LockedOutput>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LockedToolkit {
    pub id: String,
    pub version: String,
    pub manifest: String,
    pub integrity: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LockedWorkspace {
    pub standard: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LockedAgent {
    pub id: String,
    pub adapter_version: String,
    pub profile_capability: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LockedSkill {
    pub id: String,
    pub version: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub integrity: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LockedProfile {
    pub id: String,
    pub version: String,
    pub path: String,
    pub integrity: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LockedOutput {
    pub agent: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claimants: Vec<String>,
    pub path: String,
    pub kind: String,
    pub integrity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InstallPlan {
    pub toolkit: String,
    pub version: String,
    pub agents: Vec<String>,
    pub bundles: Vec<String>,
    pub profiles: Vec<String>,
    pub network_access: bool,
    pub writes_outside_repository: bool,
    pub actions: Vec<PlanAction>,
}

#[derive(Debug, Serialize)]
pub struct PlanAction {
    pub action: String,
    pub path: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

pub struct InstallOptions {
    pub dry_run: bool,
    pub json: bool,
}

struct ResolvedSkill {
    lock: LockedSkill,
    source_path: PathBuf,
}

struct ResolvedProfile {
    profile: AgentProfile,
    lock: LockedProfile,
}

enum DesiredKind {
    Symlink(PathBuf),
    File(Vec<u8>),
    Directory(Vec<(String, Vec<u8>)>),
}

struct DesiredOutput {
    lock: LockedOutput,
    absolute_path: PathBuf,
    kind: DesiredKind,
}

struct ResolvedPlan {
    public: InstallPlan,
    lock: SkillsLock,
    lock_bytes: Vec<u8>,
    previous_lock: Option<SkillsLock>,
    desired: Vec<DesiredOutput>,
    removals: Vec<LockedOutput>,
}

pub fn install(
    config: &SkillsConfig,
    project_root: &Path,
    options: InstallOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolved = build_plan(config, project_root)?;
    print_plan(&resolved.public, options.json)?;
    if options.dry_run {
        return Ok(());
    }
    apply_plan(project_root, resolved, None)?;
    eprintln!("Successfully installed the resolved toolkit and wrote {LOCKFILE_NAME}.");
    Ok(())
}

pub fn check(config: &SkillsConfig, project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path_exists(&project_root.join(TRANSACTION_PATH)) {
        return Err(
            "stale SKM transaction detected; inspect .skm/transactions/current before continuing"
                .into(),
        );
    }
    let resolved = build_plan(config, project_root)?;
    let previous = resolved
        .previous_lock
        .as_ref()
        .ok_or("skills.lock.yaml is missing; run 'skm install'")?;
    if previous != &resolved.lock || !resolved.public.actions.is_empty() {
        return Err("toolkit or managed outputs have drifted; run 'skm install --dry-run' to inspect the reconciliation plan".into());
    }
    eprintln!(
        "[SUCCESS] Toolkit {}@{} and {} managed outputs are valid.",
        resolved.lock.toolkit.id,
        resolved.lock.toolkit.version,
        resolved.lock.outputs.len()
    );
    Ok(())
}

pub fn list(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let Some(lock) = load_previous_lock(project_root)? else {
        println!("Toolkit: not installed (no {LOCKFILE_NAME})");
        return Ok(());
    };
    println!("Toolkit: {}@{}", lock.toolkit.id, lock.toolkit.version);
    println!("Bundles: {}", lock.bundles.join(", "));
    println!(
        "Profiles: {}",
        lock.profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    for agent in &lock.agents {
        println!(
            "Agent: {} (adapter {}, profiles: {})",
            agent.id, agent.adapter_version, agent.profile_capability
        );
    }
    for output in &lock.outputs {
        if output.kind == "skill-link" {
            let claimants = if output.claimants.is_empty() {
                output.agent.clone()
            } else {
                output.claimants.join(", ")
            };
            println!("Skill target: {} (agents: {})", output.path, claimants);
        }
    }
    println!("Managed outputs: {}", lock.outputs.len());
    Ok(())
}

fn build_plan(
    config: &SkillsConfig,
    project_root: &Path,
) -> Result<ResolvedPlan, Box<dyn std::error::Error>> {
    if path_exists(&project_root.join(TRANSACTION_PATH)) {
        return Err("stale SKM transaction detected; no writes were attempted".into());
    }
    validate_managed_parent(project_root, Path::new(TRANSACTION_PATH))?;
    let selection = config
        .toolkit
        .as_ref()
        .ok_or("toolkit configuration is missing")?;
    let manifest_path = resolve_project_source(project_root, &selection.manifest, false)?;
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: ToolkitManifest = serde_yaml::from_slice(&manifest_bytes)?;
    validate_manifest(&manifest, selection)?;

    let manifest_integrity = hash_bytes(&manifest_bytes);
    let manifest_rel = relative_string(project_root, &manifest_path)?;
    let mut selected_skill_ids = BTreeSet::new();
    let mut selected_profile_ids = BTreeSet::new();
    let bundle_map: BTreeMap<_, _> = manifest
        .bundles
        .iter()
        .map(|bundle| (bundle.id.as_str(), bundle))
        .collect();
    for bundle_id in &config.bundles {
        let bundle = bundle_map
            .get(bundle_id.as_str())
            .ok_or_else(|| format!("unknown toolkit bundle: {bundle_id}"))?;
        selected_skill_ids.extend(bundle.skills.iter().cloned());
        selected_profile_ids.extend(bundle.profiles.iter().cloned());
    }
    selected_profile_ids.extend(config.profiles.iter().cloned());

    let profile_refs: BTreeMap<_, _> = manifest
        .profiles
        .iter()
        .map(|profile| (profile.id.as_str(), profile))
        .collect();
    let mut profiles = Vec::new();
    for profile_id in &selected_profile_ids {
        let profile_ref = profile_refs
            .get(profile_id.as_str())
            .ok_or_else(|| format!("unknown toolkit profile: {profile_id}"))?;
        let profile_path = resolve_project_source(project_root, &profile_ref.path, false)?;
        reject_symlink(&profile_path)?;
        let profile_bytes = fs::read(&profile_path)?;
        let profile: AgentProfile = serde_yaml::from_slice(&profile_bytes)?;
        if profile.id != profile_ref.id || profile.version != profile_ref.version {
            return Err(format!("profile identity mismatch: {profile_id}").into());
        }
        validate_id(&profile.id, "profile")?;
        selected_skill_ids.extend(profile.skills.iter().cloned());
        profiles.push(ResolvedProfile {
            lock: LockedProfile {
                id: profile_ref.id.clone(),
                version: profile_ref.version.clone(),
                path: relative_string(project_root, &profile_path)?,
                integrity: hash_bytes(&profile_bytes),
            },
            profile,
        });
    }

    let skill_refs: BTreeMap<_, _> = manifest
        .skills
        .iter()
        .map(|skill| (skill.id.as_str(), skill))
        .collect();
    expand_toolkit_skill_dependencies(&mut selected_skill_ids, &skill_refs)?;
    let mut skills = Vec::new();
    for skill_id in &selected_skill_ids {
        let skill_ref = skill_refs
            .get(skill_id.as_str())
            .ok_or_else(|| format!("unknown toolkit skill: {skill_id}"))?;
        let source = resolve_project_source(project_root, &skill_ref.path, true)?;
        validate_skill_package(&source, &skill_ref.id)?;
        skills.push(ResolvedSkill {
            lock: LockedSkill {
                id: skill_ref.id.clone(),
                version: skill_ref.version.clone(),
                source: "toolkit".to_string(),
                path: Some(relative_string(project_root, &source)?),
                integrity: hash_directory(&source)?,
            },
            source_path: source,
        });
    }

    let configured_skills = linker::resolve_skill_dependency_closure(&config.skills, project_root)?;
    for skill in &configured_skills {
        if skills.iter().any(|existing| existing.lock.id == skill.name) {
            return Err(format!("duplicate resolved skill id: {}", skill.name).into());
        }
        let source = linker::resolve_skill_source_dir(skill, project_root)?;
        if !source.is_dir() || !source.join("SKILL.md").is_file() {
            return Err(format!("invalid skill source: {}", skill.name).into());
        }
        let local_path = if skill.path.is_some() {
            let secured = resolve_project_source(
                project_root,
                skill.path.as_deref().unwrap_or_default(),
                true,
            )?;
            if secured != fs::canonicalize(&source)? {
                return Err(
                    format!("local skill path changed during resolution: {}", skill.name).into(),
                );
            }
            Some(relative_string(project_root, &secured)?)
        } else {
            None
        };
        skills.push(ResolvedSkill {
            lock: LockedSkill {
                id: skill.name.clone(),
                version: skill
                    .version
                    .clone()
                    .unwrap_or_else(|| "latest".to_string()),
                source: skill.source.clone().unwrap_or_else(|| {
                    if skill.path.is_some() {
                        "project-local".to_string()
                    } else {
                        "default".to_string()
                    }
                }),
                path: local_path,
                integrity: hash_directory_allow_root_symlink(&source)?,
            },
            source_path: fs::canonicalize(source)?,
        });
    }
    skills.sort_by(|left, right| left.lock.id.cmp(&right.lock.id));

    for profile in &profiles {
        for skill_id in &profile.profile.skills {
            if !skills.iter().any(|skill| &skill.lock.id == skill_id) {
                return Err(format!(
                    "profile {} references unresolved skill {skill_id}",
                    profile.profile.id
                )
                .into());
            }
        }
    }

    let previous_lock = load_previous_lock(project_root)?;
    if let Some(lock) = &previous_lock {
        for output in &lock.outputs {
            let path = project_root.join(&output.path);
            if path_exists(&path) && !locked_output_matches(project_root, output)? {
                return Err(format!(
                    "managed output has drifted from prior lock ownership: {}; refusing to replace or remove it",
                    output.path
                )
                .into());
            }
        }
    }
    let previous_paths: BTreeSet<_> = previous_lock
        .as_ref()
        .map(|lock| {
            lock.outputs
                .iter()
                .map(|output| output.path.clone())
                .collect()
        })
        .unwrap_or_default();
    let mut desired = materialize_outputs(config, project_root, &skills, &profiles)?;
    desired.sort_by(|left, right| left.lock.path.cmp(&right.lock.path));
    reject_duplicate_outputs(&desired)?;

    let desired_paths: BTreeSet<_> = desired
        .iter()
        .map(|output| output.lock.path.clone())
        .collect();
    let removals: Vec<_> = previous_lock
        .as_ref()
        .map(|lock| {
            lock.outputs
                .iter()
                .filter(|output| !desired_paths.contains(&output.path))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    let agents = config
        .agents
        .iter()
        .map(|agent| LockedAgent {
            id: agent.clone(),
            adapter_version: ADAPTER_VERSION.to_string(),
            profile_capability: match agent.as_str() {
                "codex" => "native-agent".to_string(),
                "cursor" => "generated-skill-fallback".to_string(),
                _ if profiles.is_empty() => "skills-only".to_string(),
                _ => "unsupported".to_string(),
            },
        })
        .collect::<Vec<_>>();
    if let Some(agent) = agents
        .iter()
        .find(|agent| agent.profile_capability == "unsupported")
    {
        return Err(format!(
            "agent '{}' has no role-profile adapter; remove profiles or select codex/cursor",
            agent.id
        )
        .into());
    }

    let outputs = desired.iter().map(|output| output.lock.clone()).collect();
    let locked_workspace = if let Some(workspace) = &config.workspace {
        let integrity = match workspace.source.as_deref() {
            Some(source) if crate::workspace_source::is_git_source(source) => {
                workspace.integrity.clone()
            }
            Some(source) => Some(crate::workspace_source::source_integrity(
                project_root,
                source,
            )?),
            None => None,
        };
        Some(LockedWorkspace {
            standard: workspace.standard.clone(),
            source: workspace.source.clone(),
            integrity,
            revision: workspace.revision.clone(),
        })
    } else {
        None
    };
    let lock = SkillsLock {
        schema_version: 1,
        project: config.name.clone(),
        toolkit: LockedToolkit {
            id: manifest.id.clone(),
            version: manifest.version.clone(),
            manifest: manifest_rel,
            integrity: manifest_integrity,
        },
        workspace: locked_workspace,
        agents,
        bundles: config.bundles.clone(),
        profiles: profiles
            .iter()
            .map(|profile| profile.lock.clone())
            .collect(),
        skills: skills.iter().map(|skill| skill.lock.clone()).collect(),
        outputs,
    };
    let lock_bytes = serde_yaml::to_string(&lock)?.into_bytes();
    let mut actions = Vec::new();
    for output in &desired {
        validate_managed_parent(project_root, Path::new(&output.lock.path))?;
        if output_matches(output)? {
            continue;
        }
        if path_exists(&output.absolute_path) && !previous_paths.contains(&output.lock.path) {
            return Err(format!(
                "unmanaged destination collision at {}; choose merge, rename, adopt, or remove it explicitly",
                output.lock.path
            )
            .into());
        }
        actions.push(PlanAction {
            action: if path_exists(&output.absolute_path) {
                "replace".to_string()
            } else {
                "create".to_string()
            },
            path: output.lock.path.clone(),
            kind: output.lock.kind.clone(),
            target: output.lock.target.clone(),
        });
    }
    for removal in &removals {
        validate_managed_parent(project_root, Path::new(&removal.path))?;
        let path = project_root.join(&removal.path);
        if path_exists(&path) {
            actions.push(PlanAction {
                action: "remove".to_string(),
                path: removal.path.clone(),
                kind: removal.kind.clone(),
                target: removal.target.clone(),
            });
        }
    }
    let lock_path = project_root.join(LOCKFILE_NAME);
    if fs::read(&lock_path).ok().as_deref() != Some(lock_bytes.as_slice()) {
        actions.push(PlanAction {
            action: if lock_path.exists() {
                "replace".to_string()
            } else {
                "create".to_string()
            },
            path: LOCKFILE_NAME.to_string(),
            kind: "lockfile".to_string(),
            target: None,
        });
    }

    Ok(ResolvedPlan {
        public: InstallPlan {
            toolkit: manifest.id,
            version: manifest.version,
            agents: config.agents.clone(),
            bundles: config.bundles.clone(),
            profiles: lock
                .profiles
                .iter()
                .map(|profile| profile.id.clone())
                .collect(),
            network_access: false,
            writes_outside_repository: false,
            actions,
        },
        lock,
        lock_bytes,
        previous_lock,
        desired,
        removals,
    })
}

fn materialize_outputs(
    config: &SkillsConfig,
    project_root: &Path,
    skills: &[ResolvedSkill],
    profiles: &[ResolvedProfile],
) -> Result<Vec<DesiredOutput>, Box<dyn std::error::Error>> {
    let mut outputs = Vec::new();
    for target_group in linker::resolve_agent_skill_targets(&config.agents, project_root, false)? {
        for skill in skills {
            let target = linker::get_skill_target_path(&target_group.path, &skill.lock.id)?;
            outputs.push(DesiredOutput {
                lock: LockedOutput {
                    agent: target_group
                        .agents
                        .first()
                        .expect("resolved target has at least one claimant")
                        .clone(),
                    claimants: target_group.agents.clone(),
                    path: relative_string_for_output(project_root, &target)?,
                    kind: "skill-link".to_string(),
                    integrity: skill.lock.integrity.clone(),
                    target: skill.lock.path.clone().or_else(|| {
                        Some(format!(
                            "registry:{}/{}@{}",
                            skill.lock.source, skill.lock.id, skill.lock.version
                        ))
                    }),
                },
                absolute_path: target,
                kind: DesiredKind::Symlink(skill.source_path.clone()),
            });
        }
    }

    for agent in &config.agents {
        for resolved in profiles {
            let profile = &resolved.profile;
            match agent.as_str() {
                "codex" => {
                    let content = render_codex_profile(profile).into_bytes();
                    let target = project_root
                        .join(".codex/agents")
                        .join(format!("{}.toml", profile.id));
                    outputs.push(DesiredOutput {
                        lock: LockedOutput {
                            agent: agent.clone(),
                            claimants: vec![agent.clone()],
                            path: relative_string_for_output(project_root, &target)?,
                            kind: "native-profile".to_string(),
                            integrity: hash_bytes(&content),
                            target: Some(format!("profile:{}@{}", profile.id, profile.version)),
                        },
                        absolute_path: target,
                        kind: DesiredKind::File(content),
                    });
                }
                "cursor" => {
                    let content = render_profile_skill(profile).into_bytes();
                    let generated = project_root
                        .join(".skm/generated/profiles")
                        .join(&profile.id);
                    outputs.push(DesiredOutput {
                        lock: LockedOutput {
                            agent: "shared".to_string(),
                            claimants: Vec::new(),
                            path: relative_string_for_output(project_root, &generated)?,
                            kind: "generated-profile".to_string(),
                            integrity: hash_bytes(&content),
                            target: Some(format!("profile:{}@{}", profile.id, profile.version)),
                        },
                        absolute_path: generated.clone(),
                        kind: DesiredKind::Directory(vec![("SKILL.md".to_string(), content)]),
                    });
                    let target = project_root.join(".cursor/skills").join(&profile.id);
                    outputs.push(DesiredOutput {
                        lock: LockedOutput {
                            agent: agent.clone(),
                            claimants: vec![agent.clone()],
                            path: relative_string_for_output(project_root, &target)?,
                            kind: "profile-fallback-link".to_string(),
                            integrity: outputs
                                .last()
                                .expect("generated profile output")
                                .lock
                                .integrity
                                .clone(),
                            target: Some(relative_string_for_output(project_root, &generated)?),
                        },
                        absolute_path: target,
                        kind: DesiredKind::Symlink(generated),
                    });
                }
                _ => {}
            }
        }
    }
    Ok(outputs)
}

fn validate_manifest(
    manifest: &ToolkitManifest,
    selection: &crate::config::ToolkitSelection,
) -> Result<(), Box<dyn std::error::Error>> {
    if manifest.schema_version != 1 {
        return Err(format!(
            "unsupported toolkit schema {}; expected 1",
            manifest.schema_version
        )
        .into());
    }
    validate_id(&manifest.id, "toolkit")?;
    if manifest.version != selection.version {
        return Err(format!(
            "toolkit version mismatch: requested {}, found {}",
            selection.version, manifest.version
        )
        .into());
    }
    let current = parse_semver(env!("CARGO_PKG_VERSION"))?;
    let minimum = parse_semver(&manifest.minimum_skm_version)?;
    if current < minimum {
        return Err(format!(
            "toolkit requires SKM {}, current version is {}",
            manifest.minimum_skm_version,
            env!("CARGO_PKG_VERSION")
        )
        .into());
    }
    if !matches!(
        manifest.workspace_docs_compatibility.as_deref(),
        Some("4.x" | "5.x" | "6.x")
    ) {
        return Err(WORKSPACE_DOCS_COMPATIBILITY_ERROR.into());
    }
    ensure_unique_ids(manifest.skills.iter().map(|item| item.id.as_str()), "skill")?;
    ensure_unique_ids(
        manifest.profiles.iter().map(|item| item.id.as_str()),
        "profile",
    )?;
    ensure_unique_ids(
        manifest.bundles.iter().map(|item| item.id.as_str()),
        "bundle",
    )?;
    let skill_ids: BTreeSet<_> = manifest
        .skills
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let profile_ids: BTreeSet<_> = manifest
        .profiles
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    for skill in &manifest.skills {
        parse_semver(&skill.version)?;
        for dependency in &skill.dependencies {
            if !skill_ids.contains(dependency.as_str()) {
                return Err(format!(
                    "skill {} references unknown dependency {dependency}",
                    skill.id
                )
                .into());
            }
        }
    }
    ensure_acyclic_toolkit_dependencies(manifest)?;
    for profile in &manifest.profiles {
        parse_semver(&profile.version)?;
    }
    for bundle in &manifest.bundles {
        for skill in &bundle.skills {
            if !skill_ids.contains(skill.as_str()) {
                return Err(
                    format!("bundle {} references unknown skill {skill}", bundle.id).into(),
                );
            }
        }
        for profile in &bundle.profiles {
            if !profile_ids.contains(profile.as_str()) {
                return Err(
                    format!("bundle {} references unknown profile {profile}", bundle.id).into(),
                );
            }
        }
    }
    Ok(())
}

fn expand_toolkit_skill_dependencies<'a>(
    selected: &mut BTreeSet<String>,
    skills: &BTreeMap<&'a str, &'a ToolkitSkill>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pending: Vec<_> = selected.iter().cloned().collect();
    while let Some(skill_id) = pending.pop() {
        let skill = skills
            .get(skill_id.as_str())
            .ok_or_else(|| format!("unknown toolkit skill: {skill_id}"))?;
        for dependency in &skill.dependencies {
            if selected.insert(dependency.clone()) {
                pending.push(dependency.clone());
            }
        }
    }
    Ok(())
}

fn ensure_acyclic_toolkit_dependencies(
    manifest: &ToolkitManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let skills: BTreeMap<_, _> = manifest
        .skills
        .iter()
        .map(|skill| (skill.id.as_str(), skill))
        .collect();
    let mut complete = BTreeSet::new();
    let mut active = BTreeSet::new();
    let mut stack = Vec::new();
    for skill in &manifest.skills {
        visit_toolkit_dependency(&skill.id, &skills, &mut complete, &mut active, &mut stack)?;
    }
    Ok(())
}

fn visit_toolkit_dependency<'a>(
    skill_id: &str,
    skills: &BTreeMap<&'a str, &'a ToolkitSkill>,
    complete: &mut BTreeSet<String>,
    active: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if complete.contains(skill_id) {
        return Ok(());
    }
    if !active.insert(skill_id.to_string()) {
        stack.push(skill_id.to_string());
        let cycle = stack.join(" -> ");
        stack.pop();
        return Err(format!("toolkit skill dependency cycle: {cycle}").into());
    }
    stack.push(skill_id.to_string());
    let skill = skills
        .get(skill_id)
        .ok_or_else(|| format!("unknown toolkit skill: {skill_id}"))?;
    for dependency in &skill.dependencies {
        visit_toolkit_dependency(dependency, skills, complete, active, stack)?;
    }
    stack.pop();
    active.remove(skill_id);
    complete.insert(skill_id.to_string());
    Ok(())
}

fn ensure_unique_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    kind: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut seen = BTreeSet::new();
    for id in ids {
        validate_id(id, kind)?;
        if !seen.insert(id) {
            return Err(format!("duplicate {kind} id: {id}").into());
        }
    }
    Ok(())
}

fn validate_id(id: &str, kind: &str) -> Result<(), Box<dyn std::error::Error>> {
    let valid = !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--");
    if !valid {
        return Err(format!("invalid {kind} id: {id}").into());
    }
    Ok(())
}

fn parse_semver(value: &str) -> Result<(u64, u64, u64), Box<dyn std::error::Error>> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("invalid semantic version: {value}").into());
    }
    Ok((parts[0].parse()?, parts[1].parse()?, parts[2].parse()?))
}

fn validate_skill_package(
    path: &Path,
    expected_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    reject_tree_symlinks(path)?;
    let skill_path = path.join("SKILL.md");
    let content = fs::read_to_string(&skill_path)?;
    let Some(rest) = content.strip_prefix("---\n") else {
        return Err(format!("{} has invalid SKILL.md frontmatter", path.display()).into());
    };
    let Some((frontmatter, _)) = rest.split_once("\n---") else {
        return Err(format!("{} has unclosed SKILL.md frontmatter", path.display()).into());
    };
    let name = frontmatter
        .lines()
        .find_map(|line| line.strip_prefix("name:").map(str::trim));
    let description = frontmatter
        .lines()
        .find_map(|line| line.strip_prefix("description:").map(str::trim));
    if name != Some(expected_id) || description.is_none_or(str::is_empty) {
        return Err(format!(
            "{} has invalid skill identity or description",
            path.display()
        )
        .into());
    }
    Ok(())
}

fn resolve_project_source(
    project_root: &Path,
    relative: &str,
    directory: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("source path must be repository-relative: {relative}").into());
    }
    let joined = project_root.join(path);
    let canonical_root = fs::canonicalize(project_root)?;
    reject_symlink_components(project_root, path)?;
    let canonical = fs::canonicalize(&joined)?;
    canonical
        .strip_prefix(&canonical_root)
        .map_err(|_| format!("source path escapes repository: {relative}"))?;
    if (directory && !canonical.is_dir()) || (!directory && !canonical.is_file()) {
        return Err(format!("source has the wrong type: {relative}").into());
    }
    if directory {
        reject_tree_symlinks(&canonical)?;
    } else {
        reject_symlink(&joined)?;
    }
    Ok(canonical)
}

fn reject_symlink_components(
    root: &Path,
    relative: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if let Component::Normal(part) = component {
            current.push(part);
            if fs::symlink_metadata(&current)?.file_type().is_symlink() {
                return Err(format!("source path contains symlink: {}", relative.display()).into());
            }
        }
    }
    Ok(())
}

fn reject_tree_symlinks(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_symlink() {
            return Err(format!(
                "package contains unsafe symlink: {}",
                entry.path().display()
            )
            .into());
        }
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(format!("source is an unsafe symlink: {}", path.display()).into());
    }
    Ok(())
}

fn hash_directory(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    reject_tree_symlinks(path)?;
    hash_directory_allow_root_symlink(path)
}

fn hash_directory_allow_root_symlink(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let canonical = fs::canonicalize(path)?;
    let mut files = Vec::new();
    for entry in WalkDir::new(&canonical).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_symlink() {
            return Err(
                format!("skill contains unsafe symlink: {}", entry.path().display()).into(),
            );
        }
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    let mut hasher = Sha256::new();
    for file in files {
        let relative = file.strip_prefix(&canonical)?;
        let relative = relative
            .to_str()
            .ok_or("skill path contains non-UTF-8 data")?
            .replace('\\', "/");
        hasher.update((relative.len() as u64).to_be_bytes());
        hasher.update(relative.as_bytes());
        let content = fs::read(file)?;
        hasher.update((content.len() as u64).to_be_bytes());
        hasher.update(content);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn render_codex_profile(profile: &AgentProfile) -> String {
    let skills = profile
        .skills
        .iter()
        .map(|skill| format!("${skill}"))
        .collect::<Vec<_>>()
        .join(", ");
    let instructions = format!(
        "{}\n\nPreferred project skills: {}",
        profile.instructions, skills
    );
    let mut rendered = format!(
        "name = {}\ndescription = {}\ndeveloper_instructions = {}\n",
        json_string(&profile.id),
        json_string(&profile.description),
        json_string(&instructions)
    );
    if profile.read_only {
        rendered.push_str("sandbox_mode = \"read-only\"\n");
    }
    rendered
}

fn render_profile_skill(profile: &AgentProfile) -> String {
    let description = format!(
        "Portable role fallback for Cursor. {}",
        profile.description.replace(['\n', '\r'], " ")
    );
    let skills = profile
        .skills
        .iter()
        .map(|skill| format!("`{skill}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n# {}\n\nThis is a generated role-profile fallback, not a native isolated subagent.\n\n{}\n\n## Preferred Skills\n\n{}\n",
        profile.id,
        json_string(&description),
        title_case(&profile.id),
        profile.instructions,
        skills
    )
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string cannot fail")
}

fn title_case(id: &str) -> String {
    id.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn reject_duplicate_outputs(outputs: &[DesiredOutput]) -> Result<(), Box<dyn std::error::Error>> {
    let mut paths: Vec<&Path> = Vec::new();
    for output in outputs {
        let path = Path::new(&output.lock.path);
        if paths.iter().any(|existing| {
            path == *existing || path.starts_with(existing) || existing.starts_with(path)
        }) {
            return Err(format!("managed outputs overlap at: {}", output.lock.path).into());
        }
        paths.push(path);
    }
    Ok(())
}

fn output_matches(output: &DesiredOutput) -> Result<bool, Box<dyn std::error::Error>> {
    if !path_exists(&output.absolute_path) {
        return Ok(false);
    }
    match &output.kind {
        DesiredKind::Symlink(source) => {
            if !fs::symlink_metadata(&output.absolute_path)?
                .file_type()
                .is_symlink()
            {
                return Ok(false);
            }
            linker::symlink_points_to(&output.absolute_path, source)
        }
        DesiredKind::File(content) => Ok(output.absolute_path.is_file()
            && !output.absolute_path.is_symlink()
            && fs::read(&output.absolute_path)? == *content),
        DesiredKind::Directory(files) => {
            if !output.absolute_path.is_dir() || output.absolute_path.is_symlink() {
                return Ok(false);
            }
            let mut expected = BTreeSet::new();
            for (relative, content) in files {
                expected.insert(relative.clone());
                if fs::read(output.absolute_path.join(relative))
                    .ok()
                    .as_deref()
                    != Some(content.as_slice())
                {
                    return Ok(false);
                }
            }
            let mut actual = BTreeSet::new();
            for entry in WalkDir::new(&output.absolute_path).min_depth(1) {
                let entry = entry?;
                if !entry.file_type().is_file() {
                    return Ok(false);
                }
                actual.insert(
                    entry
                        .path()
                        .strip_prefix(&output.absolute_path)?
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
            Ok(actual == expected)
        }
    }
}

fn locked_output_matches(
    project_root: &Path,
    output: &LockedOutput,
) -> Result<bool, Box<dyn std::error::Error>> {
    let path = project_root.join(&output.path);
    let metadata = fs::symlink_metadata(&path)?;
    match output.kind.as_str() {
        "skill-link" => {
            if !metadata.file_type().is_symlink() {
                return Ok(false);
            }
            if let Some(target) = output.target.as_deref() {
                let expected = if target.starts_with("registry:") {
                    let skill = linker::parse_registry_lock_target(target)?;
                    linker::resolve_skill_source_dir(&skill, project_root)?
                } else {
                    project_root.join(target)
                };
                if !linker::symlink_points_to(&path, &expected)? {
                    return Ok(false);
                }
                if hash_directory_allow_root_symlink(&expected)? != output.integrity {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        "native-profile" => Ok(metadata.is_file()
            && !metadata.file_type().is_symlink()
            && hash_bytes(&fs::read(path)?) == output.integrity),
        "generated-profile" => {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Ok(false);
            }
            let skill = path.join("SKILL.md");
            let mut entries = fs::read_dir(&path)?;
            let only_skill = entries
                .next()
                .transpose()?
                .is_some_and(|entry| entry.file_name() == "SKILL.md")
                && entries.next().transpose()?.is_none();
            Ok(only_skill
                && skill.is_file()
                && !skill.is_symlink()
                && hash_bytes(&fs::read(skill)?) == output.integrity)
        }
        "profile-fallback-link" => {
            if !metadata.file_type().is_symlink() {
                return Ok(false);
            }
            let Some(target) = output.target.as_deref() else {
                return Ok(false);
            };
            let expected = project_root.join(target);
            Ok(linker::symlink_points_to(&path, &expected)?
                && expected.is_dir()
                && hash_bytes(&fs::read(expected.join("SKILL.md"))?) == output.integrity)
        }
        _ => Ok(false),
    }
}

fn apply_plan(
    project_root: &Path,
    resolved: ResolvedPlan,
    fail_after: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    if resolved.public.actions.is_empty() {
        return Ok(());
    }
    let transaction = project_root.join(TRANSACTION_PATH);
    let skm_existed = path_exists(&project_root.join(".skm"));
    let transactions_existed = path_exists(&project_root.join(".skm/transactions"));
    let journal = serde_yaml::to_string(&resolved.public)?;
    fs::create_dir_all(&transaction)?;
    if let Err(error) = fs::write(transaction.join("journal.yaml"), journal) {
        let _ = remove_path(&transaction);
        cleanup_transaction_parents(project_root, skm_existed, transactions_existed);
        return Err(error.into());
    }
    let backup_root = transaction.join("backup");
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut created: Vec<PathBuf> = Vec::new();
    let mut created_directories: Vec<PathBuf> = Vec::new();
    let mut operation_count = 0usize;

    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        for removal in &resolved.removals {
            let original = project_root.join(&removal.path);
            if path_exists(&original) {
                backup_path(&original, project_root, &backup_root, &mut moved)?;
                operation_count += 1;
                maybe_fail(operation_count, fail_after)?;
            }
        }

        for output in &resolved.desired {
            if output_matches(output)? {
                continue;
            }
            if path_exists(&output.absolute_path) {
                backup_path(
                    &output.absolute_path,
                    project_root,
                    &backup_root,
                    &mut moved,
                )?;
            }
            created.push(output.absolute_path.clone());
            create_output(output, project_root, &mut created_directories)?;
            operation_count += 1;
            maybe_fail(operation_count, fail_after)?;
        }

        let lock_path = project_root.join(LOCKFILE_NAME);
        if fs::read(&lock_path).ok().as_deref() != Some(resolved.lock_bytes.as_slice()) {
            if path_exists(&lock_path) {
                backup_path(&lock_path, project_root, &backup_root, &mut moved)?;
            }
            let temporary = transaction.join("new-skills.lock.yaml");
            fs::write(&temporary, &resolved.lock_bytes)?;
            fs::rename(&temporary, &lock_path)?;
            created.push(lock_path);
        }
        Ok(())
    })();

    if let Err(error) = result {
        let rollback = rollback(&created, &moved, &created_directories);
        let _ = remove_path(&transaction);
        cleanup_transaction_parents(project_root, skm_existed, transactions_existed);
        return match rollback {
            Ok(()) => Err(format!("installation failed and rollback succeeded: {error}").into()),
            Err(rollback_error) => Err(format!(
                "installation failed: {error}; rollback also failed: {rollback_error}"
            )
            .into()),
        };
    }

    remove_path(&transaction)?;
    cleanup_transaction_parents(project_root, skm_existed, transactions_existed);
    Ok(())
}

fn maybe_fail(
    operation_count: usize,
    fail_after: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    if fail_after == Some(operation_count) {
        return Err("injected transaction failure".into());
    }
    Ok(())
}

fn backup_path(
    original: &Path,
    project_root: &Path,
    backup_root: &Path,
    moved: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let relative = original.strip_prefix(project_root)?;
    let backup = backup_root.join(relative);
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(original, &backup)?;
    moved.push((original.to_path_buf(), backup));
    Ok(())
}

fn create_output(
    output: &DesiredOutput,
    project_root: &Path,
    created_directories: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_parent_directories(&output.absolute_path, project_root, created_directories)?;
    match &output.kind {
        DesiredKind::Symlink(source) => linker::symlink_dir(source, &output.absolute_path)?,
        DesiredKind::File(content) => fs::write(&output.absolute_path, content)?,
        DesiredKind::Directory(files) => {
            fs::create_dir(&output.absolute_path)?;
            for (relative, content) in files {
                let path = output.absolute_path.join(relative);
                ensure_parent_directories(&path, project_root, created_directories)?;
                fs::write(path, content)?;
            }
        }
    }
    Ok(())
}

fn rollback(
    created: &[PathBuf],
    moved: &[(PathBuf, PathBuf)],
    created_directories: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    for path in created.iter().rev() {
        remove_path(path)?;
    }
    for (original, backup) in moved.iter().rev() {
        if path_exists(original) {
            remove_path(original)?;
        }
        if let Some(parent) = original.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(backup, original)?;
    }
    for directory in created_directories.iter().rev() {
        match fs::remove_dir(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn remove_path(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_file() => {
            fs::remove_file(path)
        }
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn ensure_parent_directories(
    path: &Path,
    project_root: &Path,
    created_directories: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut missing = Vec::new();
    let mut current = path.parent();
    while let Some(directory) = current {
        if directory == project_root {
            break;
        }
        if path_exists(directory) {
            break;
        }
        missing.push(directory.to_path_buf());
        current = directory.parent();
    }
    for directory in missing.into_iter().rev() {
        fs::create_dir(&directory)?;
        created_directories.push(directory);
    }
    Ok(())
}

fn cleanup_transaction_parents(project_root: &Path, skm_existed: bool, transactions_existed: bool) {
    let transactions = project_root.join(".skm/transactions");
    if !transactions_existed {
        let _ = fs::remove_dir(&transactions);
    }
    if !skm_existed {
        let _ = fs::remove_dir(project_root.join(".skm"));
    }
}

fn load_previous_lock(
    project_root: &Path,
) -> Result<Option<SkillsLock>, Box<dyn std::error::Error>> {
    let path = project_root.join(LOCKFILE_NAME);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("skills.lock.yaml must not be a symlink".into())
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err("skills.lock.yaml must be a regular file".into())
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let lock: SkillsLock = serde_yaml::from_slice(&fs::read(path)?)?;
    if lock.schema_version != 1 {
        return Err(format!("unsupported lockfile schema: {}", lock.schema_version).into());
    }
    let mut seen: Vec<&Path> = Vec::new();
    for output in &lock.outputs {
        validate_locked_output(output)?;
        if output.agent == "codex"
            && output.kind == "skill-link"
            && Path::new(&output.path).starts_with(".codex/skills")
            && !lock
                .agents
                .iter()
                .any(|agent| agent.id == "codex" && agent.adapter_version == "1.0.0")
        {
            return Err(
                "legacy .codex/skills ownership requires the Codex 1.0.0 adapter record".into(),
            );
        }
        let path = Path::new(&output.path);
        if seen.iter().any(|existing| {
            path == *existing || path.starts_with(existing) || existing.starts_with(path)
        }) {
            return Err(format!("overlapping managed output in lockfile: {}", output.path).into());
        }
        seen.push(path);
    }
    Ok(Some(lock))
}

fn validate_locked_output(output: &LockedOutput) -> Result<(), Box<dyn std::error::Error>> {
    let value = &output.path;
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("unsafe managed output path in lockfile: {value}").into());
    }
    match output.kind.as_str() {
        "skill-link" => {
            let claimants = if output.claimants.is_empty() {
                vec![output.agent.as_str()]
            } else {
                if output.claimants.first().map(String::as_str) != Some(output.agent.as_str()) {
                    return Err("skill output primary agent must be its first claimant".into());
                }
                output.claimants.iter().map(String::as_str).collect()
            };
            let mut skill = None;
            for claimant in claimants {
                let prefixes = linker::project_agent_skill_paths(claimant)
                    .ok_or_else(|| format!("unknown lockfile adapter: {claimant}"))?;
                let matched = prefixes
                    .iter()
                    .find_map(|prefix| path.strip_prefix(prefix).ok())
                    .or_else(|| {
                        (output.claimants.is_empty()
                            && claimant == "codex"
                            && path.starts_with(".codex/skills"))
                        .then(|| path.strip_prefix(".codex/skills").expect("checked prefix"))
                    })
                    .or_else(|| {
                        (output.claimants.is_empty()
                            && claimant == "hermes"
                            && path.starts_with(".hermes/skills"))
                        .then(|| path.strip_prefix(".hermes/skills").expect("checked prefix"))
                    })
                    .ok_or_else(|| {
                        format!("skill output is outside the {claimant} adapter namespace")
                    })?;
                if skill.is_none() {
                    skill = Some(matched);
                }
            }
            let skill = skill.ok_or("skill output has no adapter claimants")?;
            linker::validated_skill_path(
                skill
                    .to_str()
                    .ok_or("managed skill path contains non-UTF-8 data")?,
            )?;
        }
        "native-profile" => {
            if output.agent != "codex"
                || path.parent() != Some(Path::new(".codex/agents"))
                || path.extension().and_then(|value| value.to_str()) != Some("toml")
            {
                return Err("native profile output is outside the Codex adapter namespace".into());
            }
            validate_id(
                path.file_stem()
                    .and_then(|value| value.to_str())
                    .ok_or("native profile name is invalid")?,
                "profile",
            )?;
        }
        "profile-fallback-link" => {
            if output.agent != "cursor" || path.parent() != Some(Path::new(".cursor/skills")) {
                return Err(
                    "fallback profile output is outside the Cursor adapter namespace".into(),
                );
            }
            validate_id(
                path.file_name()
                    .and_then(|value| value.to_str())
                    .ok_or("fallback profile name is invalid")?,
                "profile",
            )?;
        }
        "generated-profile" => {
            if output.agent != "shared"
                || path.parent() != Some(Path::new(".skm/generated/profiles"))
            {
                return Err("generated profile output is outside the SKM namespace".into());
            }
            validate_id(
                path.file_name()
                    .and_then(|value| value.to_str())
                    .ok_or("generated profile name is invalid")?,
                "profile",
            )?;
        }
        kind => return Err(format!("unknown managed output kind in lockfile: {kind}").into()),
    }
    Ok(())
}

fn validate_managed_parent(
    project_root: &Path,
    relative: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("managed output path escapes the repository".into());
    }
    let components: Vec<_> = relative.components().collect();
    let mut current = project_root.to_path_buf();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        let Component::Normal(part) = component else {
            return Err("managed output path escapes the repository".into());
        };
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "managed output parent is a symlink: {}",
                    current.strip_prefix(project_root)?.display()
                )
                .into());
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(format!(
                    "managed output parent is not a directory: {}",
                    current.strip_prefix(project_root)?.display()
                )
                .into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => break,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn relative_string(root: &Path, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let root = fs::canonicalize(root)?;
    let path = fs::canonicalize(path)?;
    let relative = path.strip_prefix(root)?;
    relative
        .to_str()
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| "path contains non-UTF-8 data".into())
}

fn relative_string_for_output(
    root: &Path,
    path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let relative = path.strip_prefix(root)?;
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("managed output escapes the repository".into());
    }
    relative
        .to_str()
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| "managed output path contains non-UTF-8 data".into())
}

fn path_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn print_plan(plan: &InstallPlan, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(plan)?);
        return Ok(());
    }
    println!("Toolkit plan: {}@{}", plan.toolkit, plan.version);
    println!("Agents: {}", plan.agents.join(", "));
    println!("Network access: {}", plan.network_access);
    println!(
        "Writes outside repository: {}",
        plan.writes_outside_repository
    );
    if plan.actions.is_empty() {
        println!("No changes required.");
    } else {
        for action in &plan.actions {
            if let Some(target) = &action.target {
                println!(
                    "- {} {} ({}) -> {}",
                    action.action, action.path, action.kind, target
                );
            } else {
                println!("- {} {} ({})", action.action, action.path, action.kind);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ToolkitSelection, WorkspaceSelection};
    use tempfile::TempDir;

    fn fixture() -> (TempDir, SkillsConfig) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("workspace/instructions/toolkit")).unwrap();
        fs::create_dir_all(root.join("workspace/instructions/skills/write-spec/agents")).unwrap();
        fs::create_dir_all(root.join("workspace/instructions/agents")).unwrap();
        fs::write(
            root.join("workspace/instructions/skills/write-spec/SKILL.md"),
            "---\nname: write-spec\ndescription: Write a specification.\n---\n\n# Write Spec\n",
        )
        .unwrap();
        fs::write(
            root.join("workspace/instructions/skills/write-spec/agents/openai.yaml"),
            "interface: {}\n",
        )
        .unwrap();
        fs::write(
            root.join("workspace/instructions/agents/delivery-planner.yaml"),
            "id: delivery-planner\nversion: 0.1.0\ndescription: Plan delivery.\nread_only: true\nskills:\n  - write-spec\ninstructions: Stay read-only.\n",
        )
        .unwrap();
        fs::write(
            root.join("workspace/instructions/toolkit/manifest.yaml"),
            "schema_version: 1\nid: test-toolkit\nversion: 0.1.0\nminimum_skm_version: 0.2.0\nworkspace_docs_compatibility: 4.x\nskills:\n  - id: write-spec\n    version: 0.1.0\n    path: workspace/instructions/skills/write-spec\nprofiles:\n  - id: delivery-planner\n    version: 0.1.0\n    path: workspace/instructions/agents/delivery-planner.yaml\nbundles:\n  - id: development-core\n    skills:\n      - write-spec\n    profiles:\n      - delivery-planner\n",
        )
        .unwrap();
        let config = SkillsConfig {
            name: "test".to_string(),
            version: Some("1.0.0".to_string()),
            registries: None,
            agents: vec!["codex".to_string(), "cursor".to_string()],
            skills: Vec::new(),
            toolkit: Some(ToolkitSelection {
                manifest: "workspace/instructions/toolkit/manifest.yaml".to_string(),
                version: "0.1.0".to_string(),
            }),
            bundles: vec!["development-core".to_string()],
            profiles: Vec::new(),
            workspace: Some(WorkspaceSelection {
                standard: "workspace-docs@4.0.0".to_string(),
                source: None,
                revision: None,
                integrity: None,
            }),
            trusted_sources: Vec::new(),
        };
        (temp, config)
    }

    #[test]
    fn installs_two_adapters_and_is_idempotent() {
        let (temp, config) = fixture();
        let first = build_plan(&config, temp.path()).unwrap();
        assert!(first.public.actions.len() >= 6);
        apply_plan(temp.path(), first, None).unwrap();
        assert!(temp.path().join(".agents/skills/write-spec").is_symlink());
        assert!(temp
            .path()
            .join(".codex/agents/delivery-planner.toml")
            .is_file());
        assert!(temp
            .path()
            .join(".cursor/skills/delivery-planner")
            .is_symlink());
        let second = build_plan(&config, temp.path()).unwrap();
        assert!(second.public.actions.is_empty());
    }

    #[test]
    fn shared_agent_directory_has_one_output_with_all_claimants() {
        let (temp, mut config) = fixture();
        let manifest_path = temp
            .path()
            .join("workspace/instructions/toolkit/manifest.yaml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        fs::write(
            &manifest_path,
            manifest.replace(
                "    profiles:\n      - delivery-planner",
                "    profiles: []",
            ),
        )
        .unwrap();
        config.agents = ["codex", "goose", "openhands", "antigravity"]
            .into_iter()
            .map(str::to_string)
            .collect();

        let plan = build_plan(&config, temp.path()).unwrap();
        let outputs = plan
            .lock
            .outputs
            .iter()
            .filter(|output| output.path == ".agents/skills/write-spec")
            .collect::<Vec<_>>();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].claimants, config.agents);
        assert_eq!(
            plan.public
                .actions
                .iter()
                .filter(|action| action.path == ".agents/skills/write-spec")
                .count(),
            1
        );

        apply_plan(temp.path(), plan, None).unwrap();
        check(&config, temp.path()).unwrap();
        assert!(build_plan(&config, temp.path())
            .unwrap()
            .public
            .actions
            .is_empty());
    }

    #[test]
    fn installs_and_checks_all_sixteen_agents() {
        let (temp, mut config) = fixture();
        let manifest_path = temp
            .path()
            .join("workspace/instructions/toolkit/manifest.yaml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        fs::write(
            &manifest_path,
            manifest.replace(
                "    profiles:\n      - delivery-planner",
                "    profiles: []",
            ),
        )
        .unwrap();
        config.agents = linker::SUPPORTED_AGENTS
            .iter()
            .map(|agent| (*agent).to_string())
            .collect();

        let plan = build_plan(&config, temp.path()).unwrap();
        apply_plan(temp.path(), plan, None).unwrap();
        check(&config, temp.path()).unwrap();

        assert!(!temp.path().join(".hermes/skills").exists());
        assert!(temp.path().join(".agents/skills/write-spec").is_symlink());
        assert!(temp.path().join(".qwen/skills/write-spec").is_symlink());
    }

    #[test]
    fn hermes_project_target_is_removed_only_when_owned_by_previous_lock() {
        let (temp, mut config) = fixture();
        let root = temp.path();
        apply_plan(root, build_plan(&config, root).unwrap(), None).unwrap();

        let current = root.join(".agents/skills/write-spec");
        let legacy = root.join(".hermes/skills/write-spec");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::rename(&current, &legacy).unwrap();
        let lock_path = root.join(LOCKFILE_NAME);
        let mut lock: SkillsLock = serde_yaml::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
        let legacy_output = lock
            .outputs
            .iter()
            .find(|output| output.agent == "codex" && output.kind == "skill-link")
            .unwrap()
            .clone();
        lock.agents = vec![LockedAgent {
            id: "hermes".to_string(),
            adapter_version: ADAPTER_VERSION.to_string(),
            profile_capability: "skills-only".to_string(),
        }];
        lock.outputs = vec![LockedOutput {
            agent: "hermes".to_string(),
            claimants: Vec::new(),
            path: ".hermes/skills/write-spec".to_string(),
            ..legacy_output
        }];
        fs::write(&lock_path, serde_yaml::to_string(&lock).unwrap()).unwrap();

        let manifest_path = root.join("workspace/instructions/toolkit/manifest.yaml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        fs::write(
            manifest_path,
            manifest.replace(
                "    profiles:\n      - delivery-planner",
                "    profiles: []",
            ),
        )
        .unwrap();
        config.agents = vec!["hermes".to_string()];
        let plan = build_plan(&config, root).unwrap();
        assert!(plan
            .public
            .actions
            .iter()
            .any(|action| action.action == "remove" && action.path == ".hermes/skills/write-spec"));
        apply_plan(root, plan, None).unwrap();
        assert!(!legacy.exists() && !legacy.is_symlink());
        assert!(!root.join(".hermes/skills/write-spec").exists());
    }

    #[test]
    fn hermes_project_target_leaves_unmanaged_content_untouched() {
        let (temp, mut config) = fixture();
        let root = temp.path();
        let unmanaged = root.join(".hermes/skills/write-spec");
        fs::create_dir_all(&unmanaged).unwrap();
        fs::write(unmanaged.join("keep"), "user data").unwrap();
        let manifest_path = root.join("workspace/instructions/toolkit/manifest.yaml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        fs::write(
            manifest_path,
            manifest.replace(
                "    profiles:\n      - delivery-planner",
                "    profiles: []",
            ),
        )
        .unwrap();
        config.agents = vec!["hermes".to_string()];

        let plan = build_plan(&config, root).unwrap();
        assert!(!plan
            .public
            .actions
            .iter()
            .any(|action| action.path == ".hermes/skills/write-spec"));
        apply_plan(root, plan, None).unwrap();
        assert_eq!(
            fs::read_to_string(unmanaged.join("keep")).unwrap(),
            "user data"
        );
    }

    #[test]
    fn toolkit_bundle_expands_transitive_skill_dependencies() {
        let (temp, mut config) = fixture();
        let root = temp.path();
        fs::create_dir_all(root.join("workspace/instructions/skills/wk-spec/agents")).unwrap();
        fs::write(
            root.join("workspace/instructions/skills/wk-spec/SKILL.md"),
            "---\nname: wk-spec\ndescription: Route specification work.\n---\n\n# wk.spec\n",
        )
        .unwrap();
        fs::write(
            root.join("workspace/instructions/skills/wk-spec/agents/openai.yaml"),
            "interface: {}\n",
        )
        .unwrap();
        let manifest_path = root.join("workspace/instructions/toolkit/manifest.yaml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        let manifest = manifest
            .replace(
                "profiles:\n  - id: delivery-planner",
                "  - id: wk-spec\n    version: 0.1.0\n    path: workspace/instructions/skills/wk-spec\n    dependencies:\n      - write-spec\nprofiles:\n  - id: delivery-planner",
            )
            .replace(
                "    skills:\n      - write-spec\n    profiles:\n      - delivery-planner",
                "    skills:\n      - wk-spec\n    profiles: []",
            );
        fs::write(manifest_path, manifest).unwrap();
        config.profiles.clear();

        let plan = build_plan(&config, root).unwrap();
        let ids: Vec<_> = plan
            .lock
            .skills
            .iter()
            .map(|skill| skill.id.as_str())
            .collect();

        assert_eq!(ids, ["wk-spec", "write-spec"]);
        assert!(plan
            .public
            .actions
            .iter()
            .any(|action| action.path == ".agents/skills/wk-spec"));
        assert!(plan
            .public
            .actions
            .iter()
            .any(|action| action.path == ".agents/skills/write-spec"));
    }

    #[test]
    fn rejects_toolkit_skill_dependency_cycle_before_writes() {
        let (temp, config) = fixture();
        let root = temp.path();
        fs::create_dir_all(root.join("workspace/instructions/skills/wk-spec")).unwrap();
        fs::write(
            root.join("workspace/instructions/skills/wk-spec/SKILL.md"),
            "---\nname: wk-spec\ndescription: Route specification work.\n---\n\n# wk.spec\n",
        )
        .unwrap();
        let manifest_path = root.join("workspace/instructions/toolkit/manifest.yaml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        let manifest = manifest
            .replace(
                "    path: workspace/instructions/skills/write-spec",
                "    path: workspace/instructions/skills/write-spec\n    dependencies:\n      - wk-spec",
            )
            .replace(
                "profiles:\n  - id: delivery-planner",
                "  - id: wk-spec\n    version: 0.1.0\n    path: workspace/instructions/skills/wk-spec\n    dependencies:\n      - write-spec\nprofiles:\n  - id: delivery-planner",
            );
        fs::write(manifest_path, manifest).unwrap();

        let error = build_plan(&config, root).err().unwrap();

        assert!(error.to_string().contains("dependency cycle"));
        assert!(!root.join(LOCKFILE_NAME).exists());
        assert!(!root.join(".agents/skills/write-spec").exists());
    }

    #[test]
    fn migrates_owned_legacy_codex_skill_output() {
        let (temp, config) = fixture();
        let root = temp.path();
        apply_plan(root, build_plan(&config, root).unwrap(), None).unwrap();
        let current = root.join(".agents/skills/write-spec");
        let legacy = root.join(".codex/skills/write-spec");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::rename(&current, &legacy).unwrap();

        let lock_path = root.join(LOCKFILE_NAME);
        let mut lock: SkillsLock = serde_yaml::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
        lock.agents
            .iter_mut()
            .find(|agent| agent.id == "codex")
            .unwrap()
            .adapter_version = "1.0.0".to_string();
        let legacy_output = lock
            .outputs
            .iter_mut()
            .find(|output| output.agent == "codex" && output.kind == "skill-link")
            .unwrap();
        legacy_output.path = ".codex/skills/write-spec".to_string();
        legacy_output.claimants.clear();
        fs::write(&lock_path, serde_yaml::to_string(&lock).unwrap()).unwrap();

        let plan = build_plan(&config, root).unwrap();
        assert!(plan
            .public
            .actions
            .iter()
            .any(|action| action.action == "remove" && action.path == ".codex/skills/write-spec"));
        assert!(plan
            .public
            .actions
            .iter()
            .any(|action| action.path == ".agents/skills/write-spec"));
        apply_plan(root, plan, None).unwrap();
        assert!(!legacy.exists() && !legacy.is_symlink());
        assert!(current.is_symlink());
    }

    #[test]
    fn rejects_legacy_codex_skill_claim_from_current_adapter() {
        let (temp, config) = fixture();
        let root = temp.path();
        apply_plan(root, build_plan(&config, root).unwrap(), None).unwrap();
        let lock_path = root.join(LOCKFILE_NAME);
        let mut lock: SkillsLock = serde_yaml::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
        let legacy_output = lock
            .outputs
            .iter_mut()
            .find(|output| output.agent == "codex" && output.kind == "skill-link")
            .unwrap();
        legacy_output.path = ".codex/skills/write-spec".to_string();
        legacy_output.claimants.clear();
        fs::write(&lock_path, serde_yaml::to_string(&lock).unwrap()).unwrap();

        let error = build_plan(&config, root).err().unwrap();
        assert!(error
            .to_string()
            .contains("requires the Codex 1.0.0 adapter"));
    }

    #[test]
    fn accepts_workspace_docs_5_compatible_toolkit() {
        let (temp, config) = fixture();
        let manifest_path = temp
            .path()
            .join("workspace/instructions/toolkit/manifest.yaml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        fs::write(
            manifest_path,
            manifest.replace(
                "workspace_docs_compatibility: 4.x",
                "workspace_docs_compatibility: 5.x",
            ),
        )
        .unwrap();

        assert!(build_plan(&config, temp.path()).is_ok());
    }

    #[test]
    fn installs_workspace_docs_6_toolkit_and_is_byte_idempotent() {
        let (temp, config) = fixture();
        let manifest_path = temp
            .path()
            .join("workspace/instructions/toolkit/manifest.yaml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        fs::write(
            manifest_path,
            manifest.replace(
                "workspace_docs_compatibility: 4.x",
                "workspace_docs_compatibility: 6.x",
            ),
        )
        .unwrap();

        let first = build_plan(&config, temp.path()).unwrap();
        apply_plan(temp.path(), first, None).unwrap();
        check(&config, temp.path()).unwrap();
        let lock_path = temp.path().join(LOCKFILE_NAME);
        let first_lock = fs::read(&lock_path).unwrap();
        let second = build_plan(&config, temp.path()).unwrap();
        assert!(second.public.actions.is_empty());
        apply_plan(temp.path(), second, None).unwrap();
        assert_eq!(fs::read(lock_path).unwrap(), first_lock);
    }

    #[test]
    fn rejects_unsupported_workspace_docs_compatibility() {
        let (temp, config) = fixture();
        let manifest_path = temp
            .path()
            .join("workspace/instructions/toolkit/manifest.yaml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        fs::write(
            manifest_path,
            manifest.replace(
                "workspace_docs_compatibility: 4.x",
                "workspace_docs_compatibility: 7.x",
            ),
        )
        .unwrap();

        let error = build_plan(&config, temp.path()).err().unwrap();
        assert!(error
            .to_string()
            .contains("supported workspace_docs_compatibility: 4.x, 5.x, or 6.x"));
        assert!(!temp.path().join(LOCKFILE_NAME).exists());
        assert!(!temp.path().join(".agents/skills/write-spec").exists());
    }

    #[test]
    fn rejects_missing_and_malformed_workspace_docs_compatibility_before_writes() {
        for replacement in ["", "workspace_docs_compatibility: 6"] {
            let (temp, config) = fixture();
            let manifest_path = temp
                .path()
                .join("workspace/instructions/toolkit/manifest.yaml");
            let manifest = fs::read_to_string(&manifest_path).unwrap();
            fs::write(
                manifest_path,
                manifest.replace("workspace_docs_compatibility: 4.x", replacement),
            )
            .unwrap();

            let error = build_plan(&config, temp.path()).err().unwrap();
            assert!(error
                .to_string()
                .contains("supported workspace_docs_compatibility: 4.x, 5.x, or 6.x"));
            assert!(!temp.path().join(LOCKFILE_NAME).exists());
            assert!(!temp.path().join(".agents/skills/write-spec").exists());
        }
    }

    #[test]
    fn workspace_docs_6_does_not_bypass_minimum_skm_version() {
        let (temp, config) = fixture();
        let manifest_path = temp
            .path()
            .join("workspace/instructions/toolkit/manifest.yaml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        fs::write(
            manifest_path,
            manifest
                .replace(
                    "workspace_docs_compatibility: 4.x",
                    "workspace_docs_compatibility: 6.x",
                )
                .replace("minimum_skm_version: 0.2.0", "minimum_skm_version: 99.0.0"),
        )
        .unwrap();

        let error = build_plan(&config, temp.path()).err().unwrap();
        assert!(error.to_string().contains("toolkit requires SKM 99.0.0"));
        assert!(!temp.path().join(LOCKFILE_NAME).exists());
    }

    #[test]
    fn dry_plan_rejects_unmanaged_collision_before_writes() {
        let (temp, config) = fixture();
        let collision = temp.path().join(".agents/skills/write-spec");
        fs::create_dir_all(&collision).unwrap();
        fs::write(collision.join("keep"), "user data").unwrap();
        let error = build_plan(&config, temp.path()).err().unwrap();
        assert!(error
            .to_string()
            .contains("unmanaged destination collision"));
        assert!(collision.join("keep").exists());
        assert!(!temp.path().join(LOCKFILE_NAME).exists());
    }

    #[test]
    fn rejects_toolkit_source_symlink() {
        let (temp, config) = fixture();
        let source = temp
            .path()
            .join("workspace/instructions/skills/write-spec/SKILL.md");
        fs::remove_file(&source).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(temp.path().join("README.md"), &source).unwrap();
        let error = build_plan(&config, temp.path()).err().unwrap();
        assert!(error.to_string().contains("symlink"));
    }

    #[test]
    fn rolls_back_partial_apply() {
        let (temp, config) = fixture();
        let plan = build_plan(&config, temp.path()).unwrap();
        let error = apply_plan(temp.path(), plan, Some(2)).unwrap_err();
        assert!(error.to_string().contains("rollback succeeded"));
        assert!(!temp.path().join(LOCKFILE_NAME).exists());
        assert!(!temp.path().join(".agents/skills/write-spec").exists());
    }

    #[test]
    fn rollback_preserves_preexisting_empty_adapter_directories() {
        let (temp, config) = fixture();
        fs::create_dir_all(temp.path().join(".agents/skills")).unwrap();
        fs::create_dir_all(temp.path().join(".cursor/skills")).unwrap();
        let plan = build_plan(&config, temp.path()).unwrap();
        let error = apply_plan(temp.path(), plan, Some(2)).unwrap_err();
        assert!(error.to_string().contains("rollback succeeded"));
        assert!(temp.path().join(".agents/skills").is_dir());
        assert!(temp.path().join(".cursor/skills").is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_managed_parent_without_external_writes() {
        let (temp, config) = fixture();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join(".agents")).unwrap();
        let error = build_plan(&config, temp.path()).err().unwrap();
        assert!(error
            .to_string()
            .contains("managed output parent is a symlink"));
        assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
    }

    #[test]
    fn removing_cursor_preserves_codex_outputs() {
        let (temp, mut config) = fixture();
        apply_plan(temp.path(), build_plan(&config, temp.path()).unwrap(), None).unwrap();
        config.agents = vec!["codex".to_string()];
        let removal = build_plan(&config, temp.path()).unwrap();
        assert!(removal
            .public
            .actions
            .iter()
            .any(|action| action.action == "remove" && action.path.starts_with(".cursor/")));
        apply_plan(temp.path(), removal, None).unwrap();
        assert!(temp.path().join(".agents/skills/write-spec").is_symlink());
        assert!(!temp.path().join(".cursor/skills/write-spec").exists());
        assert!(!temp
            .path()
            .join(".skm/generated/profiles/delivery-planner")
            .exists());
    }

    #[test]
    fn rejects_unsafe_output_path_from_lockfile() {
        let (temp, config) = fixture();
        apply_plan(temp.path(), build_plan(&config, temp.path()).unwrap(), None).unwrap();
        let lock_path = temp.path().join(LOCKFILE_NAME);
        let mut lock: SkillsLock = serde_yaml::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
        lock.outputs[0].path = "../outside".to_string();
        fs::write(&lock_path, serde_yaml::to_string(&lock).unwrap()).unwrap();
        let error = build_plan(&config, temp.path()).err().unwrap();
        assert!(error.to_string().contains("unsafe managed output path"));
    }

    #[test]
    fn rejects_lockfile_output_outside_adapter_namespace() {
        let (temp, config) = fixture();
        apply_plan(temp.path(), build_plan(&config, temp.path()).unwrap(), None).unwrap();
        let lock_path = temp.path().join(LOCKFILE_NAME);
        let mut lock: SkillsLock = serde_yaml::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
        lock.outputs[0].path = "src/user-owned.rs".to_string();
        fs::write(&lock_path, serde_yaml::to_string(&lock).unwrap()).unwrap();
        let error = build_plan(&config, temp.path()).err().unwrap();
        assert!(error.to_string().contains("adapter namespace"));
        assert!(!temp.path().join("src/user-owned.rs").exists());
    }

    #[test]
    fn rejects_overlapping_lockfile_outputs() {
        let (temp, config) = fixture();
        apply_plan(temp.path(), build_plan(&config, temp.path()).unwrap(), None).unwrap();
        let lock_path = temp.path().join(LOCKFILE_NAME);
        let mut lock: SkillsLock = serde_yaml::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
        lock.outputs.push(lock.outputs[0].clone());
        fs::write(&lock_path, serde_yaml::to_string(&lock).unwrap()).unwrap();
        let error = build_plan(&config, temp.path()).err().unwrap();
        assert!(error.to_string().contains("overlapping managed output"));
    }

    #[test]
    fn malicious_lock_cannot_claim_and_remove_user_adapter_content() {
        let (temp, config) = fixture();
        apply_plan(temp.path(), build_plan(&config, temp.path()).unwrap(), None).unwrap();
        let user_owned = temp.path().join(".agents/skills/user-owned");
        fs::create_dir_all(&user_owned).unwrap();
        fs::write(user_owned.join("keep.txt"), "keep me\n").unwrap();

        let lock_path = temp.path().join(LOCKFILE_NAME);
        let mut lock: SkillsLock = serde_yaml::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
        let mut claim = lock
            .outputs
            .iter()
            .find(|output| output.kind == "skill-link" && output.agent == "codex")
            .unwrap()
            .clone();
        claim.path = ".agents/skills/user-owned".to_string();
        lock.outputs.push(claim);
        fs::write(&lock_path, serde_yaml::to_string(&lock).unwrap()).unwrap();

        let error = build_plan(&config, temp.path()).err().unwrap();
        assert!(error
            .to_string()
            .contains("drifted from prior lock ownership"));
        assert_eq!(
            fs::read_to_string(user_owned.join("keep.txt")).unwrap(),
            "keep me\n"
        );
    }

    #[test]
    fn check_detects_source_integrity_drift() {
        let (temp, config) = fixture();
        apply_plan(temp.path(), build_plan(&config, temp.path()).unwrap(), None).unwrap();
        fs::write(
            temp.path()
                .join("workspace/instructions/skills/write-spec/SKILL.md"),
            "---\nname: write-spec\ndescription: Changed.\n---\n\n# Changed\n",
        )
        .unwrap();
        let error = check(&config, temp.path()).unwrap_err();
        assert!(error.to_string().contains("drifted"));
    }

    #[test]
    fn rejects_profile_for_unsupported_agent_before_writes() {
        let (temp, mut config) = fixture();
        config.agents = vec!["claude".to_string()];
        let error = build_plan(&config, temp.path()).err().unwrap();
        assert!(error.to_string().contains("no role-profile adapter"));
        assert!(!temp.path().join(LOCKFILE_NAME).exists());
        assert!(!temp.path().join(".claude").exists());
    }

    #[test]
    fn rejects_invalid_skill_frontmatter_before_writes() {
        let (temp, config) = fixture();
        fs::write(
            temp.path()
                .join("workspace/instructions/skills/write-spec/SKILL.md"),
            "# Missing frontmatter\n",
        )
        .unwrap();
        let error = build_plan(&config, temp.path()).err().unwrap();
        assert!(error.to_string().contains("frontmatter"));
        assert!(!temp.path().join(LOCKFILE_NAME).exists());
    }
}
