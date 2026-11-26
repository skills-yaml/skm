use crate::config::SkillSpec;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

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
    #[serde(default)]
    bundle_versions: UniqueMap<UniqueMap<UniqueMap<String>>>,
}

// BTreeMap's ordinary deserializer accepts duplicate YAML keys. Release history
// is an integrity boundary, so reject duplicates at every nested map level.
#[derive(Debug)]
struct UniqueMap<V>(BTreeMap<String, V>);

impl<V> Default for UniqueMap<V> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<V> std::ops::Deref for UniqueMap<V> {
    type Target = BTreeMap<String, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de, V: Deserialize<'de>> Deserialize<'de> for UniqueMap<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor<V>(std::marker::PhantomData<V>);
        impl<'de, V: Deserialize<'de>> Visitor<'de> for UniqueVisitor<V> {
            type Value = UniqueMap<V>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a mapping with unique keys")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = BTreeMap::new();
                while let Some((key, value)) = map.next_entry::<String, V>()? {
                    if values.insert(key.clone(), value).is_some() {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate bundle history key: {key}"
                        )));
                    }
                }
                Ok(UniqueMap(values))
            }
        }
        deserializer.deserialize_map(UniqueVisitor(std::marker::PhantomData))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleMembers {
    packages: Vec<String>,
}

pub(crate) const MAX_NAMESPACE_MANIFEST_SIZE: u64 = 8 * 1024 * 1024;

fn read_manifest(
    root: &Path,
    namespace: &str,
) -> Result<NamespaceManifest, Box<dyn std::error::Error>> {
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
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_NAMESPACE_MANIFEST_SIZE
    {
        return Err("bundle manifest must be a regular file of at most 8 MiB".into());
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
    selector: Option<&str>,
) -> Result<(Option<String>, Vec<SkillSpec>), Box<dyn std::error::Error>> {
    let manifest = read_manifest(root, namespace)?;
    let bundle = manifest.bundles.get(id).ok_or_else(|| {
        format!("Bundle '{namespace}/{id}' is not published by registry '{source}'")
    })?;
    let history = manifest.bundle_versions.get(id);
    let alias = matches!(selector, None | Some("latest") | Some("default"));
    let resolved = if alias {
        history
            .and_then(|history| {
                history
                    .keys()
                    .max_by_key(|v| semver_parts(v).expect("validated release"))
            })
            .cloned()
    } else {
        let selected = selector.expect("exact selector");
        validate_bundle_release(selected)?;
        let history = history.ok_or_else(|| format!("Bundle '{namespace}/{id}' has no published version history; use latest or default or ask the publisher to provide bundle_versions"))?;
        if !history.contains_key(selected) {
            return Err(
                format!("Bundle '{namespace}/{id}' has no published release {selected}").into(),
            );
        }
        Some(selected.to_string())
    };
    let members: BTreeMap<String, String> = match &resolved {
        Some(version) => history.expect("resolved history")[version].0.clone(),
        None => bundle
            .packages
            .iter()
            .map(|member| (member.clone(), manifest.packages[member].clone()))
            .collect(),
    };
    Ok((
        resolved,
        members
            .into_iter()
            .map(|(member, version)| SkillSpec {
                name: format!("{namespace}/{member}"),
                version: Some(version),
                source: Some(source.to_string()),
                path: None,
                kind: None,
            })
            .collect(),
    ))
}

fn validate_bundle_release(version: &str) -> Result<(), Box<dyn std::error::Error>> {
    validate_exact_version(version)?;
    if version.starts_with('v') {
        return Err(format!(
            "Bundle release must be MAJOR.MINOR.PATCH without a v prefix: {version}"
        )
        .into());
    }
    semver_parts(version)?;
    Ok(())
}

fn validate_manifest(
    manifest: &NamespaceManifest,
    namespace: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if manifest.schema_version != 2 || manifest.namespace != namespace {
        return Err("bundle requires a schema-2 manifest matching its namespace".into());
    }
    if manifest.packages.is_empty() || manifest.bundles.is_empty() {
        return Err("bundle manifest must publish packages and bundles".into());
    }
    for (name, version) in &manifest.packages {
        valid_id(name)?;
        validate_skill_name(&format!("{namespace}/{name}"))?;
        validate_exact_version(version)?;
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
    for (id, history) in &manifest.bundle_versions.0 {
        valid_id(id)?;
        let bundle = manifest
            .bundles
            .get(id)
            .ok_or_else(|| format!("Bundle history '{id}' has no current bundle"))?;
        if history.is_empty() {
            return Err(format!("Bundle history '{id}' is empty").into());
        }
        for (release, members) in &history.0 {
            validate_bundle_release(release)?;
            if members.is_empty() {
                return Err(format!("Bundle '{id}' release {release} is empty").into());
            }
            for (member, version) in &members.0 {
                valid_id(member)?;
                validate_skill_name(&format!("{namespace}/{member}"))?;
                validate_exact_version(version)?;
            }
        }
        let latest = history
            .keys()
            .max_by_key(|v| semver_parts(v).expect("validated release"))
            .expect("nonempty history");
        let projection: BTreeMap<_, _> = bundle
            .packages
            .iter()
            .map(|name| (name.clone(), manifest.packages[name].clone()))
            .collect();
        if history[latest].0 != projection {
            return Err(format!(
                "Bundle '{id}' latest release {latest} disagrees with bundles/packages projection"
            )
            .into());
        }
    }
    if let Some(version) = &manifest.toolkit_version {
        validate_exact_version(version)?;
    }
    if let Some(minimum) = &manifest.minimum_skm_version {
        validate_exact_version(minimum)?;
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

fn semver_parts(version: &str) -> Result<(u64, u64, u64), Box<dyn std::error::Error>> {
    let mut parts = version.strip_prefix('v').unwrap_or(version).split('.');
    Ok((
        parts.next().ok_or("missing major version")?.parse()?,
        parts.next().ok_or("missing minor version")?.parse()?,
        parts.next().ok_or("missing patch version")?.parse()?,
    ))
}

fn valid_id(id: &str) -> Result<(), Box<dyn std::error::Error>> {
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

const VERSION_METADATA_KEY: &str = "skm-version";
const DEPENDENCIES_METADATA_KEY: &str = "skm-dependencies";
const MAX_SKILL_DEPENDENCY_DEPTH: usize = 128;

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    #[serde(default)]
    metadata: BTreeMap<String, String>,
}

struct SkillPackageMetadata {
    published_version: Option<String>,
    dependencies: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum UnlinkTargetKind {
    Symlink,
    File,
    Directory,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnlinkTarget {
    pub agent: String,
    pub path: PathBuf,
    pub kind: UnlinkTargetKind,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnlinkFailure {
    pub agent: String,
    pub path: PathBuf,
    pub error: String,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct UnlinkResult {
    pub removed: Vec<PathBuf>,
    pub failures: Vec<UnlinkFailure>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AgentSkillTarget {
    pub path: PathBuf,
    pub agents: Vec<String>,
}

/// A resolved manifest entry. Bundle entries carry their expanded members so
/// the lockfile can record both the bundle declaration and its resolution.
#[derive(Debug, Clone)]
pub enum ResolvedEntry {
    Skill(SkillSpec),
    Bundle {
        spec: SkillSpec,
        members: Vec<SkillSpec>,
        resolved_version: Option<String>,
    },
}

impl ResolvedEntry {
    /// Flatten to the concrete skill specs that need linking.
    pub fn linkable_skills(&self) -> Vec<SkillSpec> {
        match self {
            ResolvedEntry::Skill(spec) => vec![spec.clone()],
            ResolvedEntry::Bundle { members, .. } => members.clone(),
        }
    }
}

pub const SUPPORTED_AGENTS: &[&str] = &[
    "claude",
    "codex",
    "copilot",
    "cursor",
    "antigravity",
    "pi",
    "opencode",
    "cline",
    "kilo",
    "gemini-cli",
    "goose",
    "crush",
    "openhands",
    "grok",
    "qwen",
    "hermes",
];

fn agent_skill_paths(agent: &str, global: bool) -> Option<&'static [&'static str]> {
    let paths: &[&str] = match (agent, global) {
        ("claude", _) => &[".claude/skills"],
        ("codex", _) => &[".agents/skills"],
        ("copilot", false) => &[".github/skills"],
        ("copilot", true) => &[".copilot/skills"],
        ("cursor", _) => &[".cursor/skills"],
        ("antigravity", false) => &[".agents/skills"],
        ("antigravity", true) => &[".gemini/config/skills"],
        ("pi", false) => &[".pi/skills"],
        ("pi", true) => &[".pi/agent/skills"],
        ("opencode", false) => &[".opencode/skills"],
        ("opencode", true) => &[".config/opencode/skills"],
        ("cline", _) => &[".cline/skills"],
        ("kilo", _) => &[".kilo/skills"],
        ("gemini-cli", _) => &[".gemini/skills"],
        ("goose", _) => &[".agents/skills"],
        ("crush", false) => &[".crush/skills"],
        ("crush", true) => &[".config/crush/skills"],
        ("openhands", false) => &[".agents/skills"],
        ("openhands", true) => &[".openhands/skills"],
        ("grok", _) => &[".grok/skills"],
        ("qwen", _) => &[".qwen/skills"],
        ("hermes", false) => &[],
        ("hermes", true) => &[".hermes/skills"],
        _ => return None,
    };
    Some(paths)
}

pub(crate) fn project_agent_skill_paths(agent: &str) -> Option<&'static [&'static str]> {
    agent_skill_paths(agent, false)
}

pub fn get_global_agent_skills_dirs(agent: &str) -> Option<Vec<PathBuf>> {
    let home = dirs::home_dir()?;
    agent_skill_paths(agent, true).map(|paths| paths.iter().map(|path| home.join(path)).collect())
}

pub fn get_project_agent_skills_dirs(agent: &str, project_root: &Path) -> Option<Vec<PathBuf>> {
    agent_skill_paths(agent, false)
        .map(|paths| paths.iter().map(|path| project_root.join(path)).collect())
}

pub fn resolve_agent_skill_targets(
    agents: &[String],
    project_root: &Path,
    global: bool,
) -> Result<Vec<AgentSkillTarget>, Box<dyn std::error::Error>> {
    validate_agents(agents)?;
    let mut targets: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for agent in agents {
        let paths = if global {
            get_global_agent_skills_dirs(agent)
                .ok_or_else(|| format!("Could not determine skills directory for '{agent}'"))?
        } else {
            get_project_agent_skills_dirs(agent, project_root)
                .ok_or_else(|| format!("Could not determine skills directory for '{agent}'"))?
        };
        for path in paths {
            let claimants = targets.entry(path).or_default();
            if !claimants.contains(agent) {
                claimants.push(agent.clone());
            }
        }
    }
    Ok(targets
        .into_iter()
        .map(|(path, agents)| AgentSkillTarget { path, agents })
        .collect())
}

/// Resolve the complete, exact dependency closure for configured skills.
///
/// Registry packages declare same-registry dependencies through the
/// string-valued Agent Skills metadata key `skm-dependencies`. Each dependency
/// uses `namespace/name@MAJOR.MINOR.PATCH`; comma-separated entries are
/// supported. Packages can declare their exact published version through
/// `skm-version`, allowing `latest` and `default` selections to become exact in
/// lockfiles and dependency comparisons.
pub fn resolve_skill_dependency_closure(
    skills: &[SkillSpec],
    project_root: &Path,
) -> Result<Vec<SkillSpec>, Box<dyn std::error::Error>> {
    let entries = resolve_entries(skills, project_root)?;
    Ok(linkable_entries(&entries))
}

/// Resolve manifest entries, expanding `kind: bundle` entries into their
/// published members plus transitive dependencies. Returns one ResolvedEntry
/// per manifest entry; bundle entries carry their resolved members.
pub fn resolve_entries(
    skills: &[SkillSpec],
    project_root: &Path,
) -> Result<Vec<ResolvedEntry>, Box<dyn std::error::Error>> {
    resolve_entries_with(skills, project_root, resolve_skill_source_dir)
}

pub(crate) fn resolve_entries_with<F>(
    skills: &[SkillSpec],
    project_root: &Path,
    resolve_source: F,
) -> Result<Vec<ResolvedEntry>, Box<dyn std::error::Error>>
where
    F: Fn(&SkillSpec, &Path) -> Result<PathBuf, Box<dyn std::error::Error>> + Clone,
{
    let mut entries = Vec::new();
    let mut all_specs = Vec::new();
    for skill in skills {
        skill.validate_kind()?;
        if skill.is_bundle() {
            let (resolved_version, direct) =
                resolve_bundle_members_with(skill, project_root, &resolve_source)?;
            let members =
                resolve_skill_dependency_closure_with(&direct, project_root, &resolve_source)?;
            all_specs.extend(members.iter().cloned());
            entries.push(ResolvedEntry::Bundle {
                spec: skill.clone(),
                members,
                resolved_version,
            });
        } else {
            let closure = resolve_skill_dependency_closure_with(
                std::slice::from_ref(skill),
                project_root,
                &resolve_source,
            )?;
            all_specs.extend(closure);
            entries.push(ResolvedEntry::Skill(skill.clone()));
        }
    }
    // One global pass detects incompatible overlapping requests across entries.
    let resolved = resolve_skill_dependency_closure_with(&all_specs, project_root, resolve_source)?;
    let resolved_map: BTreeMap<_, _> = resolved.into_iter().map(|s| (s.name.clone(), s)).collect();
    let mut included = BTreeSet::new();
    for entry in &mut entries {
        match entry {
            ResolvedEntry::Skill(spec) => {
                *spec = resolved_map[&spec.name].clone();
                included.insert(spec.name.clone());
            }
            ResolvedEntry::Bundle { members, .. } => {
                included.extend(members.iter().map(|s| s.name.clone()));
            }
        }
    }
    // Preserve standalone transitive dependencies without assigning them to a bundle.
    for (name, spec) in resolved_map {
        if !included.contains(&name) {
            entries.push(ResolvedEntry::Skill(spec));
        }
    }
    Ok(entries)
}

/// Flatten entries into a deterministic set of linkable skills.
pub fn linkable_entries(entries: &[ResolvedEntry]) -> Vec<SkillSpec> {
    entries
        .iter()
        .flat_map(ResolvedEntry::linkable_skills)
        .map(|skill| (skill.name.clone(), skill))
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect()
}

/// Expand a `kind: bundle` entry into its published member SkillSpecs by
/// reading the registry namespace manifest.
fn resolve_bundle_members_with<F>(
    bundle: &SkillSpec,
    project_root: &Path,
    _resolve_source: F,
) -> Result<(Option<String>, Vec<SkillSpec>), Box<dyn std::error::Error>>
where
    F: Fn(&SkillSpec, &Path) -> Result<PathBuf, Box<dyn std::error::Error>>,
{
    validate_skill_name(&bundle.name)?;
    if bundle.path.is_some() {
        return Err(format!(
            "Bundle '{}' cannot use a local path; bundles are registry-only",
            bundle.name
        )
        .into());
    }
    let source = bundle
        .source
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("Bundle requires an explicit registry source")?;
    let (namespace, id) = bundle
        .name
        .split_once('/')
        .ok_or_else(|| format!("Bundle ID must be namespace/name: {}", bundle.name))?;
    if id.contains('/') {
        return Err(format!("Bundle ID must be namespace/name: {}", bundle.name).into());
    }
    let mut registry_root = effective_registry_path(source, project_root)?;
    if !registry_root.is_dir() {
        let location = effective_registry_location(source, project_root)?;
        let local = project_root.join(location);
        if local.is_dir() {
            registry_root = local;
        }
    }
    if !registry_root.is_dir() {
        return Err(format!(
            "Could not resolve registry '{source}' for bundle '{}'",
            bundle.name
        )
        .into());
    }
    published_members(
        &registry_root,
        namespace,
        id,
        source,
        bundle.version.as_deref(),
    )
}

pub(crate) fn resolve_skill_dependency_closure_with<F>(
    skills: &[SkillSpec],
    project_root: &Path,
    resolve_source: F,
) -> Result<Vec<SkillSpec>, Box<dyn std::error::Error>>
where
    F: Fn(&SkillSpec, &Path) -> Result<PathBuf, Box<dyn std::error::Error>>,
{
    let mut resolved = BTreeMap::new();
    let mut active = BTreeMap::new();
    let mut stack = Vec::new();
    for skill in skills {
        resolve_skill_dependency(
            skill,
            project_root,
            &resolve_source,
            &mut resolved,
            &mut active,
            &mut stack,
        )?;
    }
    Ok(resolved.into_values().collect())
}

fn resolve_skill_dependency<F>(
    requested: &SkillSpec,
    project_root: &Path,
    resolve_source: &F,
    resolved: &mut BTreeMap<String, SkillSpec>,
    active: &mut BTreeMap<String, SkillSpec>,
    stack: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&SkillSpec, &Path) -> Result<PathBuf, Box<dyn std::error::Error>>,
{
    if stack.len() >= MAX_SKILL_DEPENDENCY_DEPTH {
        return Err(format!(
            "registry skill dependency depth exceeds {MAX_SKILL_DEPENDENCY_DEPTH}"
        )
        .into());
    }
    validate_skill_name(&requested.name)?;
    let source = resolve_source(requested, project_root)?;
    if !source.is_dir() {
        return Err(format!("Skill source path does not exist: {}", source.display()).into());
    }
    let metadata = read_skill_package_metadata(&source, &requested.name)?;
    let normalized = normalize_published_skill(requested, metadata.published_version.as_deref())?;

    if let Some(existing) = resolved.get(&normalized.name) {
        return ensure_compatible_skill_requests(existing, &normalized);
    }
    if let Some(existing) = active.get(&normalized.name) {
        ensure_compatible_skill_requests(existing, &normalized)?;
        stack.push(normalized.name.clone());
        let cycle = stack.join(" -> ");
        stack.pop();
        return Err(format!("registry skill dependency cycle: {cycle}").into());
    }

    active.insert(normalized.name.clone(), normalized.clone());
    stack.push(normalized.name.clone());
    for (dependency_name, dependency_version) in metadata.dependencies {
        if normalized.path.is_some() {
            return Err(format!(
                "local skill {} cannot declare registry dependencies",
                normalized.name
            )
            .into());
        }
        let dependency = SkillSpec {
            name: dependency_name,
            version: Some(dependency_version),
            source: normalized.source.clone(),
            path: None,
            kind: None,
        };
        resolve_skill_dependency(
            &dependency,
            project_root,
            resolve_source,
            resolved,
            active,
            stack,
        )?;
    }
    stack.pop();
    active.remove(&normalized.name);
    resolved.insert(normalized.name.clone(), normalized);
    Ok(())
}

fn read_skill_package_metadata(
    source: &Path,
    requested_name: &str,
) -> Result<SkillPackageMetadata, Box<dyn std::error::Error>> {
    let skill_path = source.join("SKILL.md");
    let content = fs::read_to_string(&skill_path)?;
    let Some(rest) = content.strip_prefix("---\n") else {
        return Err(format!("{} has invalid SKILL.md frontmatter", source.display()).into());
    };
    let Some((frontmatter, _)) = rest.split_once("\n---") else {
        return Err(format!("{} has unclosed SKILL.md frontmatter", source.display()).into());
    };
    let metadata: SkillFrontmatter = serde_yaml::from_str(frontmatter)?;
    let expected_name = requested_name
        .rsplit('/')
        .next()
        .ok_or("skill name has no final component")?;
    if metadata.name != expected_name {
        return Err(format!(
            "skill identity mismatch: requested {requested_name}, package declares {}",
            metadata.name
        )
        .into());
    }

    let published_version = metadata.metadata.get(VERSION_METADATA_KEY).cloned();
    if let Some(version) = &published_version {
        validate_exact_version(version)?;
    }
    let dependencies = parse_skill_dependencies(
        metadata
            .metadata
            .get(DEPENDENCIES_METADATA_KEY)
            .map(String::as_str),
    )?;
    Ok(SkillPackageMetadata {
        published_version,
        dependencies,
    })
}

fn normalize_published_skill(
    requested: &SkillSpec,
    published_version: Option<&str>,
) -> Result<SkillSpec, Box<dyn std::error::Error>> {
    let requested_version = requested.version.as_deref().unwrap_or("latest");
    let version = if let Some(published) = published_version {
        let requested_exact = requested_version.trim_start_matches('v');
        if !matches!(requested_version, "latest" | "default") && requested_exact != published {
            return Err(format!(
                "skill {} requested version {}, package declares {}",
                requested.name, requested_version, published
            )
            .into());
        }
        published.to_string()
    } else {
        requested_version.to_string()
    };
    Ok(SkillSpec {
        name: requested.name.clone(),
        version: Some(version),
        source: requested.source.clone(),
        path: requested.path.clone(),
        kind: None,
    })
}

pub(crate) fn parse_skill_dependencies(
    raw: Option<&str>,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let mut dependencies = Vec::new();
    let mut names = BTreeSet::new();
    for value in raw.split(',').map(str::trim) {
        if value.is_empty() {
            return Err("skm-dependencies contains an empty entry".into());
        }
        let (name, version) = value
            .rsplit_once('@')
            .ok_or_else(|| format!("invalid registry skill dependency: {value}"))?;
        validate_skill_name(name)?;
        validate_exact_version(version)?;
        if !names.insert(name.to_string()) {
            return Err(format!("duplicate registry skill dependency: {name}").into());
        }
        dependencies.push((name.to_string(), version.to_string()));
    }
    Ok(dependencies)
}

pub(crate) fn validate_exact_version(version: &str) -> Result<(), Box<dyn std::error::Error>> {
    let version = version.strip_prefix('v').unwrap_or(version);
    let parts: Vec<_> = version.split('.').collect();
    let valid = parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
        });
    if !valid {
        return Err(
            format!("dependency version must be exact semantic versioning: {version}").into(),
        );
    }
    Ok(())
}

fn ensure_compatible_skill_requests(
    existing: &SkillSpec,
    requested: &SkillSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    let existing_source = existing.source.as_deref().unwrap_or("default");
    let requested_source = requested.source.as_deref().unwrap_or("default");
    if existing.version != requested.version
        || existing_source != requested_source
        || existing.path != requested.path
    {
        return Err(format!(
            "conflicting registry skill requirements for {}: {} from {} versus {} from {}",
            requested.name,
            existing.version.as_deref().unwrap_or("latest"),
            existing_source,
            requested.version.as_deref().unwrap_or("latest"),
            requested_source
        )
        .into());
    }
    Ok(())
}

pub fn resolve_registry_path(name: &str) -> Option<PathBuf> {
    if !is_safe_registry_name(name) {
        return None;
    }
    canonical_registry_path(name)
}

/// Destination for canonical cache writes.
pub(crate) fn canonical_registry_path(name: &str) -> Option<PathBuf> {
    if !is_safe_registry_name(name) {
        return None;
    }
    Some(
        crate::config_manager::get_user_cache_dir()?
            .join("registries")
            .join(name),
    )
}

pub(crate) fn is_git_repository(path: &Path) -> bool {
    path.join(".git").exists() || (path.join("HEAD").is_file() && path.join("objects").is_dir())
}

/// Check the effective Git fetch origin without creating temporary files or fetching.
pub(crate) fn registry_cache_matches(cache: &Path, location: &str, project: &Path) -> bool {
    let local = project.join(location);
    let canonical_local = fs::canonicalize(&local).ok();
    if canonical_local
        .as_ref()
        .is_some_and(|local| fs::canonicalize(cache).ok().as_ref() == Some(local))
    {
        return true;
    }
    if !cache.is_dir() || !is_git_repository(cache) {
        return false;
    }
    let origin = std::process::Command::new("git")
        .arg("-C")
        .arg(cache)
        .args(["remote", "get-url", "origin"])
        .output();
    let remotes = std::process::Command::new("git")
        .arg("-C")
        .arg(cache)
        .arg("remote")
        .output();
    let (Ok(origin), Ok(remotes)) = (origin, remotes) else {
        return false;
    };
    if !origin.status.success() || !remotes.status.success() {
        return false;
    }
    let names = String::from_utf8_lossy(&remotes.stdout);
    let mut name = "skm-url-validation".to_owned();
    while names.lines().any(|line| line == name) {
        name.push('-');
    }
    // Ask Git to resolve the literal first so repository-local URL rewrites
    // retain their normal semantics, even for a bare name without a colon.
    let effective_url = |value: &str| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(cache)
            .arg("-c")
            .arg(format!("remote.{name}.url={value}"))
            .args(["ls-remote", "--get-url", "--", &name])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| output.stdout)
    };
    let Some(literal) = effective_url(location) else {
        return false;
    };
    if literal == origin.stdout {
        return true;
    }
    let mut unrewritten = location.as_bytes().to_vec();
    unrewritten.push(b'\n');
    // Do not bypass a rewrite that resolved to a different source. For an
    // unrewritten local path Git stores an absolute clone URL, retaining `..`
    // components even after the original directory has disappeared.
    if literal != unrewritten
        || !(canonical_local.is_some() || (!location.contains("://") && !location.contains(':')))
    {
        return false;
    }
    let root = if project.is_absolute() {
        project.to_path_buf()
    } else if let Ok(current) = std::env::current_dir() {
        current.join(project)
    } else {
        return false;
    };
    let Some(local_spelling) = root.join(location).to_str().map(str::to_owned) else {
        return false;
    };
    effective_url(&local_spelling).is_some_and(|expected| expected == origin.stdout)
}

/// Select and validate an existing cache before consuming it. Missing caches
/// resolve to the canonical destination for a caller that will populate it.
pub(crate) fn validated_registry_path(
    name: &str,
    location: &str,
    project: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let canonical =
        canonical_registry_path(name).ok_or_else(|| format!("Invalid registry name: {name}"))?;
    let home = dirs::home_dir().ok_or("Could not determine user home directory")?;
    let mut candidates = vec![
        canonical.clone(),
        home.join(".cache/skm/registries").join(name),
    ];
    if let Some(cache) = dirs::cache_dir() {
        let legacy = cache.join("skm/registries").join(name);
        if !candidates.contains(&legacy) {
            candidates.push(legacy);
        }
    }
    for cache in candidates {
        match fs::symlink_metadata(&cache) {
            Ok(_) if !registry_cache_matches(&cache, location, project) => {
                return Err(format!("Cached registry '{name}' at {} has a different or missing origin; choose a distinct registry alias", cache.display()).into());
            }
            Ok(_) => return Ok(cache),
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(
                    format!("Cannot inspect registry cache {}: {error}", cache.display()).into(),
                )
            }
        }
    }
    Ok(canonical)
}

/// Resolve an effective registry from a manifest directory and validate its cache.
/// Explicit callers can use `validated_registry_path` with a selected URL directly.
pub(crate) fn effective_registry_path(
    name: &str,
    manifest_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let url = effective_registry_location(name, manifest_root)?;
    validated_registry_path(name, &url, manifest_root)
}

fn effective_registry_location(
    name: &str,
    manifest_root: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let manifest = manifest_root.join("skills.yaml");
    let override_url = if manifest.exists() {
        crate::config::SkillsConfig::load_from_file(&manifest)?
            .registries
            .and_then(|registries| registries.get(name).cloned())
    } else {
        None
    };
    let url = match override_url {
        Some(url) => url,
        None => crate::config_manager::BaseConfig::load()?
            .registries
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Registry '{name}' is not configured"))?,
    };
    Ok(url)
}

/// Recognize a locked exact pin in the selected cache or a validated retained
/// cache. This is for ownership-sensitive removal only, not health checks.
pub(crate) fn locked_registry_link_matches(
    link: &Path,
    skill: &SkillSpec,
    project: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    let name = skill
        .source
        .as_deref()
        .ok_or("Locked registry skill has no source")?;
    let version = skill
        .version
        .as_deref()
        .ok_or("Locked registry skill has no version")?;
    validate_exact_version(version)?;
    let location = effective_registry_location(name, project)?;
    let selected = validated_registry_path(name, &location, project)?;
    let home = dirs::home_dir().ok_or("Could not determine user home directory")?;
    let mut roots = vec![selected, home.join(".cache/skm/registries").join(name)];
    if let Some(cache) = dirs::cache_dir() {
        let legacy = cache.join("skm/registries").join(name);
        if !roots.contains(&legacy) {
            roots.push(legacy);
        }
    }
    for root in roots {
        let expected = root
            .join("skills")
            .join(validated_skill_path(&skill.name)?)
            .join(format!("v{}", version.trim_start_matches('v')));
        if !symlink_matches_expected(link, &expected)? {
            continue;
        }
        if !registry_cache_matches(&root, &location, project) {
            return Err(format!("Cached registry '{name}' at {} has a different or missing origin; choose a distinct registry alias", root.display()).into());
        }
        if expected.exists() {
            resolve_registry_skill_source_dir(skill, &root)?;
        }
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn validate_registry_cache_origin(
    name: &str,
    location: &str,
    project: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    validated_registry_path(name, location, project).map(|_| ())
}

pub fn validate_agents(agents: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    for agent in agents {
        if !is_supported_agent(agent) {
            return Err(format!(
                "Unsupported agent '{}'. Supported agents: {}",
                agent,
                SUPPORTED_AGENTS.join(", ")
            )
            .into());
        }
    }

    Ok(())
}

pub fn validate_skill_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    validated_skill_path(name).map(|_| ())
}

pub fn resolve_skill_source_dir(
    skill: &SkillSpec,
    project_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_skill_name(&skill.name)?;

    if let Some(ref local_path) = skill.path {
        Ok(project_root.join(local_path))
    } else {
        let registry_name = skill.source.as_deref().unwrap_or("default");
        let reg_path = effective_registry_path(registry_name, project_root)?;

        resolve_registry_skill_source_dir(skill, &reg_path)
    }
}

pub(crate) fn resolve_registry_skill_source_dir(
    skill: &SkillSpec,
    registry_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let skill_path = validated_skill_path(&skill.name)?;
    let package_root = registry_root.join("skills").join(&skill_path);
    validate_registry_package_root(registry_root, &skill_path, &package_root)?;
    let version_path = resolve_version_path(skill)?;
    let source = package_root.join(&version_path);
    validate_registry_version_path(&package_root, &source, &version_path)?;
    Ok(source)
}

fn validate_registry_package_root(
    registry_root: &Path,
    skill_path: &Path,
    package_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut current = registry_root.join("skills");
    for component in skill_path.components() {
        let Component::Normal(part) = component else {
            return Err("registry skill path is invalid".into());
        };
        current.push(part);
        let metadata = fs::symlink_metadata(&current)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(format!(
                "registry skill package path must contain only real directories: {}",
                package_root.display()
            )
            .into());
        }
    }
    Ok(())
}

fn validate_registry_version_path(
    package_root: &Path,
    source: &Path,
    version_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let version = version_path
        .to_str()
        .ok_or("registry skill version path contains non-UTF-8 data")?;
    let metadata = fs::symlink_metadata(source)?;
    let alias = matches!(version, "latest" | "default");
    if alias != metadata.file_type().is_symlink() {
        return Err(format!(
            "registry skill version '{}' must {}be a symlink",
            version,
            if alias { "" } else { "not " }
        )
        .into());
    }
    let canonical_package = fs::canonicalize(package_root)?;
    let canonical_source = fs::canonicalize(source)?;
    if !canonical_source.is_dir() || canonical_source.parent() != Some(&canonical_package) {
        return Err(format!(
            "registry skill version escapes its package: {}",
            source.display()
        )
        .into());
    }
    let resolved_version = canonical_source
        .file_name()
        .and_then(|value| value.to_str())
        .and_then(|value| value.strip_prefix('v'))
        .ok_or("registry skill version target must use vMAJOR.MINOR.PATCH")?;
    validate_exact_version(resolved_version)?;
    if !alias && version.strip_prefix('v') != Some(resolved_version) {
        return Err("registry skill version path does not match its resolved version".into());
    }
    Ok(())
}

pub(crate) fn parse_registry_lock_target(
    target: &str,
) -> Result<SkillSpec, Box<dyn std::error::Error>> {
    let value = target
        .strip_prefix("registry:")
        .ok_or("registry lock target has an invalid prefix")?;
    let (source, package) = value
        .split_once('/')
        .ok_or("registry lock target is missing its source")?;
    if !is_safe_registry_name(source) {
        return Err("registry lock target has an invalid source".into());
    }
    let (name, version) = package
        .rsplit_once('@')
        .ok_or("registry lock target is missing its exact version")?;
    validate_skill_name(name)?;
    validate_exact_version(version)?;
    Ok(SkillSpec {
        name: name.to_string(),
        version: Some(version.to_string()),
        source: Some(source.to_string()),
        path: None,
        kind: None,
    })
}

/// Resolves the version component of a skill path.
///
/// Version resolution order:
/// 1. If version is "latest" → use "latest" (follows symlink)
/// 2. If version is "default" → use "default" (follows symlink)
/// 3. If version is a semantic version (e.g., "1.2.3") → use "v1.2.3"
/// 4. If version is None → use "latest"
pub fn resolve_version_path(skill: &SkillSpec) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let version = skill.version.as_deref().unwrap_or("latest");

    // Normalize version to path component
    let version_path = if version == "latest" || version == "default" {
        // Use as-is, will follow symlink
        PathBuf::from(version)
    } else {
        // Prepend "v" for semantic versions
        if version.starts_with('v') {
            PathBuf::from(version)
        } else {
            PathBuf::from(format!("v{}", version))
        }
    };

    // Validate version path is safe
    if !is_safe_version_path(&version_path) {
        return Err(format!("Invalid version path: {}", version_path.display()).into());
    }

    Ok(version_path)
}

/// Validates that a version path component is safe (no path traversal).
fn is_safe_version_path(path: &Path) -> bool {
    path.components().all(|c| matches!(c, Component::Normal(_)))
}

pub fn get_skill_target_path(
    base_dir: &Path,
    skill_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let validated = validated_skill_path(skill_name)?;
    let leaf = validated
        .file_name()
        .ok_or("skill name has no final component")?;
    Ok(base_dir.join(leaf))
}

pub fn validate_unique_skill_targets(
    skills: &[SkillSpec],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut targets = BTreeMap::new();
    for skill in skills {
        let path = validated_skill_path(&skill.name)?;
        let leaf = path
            .file_name()
            .ok_or("skill name has no final component")?
            .to_string_lossy()
            .into_owned();
        if let Some(previous) = targets.insert(leaf.clone(), skill.name.as_str()) {
            if previous != skill.name {
                return Err(format!(
                    "Skills '{previous}' and '{}' both install as '{leaf}'; choose only one",
                    skill.name
                )
                .into());
            }
        }
    }
    Ok(())
}

/// Validate directly linkable configured entries; bundles have no own output.
pub fn validate_configured_skill_targets(
    skills: &[SkillSpec],
) -> Result<(), Box<dyn std::error::Error>> {
    let concrete: Vec<_> = skills
        .iter()
        .filter(|skill| !skill.is_bundle())
        .cloned()
        .collect();
    validate_unique_skill_targets(&concrete)
}

pub fn require_skill_targets(
    agents: &[String],
    project_root: &Path,
    global: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if resolve_agent_skill_targets(agents, project_root, global)?.is_empty() {
        return Err(if agents.is_empty() {
            "No agent skill targets configured. Add a supported agent to skills.yaml agents before installing skills."
        } else {
            "No project skill targets for the selected agents. Hermes supports global skills only; select another agent or use skm install --global."
        }
        .into());
    }
    Ok(())
}

pub(crate) fn validate_skill_target_parent(
    base_dir: &Path,
    skill_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let relative = validated_skill_path(skill_name)?;
    let components: Vec<_> = relative.components().collect();
    let mut current = base_dir.to_path_buf();

    for component in components.iter().take(components.len().saturating_sub(1)) {
        let Component::Normal(part) = component else {
            return Err(format!("Invalid skill name '{}'", skill_name).into());
        };
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "Refusing to use symlinked skill namespace: {}",
                    current.display()
                )
                .into());
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(format!(
                    "Refusing to use non-directory skill namespace: {}",
                    current.display()
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

pub fn symlink_points_to(
    link_path: &Path,
    expected_target: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    let actual_target = fs::read_link(link_path)?;
    let actual_target = if actual_target.is_absolute() {
        actual_target
    } else {
        link_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(actual_target)
    };

    let Ok(actual_target) = fs::canonicalize(actual_target) else {
        return Ok(false);
    };
    let expected_target = fs::canonicalize(expected_target)?;

    Ok(actual_target == expected_target)
}

pub fn link_skill(
    skill: &SkillSpec,
    project_root: &Path,
    agents: &[String],
    global: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_agents(agents)?;
    let source_dir = resolve_skill_source_dir(skill, project_root)?;

    if !source_dir.exists() {
        return Err(format!("Skill source path does not exist: {:?}", source_dir).into());
    }

    if !source_dir.join("SKILL.md").exists() {
        return Err(format!("Missing SKILL.md in: {:?}", source_dir).into());
    }

    let targets = resolve_agent_skill_targets(agents, project_root, global)?;
    for target in &targets {
        let base_dir = &target.path;
        validate_skill_target_parent(base_dir, &skill.name)?;
        let skill_target = get_skill_target_path(base_dir, &skill.name)?;

        match fs::symlink_metadata(&skill_target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(format!(
                    "Refusing to replace existing non-symlink path: {:?}",
                    skill_target
                )
                .into());
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }

    for target in targets {
        let base_dir = target.path;
        let skill_target = get_skill_target_path(&base_dir, &skill.name)?;

        if let Some(parent) = skill_target.parent() {
            fs::create_dir_all(parent)?;
        }

        match fs::symlink_metadata(&skill_target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                if symlink_points_to(&skill_target, &source_dir)? {
                    eprintln!("Already linked {} to {:?}", skill.name, skill_target);
                    continue;
                }

                fs::remove_file(&skill_target)?;
            }
            Ok(_) => {
                return Err(format!(
                    "Refusing to replace existing non-symlink path: {:?}",
                    skill_target
                )
                .into());
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }

        symlink_dir(&source_dir, &skill_target)?;
        eprintln!("Linked {} to {:?}", skill.name, skill_target);
    }

    Ok(())
}
/// Resolve and validate every configured target before removal.
pub fn plan_skill_unlink(
    skill: &SkillSpec,
    project_root: &Path,
    agents: &[String],
    global: bool,
    force: bool,
    verbose: bool,
) -> Result<Vec<UnlinkTarget>, Box<dyn std::error::Error>> {
    let expected_source = resolve_skill_source_dir(skill, project_root)?;
    plan_skill_unlink_expected(
        skill,
        project_root,
        agents,
        global,
        force,
        verbose,
        &expected_source,
        false,
    )
}

/// Plan against the installed pin even if its registry version disappeared.
pub(crate) fn plan_locked_skill_unlink(
    skill: &SkillSpec,
    project_root: &Path,
    agents: &[String],
    force: bool,
    verbose: bool,
) -> Result<Vec<UnlinkTarget>, Box<dyn std::error::Error>> {
    let expected = if let Some(path) = &skill.path {
        project_root.join(path)
    } else {
        let source = skill
            .source
            .as_deref()
            .ok_or("Locked registry skill has no source")?;
        let version = skill
            .version
            .as_deref()
            .ok_or("Locked registry skill has no version")?;
        if matches!(version, "latest" | "default") {
            resolve_skill_source_dir(skill, project_root)?
        } else {
            validate_exact_version(version)?;
            effective_registry_path(source, project_root)?
                .join("skills")
                .join(validated_skill_path(&skill.name)?)
                .join(format!("v{}", version.trim_start_matches('v')))
        }
    };
    plan_skill_unlink_expected(
        skill,
        project_root,
        agents,
        false,
        force,
        verbose,
        &expected,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn plan_skill_unlink_expected(
    skill: &SkillSpec,
    project_root: &Path,
    agents: &[String],
    global: bool,
    force: bool,
    verbose: bool,
    expected_source: &Path,
    locked: bool,
) -> Result<Vec<UnlinkTarget>, Box<dyn std::error::Error>> {
    validate_agents(agents)?;
    let mut targets = Vec::new();

    for target in resolve_agent_skill_targets(agents, project_root, global)? {
        let agent = target.agents.join(", ");
        let base_dir = target.path;
        validate_skill_target_parent(&base_dir, &skill.name)?;
        let skill_path = get_skill_target_path(&base_dir, &skill.name)?;

        match fs::symlink_metadata(&skill_path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let matches = if force {
                    true
                } else if locked
                    && skill.path.is_none()
                    && !matches!(skill.version.as_deref(), None | Some("latest" | "default"))
                {
                    locked_registry_link_matches(&skill_path, skill, project_root)?
                } else {
                    symlink_matches_expected(&skill_path, expected_source)?
                };
                if !force && !matches {
                    return Err(format!(
                        "Refusing to remove unexpected symlink for agent '{}': {} does not point to {}. Use --force to override this safety check.",
                        agent,
                        skill_path.display(),
                        expected_source.display()
                    )
                    .into());
                }
                targets.push(UnlinkTarget {
                    agent: agent.clone(),
                    path: skill_path,
                    kind: UnlinkTargetKind::Symlink,
                });
            }
            Ok(metadata) if force => {
                targets.push(UnlinkTarget {
                    agent: agent.clone(),
                    path: skill_path,
                    kind: if metadata.is_dir() {
                        UnlinkTargetKind::Directory
                    } else {
                        UnlinkTargetKind::File
                    },
                });
            }
            Ok(_) => {
                return Err(format!(
                    "Refusing to remove non-symlink path for agent '{}': {}. Use --force to override this safety check.",
                    agent,
                    skill_path.display()
                )
                .into());
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if verbose {
                    eprintln!(
                        "Skill target already missing for agent '{}': {}",
                        agent,
                        skill_path.display()
                    );
                }
            }
            Err(error) => return Err(error.into()),
        }
    }

    Ok(targets)
}

/// Remove all preflighted targets, continuing after per-target failures.
pub fn apply_skill_unlink(targets: &[UnlinkTarget]) -> UnlinkResult {
    let mut result = UnlinkResult::default();

    for target in targets {
        let removal = match fs::symlink_metadata(&target.path) {
            Ok(metadata)
                if target.kind == UnlinkTargetKind::Symlink
                    && !metadata.file_type().is_symlink() =>
            {
                Err(io::Error::other(
                    "target changed after preflight and is no longer a symlink",
                ))
            }
            Ok(metadata) if target.kind == UnlinkTargetKind::Directory && !metadata.is_dir() => {
                Err(io::Error::other(
                    "target changed after preflight and is no longer a directory",
                ))
            }
            Ok(metadata)
                if target.kind == UnlinkTargetKind::File
                    && (metadata.is_dir() || metadata.file_type().is_symlink()) =>
            {
                Err(io::Error::other(
                    "target changed after preflight and is no longer a file",
                ))
            }
            Ok(_) if target.kind == UnlinkTargetKind::Directory => fs::remove_dir_all(&target.path),
            Ok(_) => fs::remove_file(&target.path),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        };

        match removal {
            Ok(()) => result.removed.push(target.path.clone()),
            Err(error) => result.failures.push(UnlinkFailure {
                agent: target.agent.clone(),
                path: target.path.clone(),
                error: error.to_string(),
            }),
        }
    }

    result
}

fn symlink_matches_expected(
    link_path: &Path,
    expected_target: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    if link_path.exists() && expected_target.exists() {
        return symlink_points_to(link_path, expected_target);
    }

    let actual_target = fs::read_link(link_path)?;
    let actual_target = if actual_target.is_absolute() {
        actual_target
    } else {
        link_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(actual_target)
    };

    Ok(actual_target == expected_target)
}

pub fn is_supported_agent(agent: &str) -> bool {
    SUPPORTED_AGENTS.contains(&agent)
}

pub(crate) fn is_safe_registry_name(name: &str) -> bool {
    !name.is_empty()
        && Path::new(name)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && Path::new(name).components().count() == 1
}

pub fn validated_skill_path(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = Path::new(name);
    let mut has_component = false;
    let mut safe_path = PathBuf::new();

    if name.is_empty() || path.is_absolute() {
        return Err(format!("Invalid skill name '{}'", name).into());
    }

    for component in path.components() {
        match component {
            Component::Normal(part) => {
                has_component = true;
                safe_path.push(part);
            }
            _ => return Err(format!("Invalid skill name '{}'", name).into()),
        }
    }

    if !has_component {
        return Err(format!("Invalid skill name '{}'", name).into());
    }

    Ok(safe_path)
}

#[cfg(unix)]
pub fn symlink_dir(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(windows)]
pub fn symlink_dir(source: &Path, target: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(source, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("skm-test-{}-{}", std::process::id(), unique));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn local_skill(project: &Path, name: &str) -> SkillSpec {
        let source = project.join("source").join(name);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# Test skill\n").unwrap();

        SkillSpec {
            name: name.to_string(),
            version: Some("latest".to_string()),
            source: None,
            path: Some(format!("source/{}", name)),
            kind: None,
        }
    }

    fn write_registry_skill(
        registry: &Path,
        name: &str,
        version: &str,
        dependencies: Option<&str>,
    ) -> PathBuf {
        let path = registry.join(name).join(format!("v{version}"));
        fs::create_dir_all(&path).unwrap();
        let declared_name = name.rsplit('/').next().unwrap();
        let dependency_metadata = dependencies
            .map(|value| format!("  skm-dependencies: \"{value}\"\n"))
            .unwrap_or_default();
        fs::write(
            path.join("SKILL.md"),
            format!(
                "---\nname: {declared_name}\ndescription: Registry test skill.\nmetadata:\n  skm-version: \"{version}\"\n{dependency_metadata}---\n\n# Test\n"
            ),
        )
        .unwrap();
        path
    }

    fn registry_resolver<'a>(
        registry: &'a Path,
    ) -> impl Fn(&SkillSpec, &Path) -> Result<PathBuf, Box<dyn std::error::Error>> + 'a {
        move |skill, _| {
            let version = skill.version.as_deref().unwrap_or("latest");
            Ok(registry
                .join(&skill.name)
                .join(if version.starts_with('v') {
                    version.to_string()
                } else {
                    format!("v{version}")
                }))
        }
    }

    #[test]
    fn resolves_exact_registry_skill_dependency_closure() {
        let project = temp_project();
        let registry = project.join("registry");
        write_registry_skill(&registry, "workspace/write-spec", "0.2.0", None);
        write_registry_skill(
            &registry,
            "workspace/wk-spec",
            "0.1.0",
            Some("workspace/write-spec@0.2.0"),
        );
        let requested = vec![SkillSpec {
            name: "workspace/wk-spec".to_string(),
            version: Some("0.1.0".to_string()),
            source: Some("default".to_string()),
            path: None,
            kind: None,
        }];

        let resolved = resolve_skill_dependency_closure_with(
            &requested,
            &project,
            registry_resolver(&registry),
        )
        .unwrap();

        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].name, "workspace/wk-spec");
        assert_eq!(resolved[0].version.as_deref(), Some("0.1.0"));
        assert_eq!(resolved[1].name, "workspace/write-spec");
        assert_eq!(resolved[1].version.as_deref(), Some("0.2.0"));
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn rejects_excessive_registry_dependency_depth() {
        let project = temp_project();
        let registry = project.join("registry");
        for index in 0..=MAX_SKILL_DEPENDENCY_DEPTH {
            let dependency = (index < MAX_SKILL_DEPENDENCY_DEPTH)
                .then(|| format!("workspace/skill{}@1.0.0", index + 1));
            write_registry_skill(
                &registry,
                &format!("workspace/skill{index}"),
                "1.0.0",
                dependency.as_deref(),
            );
        }
        let requested = [SkillSpec {
            name: "workspace/skill0".into(),
            version: Some("1.0.0".into()),
            source: Some("default".into()),
            path: None,
            kind: None,
        }];
        let error = resolve_skill_dependency_closure_with(
            &requested,
            &project,
            registry_resolver(&registry),
        )
        .expect_err("deep dependency graph is rejected");
        assert!(error.to_string().contains("dependency depth exceeds"));
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn resolves_latest_alias_to_published_metadata_version() {
        let project = temp_project();
        let registry = project.join("registry");
        let published = write_registry_skill(&registry, "workspace/wk-review", "0.1.0", None);
        let requested = vec![SkillSpec {
            name: "workspace/wk-review".to_string(),
            version: Some("latest".to_string()),
            source: None,
            path: None,
            kind: None,
        }];

        let resolved = resolve_skill_dependency_closure_with(&requested, &project, move |_, _| {
            Ok(published.clone())
        })
        .unwrap();

        assert_eq!(resolved[0].version.as_deref(), Some("0.1.0"));
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn rejects_registry_dependency_cycle() {
        let project = temp_project();
        let registry = project.join("registry");
        write_registry_skill(
            &registry,
            "workspace/wk-plan",
            "0.1.0",
            Some("workspace/wk-spec@0.1.0"),
        );
        write_registry_skill(
            &registry,
            "workspace/wk-spec",
            "0.1.0",
            Some("workspace/wk-plan@0.1.0"),
        );
        let requested = vec![SkillSpec {
            name: "workspace/wk-plan".to_string(),
            version: Some("0.1.0".to_string()),
            source: None,
            path: None,
            kind: None,
        }];

        let error = resolve_skill_dependency_closure_with(
            &requested,
            &project,
            registry_resolver(&registry),
        )
        .unwrap_err();

        assert!(error.to_string().contains("dependency cycle"));
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn rejects_conflicting_registry_dependency_versions() {
        let project = temp_project();
        let registry = project.join("registry");
        write_registry_skill(&registry, "workspace/shared", "1.0.0", None);
        write_registry_skill(&registry, "workspace/shared", "2.0.0", None);
        let requested = vec![
            SkillSpec {
                name: "workspace/shared".to_string(),
                version: Some("1.0.0".to_string()),
                source: None,
                path: None,
                kind: None,
            },
            SkillSpec {
                name: "workspace/shared".to_string(),
                version: Some("2.0.0".to_string()),
                source: None,
                path: None,
                kind: None,
            },
        ];

        let error = resolve_skill_dependency_closure_with(
            &requested,
            &project,
            registry_resolver(&registry),
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("conflicting registry skill requirements"));
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn rejects_malformed_or_local_registry_dependencies() {
        let project = temp_project();
        let registry = project.join("registry");
        write_registry_skill(
            &registry,
            "workspace/wk-spec",
            "0.1.0",
            Some("workspace/write-spec@latest"),
        );
        let requested = vec![SkillSpec {
            name: "workspace/wk-spec".to_string(),
            version: Some("0.1.0".to_string()),
            source: None,
            path: None,
            kind: None,
        }];
        let error = resolve_skill_dependency_closure_with(
            &requested,
            &project,
            registry_resolver(&registry),
        )
        .unwrap_err();
        assert!(error.to_string().contains("exact semantic versioning"));

        write_registry_skill(
            &registry,
            "workspace/local",
            "0.1.0",
            Some("workspace/write-spec@0.2.0"),
        );
        let local = vec![SkillSpec {
            name: "workspace/local".to_string(),
            version: Some("0.1.0".to_string()),
            source: None,
            path: Some("local".to_string()),
            kind: None,
        }];
        let error =
            resolve_skill_dependency_closure_with(&local, &project, registry_resolver(&registry))
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("cannot declare registry dependencies"));
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn parses_exact_registry_lock_target() {
        let skill =
            parse_registry_lock_target("registry:default/workspace/write-spec@0.2.0").unwrap();
        assert_eq!(skill.name, "workspace/write-spec");
        assert_eq!(skill.version.as_deref(), Some("0.2.0"));
        assert_eq!(skill.source.as_deref(), Some("default"));
        assert!(
            parse_registry_lock_target("registry:default/workspace/write-spec@latest").is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_registry_version_symlinks_that_escape_or_replace_exact_versions() {
        let project = temp_project();
        let package = project.join("registry/skills/workspace/example");
        let outside = project.join("outside");
        fs::create_dir_all(&package).unwrap();
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, package.join("latest")).unwrap();
        let error =
            validate_registry_version_path(&package, &package.join("latest"), Path::new("latest"))
                .unwrap_err();
        assert!(error.to_string().contains("escapes its package"));

        std::os::unix::fs::symlink(&outside, package.join("v0.1.0")).unwrap();
        let error =
            validate_registry_version_path(&package, &package.join("v0.1.0"), Path::new("v0.1.0"))
                .unwrap_err();
        assert!(error.to_string().contains("must not be a symlink"));
        fs::remove_dir_all(project).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn accepts_registry_alias_to_real_version_inside_package() {
        let project = temp_project();
        let package = project.join("registry/skills/workspace/example");
        fs::create_dir_all(package.join("v0.1.0")).unwrap();
        std::os::unix::fs::symlink("v0.1.0", package.join("latest")).unwrap();
        validate_registry_version_path(&package, &package.join("latest"), Path::new("latest"))
            .unwrap();
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn rejects_unsafe_skill_names() {
        for name in ["", ".", "../escape", "foo/../../escape", "/tmp/escape"] {
            assert!(validated_skill_path(name).is_err(), "{name} should fail");
        }
    }

    #[test]
    fn refuses_to_replace_existing_non_symlink_directory() {
        let project = temp_project();
        let skill = local_skill(&project, "foo");
        let agents = vec!["codex".to_string()];
        let existing = project.join(".agents").join("skills").join("foo");
        fs::create_dir_all(&existing).unwrap();
        fs::write(existing.join("keep.txt"), "keep").unwrap();

        let error = link_skill(&skill, &project, &agents, false).unwrap_err();

        assert!(error.to_string().contains("Refusing to replace"));
        assert!(existing.join("keep.txt").exists());
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn replaces_existing_symlink_only() {
        let project = temp_project();
        let skill = local_skill(&project, "foo");
        let other = project.join("other");
        fs::create_dir_all(&other).unwrap();
        let agents = vec!["codex".to_string()];
        let target = project.join(".agents").join("skills").join("foo");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        symlink_dir(&other, &target).unwrap();

        link_skill(&skill, &project, &agents, false).unwrap();

        assert!(symlink_points_to(&target, &project.join("source").join("foo")).unwrap());
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn rejects_unknown_agents() {
        let agents = vec!["codxe".to_string()];

        let error = validate_agents(&agents).unwrap_err();
        assert!(error
            .to_string()
            .contains("Supported agents: claude, codex"));
        assert!(error.to_string().contains("qwen, hermes"));
    }

    #[test]
    fn namespaced_skills_use_discoverable_direct_child_targets() {
        let root = Path::new("/project/.agents/skills");
        assert_eq!(
            get_skill_target_path(root, "workspace/wk-spec").unwrap(),
            root.join("wk-spec")
        );
        let skills = ["workspace/spec", "software-development/spec"]
            .into_iter()
            .map(|name| SkillSpec {
                name: name.into(),
                version: None,
                source: None,
                path: None,
                kind: None,
            })
            .collect::<Vec<_>>();
        assert!(validate_unique_skill_targets(&skills)
            .unwrap_err()
            .to_string()
            .contains("both install as 'spec'"));
    }

    #[test]
    fn empty_effective_agent_targets_are_rejected() {
        let root = Path::new("/project");
        assert!(require_skill_targets(&[], root, false)
            .unwrap_err()
            .to_string()
            .contains("No agent skill targets"));
        assert!(require_skill_targets(&["hermes".into()], root, false)
            .unwrap_err()
            .to_string()
            .contains("global skills only"));
        assert!(require_skill_targets(&["codex".into()], root, false).is_ok());
    }

    #[test]
    fn resolves_every_supported_agent_to_the_documented_paths() {
        let project = Path::new("/project");
        let cases = [
            ("claude", ".claude/skills", ".claude/skills"),
            ("codex", ".agents/skills", ".agents/skills"),
            ("copilot", ".github/skills", ".copilot/skills"),
            ("cursor", ".cursor/skills", ".cursor/skills"),
            ("antigravity", ".agents/skills", ".gemini/config/skills"),
            ("pi", ".pi/skills", ".pi/agent/skills"),
            ("opencode", ".opencode/skills", ".config/opencode/skills"),
            ("cline", ".cline/skills", ".cline/skills"),
            ("kilo", ".kilo/skills", ".kilo/skills"),
            ("gemini-cli", ".gemini/skills", ".gemini/skills"),
            ("goose", ".agents/skills", ".agents/skills"),
            ("crush", ".crush/skills", ".config/crush/skills"),
            ("openhands", ".agents/skills", ".openhands/skills"),
            ("grok", ".grok/skills", ".grok/skills"),
            ("qwen", ".qwen/skills", ".qwen/skills"),
        ];

        assert_eq!(SUPPORTED_AGENTS.len(), 16);
        for (agent, project_path, global_path) in cases {
            assert_eq!(
                get_project_agent_skills_dirs(agent, project).unwrap(),
                vec![project.join(project_path)]
            );
            assert_eq!(agent_skill_paths(agent, true).unwrap(), &[global_path]);
        }
        assert_eq!(
            get_project_agent_skills_dirs("hermes", project).unwrap(),
            Vec::<PathBuf>::new()
        );
        assert_eq!(
            agent_skill_paths("hermes", true).unwrap(),
            &[".hermes/skills"]
        );
    }

    #[test]
    fn deduplicates_shared_agent_targets_and_records_claimants() {
        let project = Path::new("/project");
        let agents = ["codex", "goose", "openhands", "antigravity"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        let targets = resolve_agent_skill_targets(&agents, project, false).unwrap();

        assert_eq!(
            targets,
            vec![AgentSkillTarget {
                path: project.join(".agents/skills"),
                agents,
            }]
        );
    }

    #[test]
    fn project_hermes_has_no_link_target() {
        let project = Path::new("/project");
        let targets = resolve_agent_skill_targets(&["hermes".to_string()], project, false).unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn plans_and_applies_skill_unlink_idempotently() {
        let project = temp_project();
        let skill = local_skill(&project, "foo");
        let agents = vec!["codex".to_string()];
        let target = project.join(".agents").join("skills").join("foo");

        link_skill(&skill, &project, &agents, false).unwrap();
        assert!(target.exists() || target.is_symlink());

        let targets = plan_skill_unlink(&skill, &project, &agents, false, false, false).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].path, target);
        let result = apply_skill_unlink(&targets);
        assert_eq!(result.removed, vec![target.clone()]);
        assert!(result.failures.is_empty());
        assert!(!target.exists() && !target.is_symlink());

        // Idempotent check
        let targets = plan_skill_unlink(&skill, &project, &agents, false, false, false).unwrap();
        assert!(targets.is_empty());

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn refuses_to_unlink_unexpected_symlink_without_force() {
        let project = temp_project();
        let skill = local_skill(&project, "foo");
        let agents = vec!["codex".to_string()];
        let unexpected = project.join("unexpected");
        fs::create_dir_all(&unexpected).unwrap();
        let target = project.join(".agents").join("skills").join("foo");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        symlink_dir(&unexpected, &target).unwrap();

        let error = plan_skill_unlink(&skill, &project, &agents, false, false, false).unwrap_err();

        assert!(error.to_string().contains("unexpected symlink"));
        assert!(target.is_symlink());

        let forced = plan_skill_unlink(&skill, &project, &agents, false, true, false).unwrap();
        assert_eq!(forced.len(), 1);

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn refuses_symlinked_skill_namespace_without_external_writes() {
        let project = temp_project();
        let skill = local_skill(&project, "group/foo");
        let agents = vec!["codex".to_string()];
        let external = project.join("external");
        fs::create_dir_all(&external).unwrap();
        fs::write(external.join("keep.txt"), "keep").unwrap();
        let namespace = project.join(".agents/skills/group");
        fs::create_dir_all(namespace.parent().unwrap()).unwrap();
        symlink_dir(&external, &namespace).unwrap();

        let link_error = link_skill(&skill, &project, &agents, false).unwrap_err();
        assert!(link_error.to_string().contains("symlinked skill namespace"));

        let external_target = external.join("foo");
        fs::create_dir_all(&external_target).unwrap();
        fs::write(external_target.join("content.txt"), "content").unwrap();
        let unlink_error =
            plan_skill_unlink(&skill, &project, &agents, false, true, false).unwrap_err();
        assert!(unlink_error
            .to_string()
            .contains("symlinked skill namespace"));
        assert_eq!(
            fs::read_to_string(external.join("keep.txt")).unwrap(),
            "keep"
        );
        assert_eq!(
            fs::read_to_string(external_target.join("content.txt")).unwrap(),
            "content"
        );

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn unlink_continues_after_per_target_failure() {
        let project = temp_project();
        let first = project.join("first");
        let failed = project.join("failed");
        let last = project.join("last");
        let source = project.join("source-dir");
        fs::create_dir_all(&source).unwrap();
        symlink_dir(&source, &first).unwrap();
        fs::create_dir_all(&failed).unwrap();
        symlink_dir(&source, &last).unwrap();

        let targets = vec![
            UnlinkTarget {
                agent: "claude".to_string(),
                path: first.clone(),
                kind: UnlinkTargetKind::Symlink,
            },
            UnlinkTarget {
                agent: "codex".to_string(),
                path: failed.clone(),
                kind: UnlinkTargetKind::Symlink,
            },
            UnlinkTarget {
                agent: "cursor".to_string(),
                path: last.clone(),
                kind: UnlinkTargetKind::Symlink,
            },
        ];

        let result = apply_skill_unlink(&targets);

        assert_eq!(result.removed, vec![first.clone(), last.clone()]);
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].path, failed);
        assert!(!first.is_symlink());
        assert!(result.failures[0].error.contains("no longer a symlink"));
        assert!(!last.is_symlink());

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn versioned_bundle_history_rejects_duplicate_keys_at_every_depth() {
        let prefix = "schema_version: 2\nnamespace: acme\npackages: {alpha: 1.0.0}\nbundles: {starter: {packages: [alpha]}}\nbundle_versions:\n";
        for history in [
            "  starter: {1.0.0: {alpha: 1.0.0}}\n  starter: {1.0.0: {alpha: 1.0.0}}\n",
            "  starter:\n    1.0.0: {alpha: 1.0.0}\n    1.0.0: {alpha: 1.0.0}\n",
            "  starter:\n    1.0.0: {alpha: 1.0.0, alpha: 1.0.0}\n",
        ] {
            assert!(
                serde_yaml::from_str::<NamespaceManifest>(&format!("{prefix}{history}")).is_err(),
                "accepted {history}"
            );
        }
    }

    #[test]
    fn versioned_bundle_history_rejects_invalid_releases_and_projections() {
        let prefix = "schema_version: 2\nnamespace: acme\npackages: {alpha: 1.0.0}\nbundles: {starter: {packages: [alpha]}}\nbundle_versions:\n";
        for history in [
            "  starter: {}\n",
            "  missing: {1.0.0: {alpha: 1.0.0}}\n",
            "  starter: {1.0: {alpha: 1.0.0}}\n",
            "  starter: {v1.0.0: {alpha: 1.0.0}}\n",
            "  starter: {1.0.0: {}}\n",
            "  starter: {1.0.0: {alpha: 2.0.0}}\n",
            "  starter: {1.0.0: {../bad: 1.0.0}}\n",
            "  starter: {1.0.0: {alpha: latest}}\n",
        ] {
            let manifest: NamespaceManifest =
                serde_yaml::from_str(&format!("{prefix}{history}")).unwrap();
            assert!(
                validate_manifest(&manifest, "acme").is_err(),
                "accepted {history}"
            );
        }
    }

    #[test]
    #[cfg(unix)]
    #[serial_test::serial]
    fn resolve_entries_expands_bundle_entries_to_members() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        let registry = root.path().join("registry");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(registry.join("skills/acme/alpha/v1.0.0")).unwrap();
        fs::create_dir_all(registry.join("skills/acme/helper/v2.0.0")).unwrap();
        fs::write(
            registry.join("skills/acme/alpha/v1.0.0/SKILL.md"),
            "---\nname: alpha\nmetadata:\n  skm-version: \"1.0.0\"\n  skm-dependencies: \"acme/helper@2.0.0\"\n---\n# Alpha\n",
        )
        .unwrap();
        fs::write(
            registry.join("skills/acme/helper/v2.0.0/SKILL.md"),
            "---\nname: helper\nmetadata:\n  skm-version: \"2.0.0\"\n---\n# Helper\n",
        )
        .unwrap();
        fs::write(
            registry.join("skills/acme/manifest.yaml"),
            "schema_version: 2\nnamespace: acme\npackages:\n  alpha: 1.0.0\n  helper: 2.0.0\nbundles:\n  starter:\n    packages: [alpha]\n",
        )
        .unwrap();
        fs::write(
            project.join("skills.yaml"),
            format!(
                "name: test\nagents: [codex]\nregistries:\n  local: {}\nskills: []\n",
                registry.display()
            ),
        )
        .unwrap();

        let bundle = SkillSpec {
            name: "acme/starter".into(),
            version: Some("latest".into()),
            source: Some("local".into()),
            path: None,
            kind: Some("bundle".into()),
        };

        // Use a custom resolver that resolves through the project-local registry.
        let reg = registry.clone();
        let resolve = move |skill: &SkillSpec,
                            _project_root: &Path|
              -> Result<PathBuf, Box<dyn std::error::Error>> {
            resolve_registry_skill_source_dir(skill, &reg)
        };
        let entries = resolve_entries_with(&[bundle], &project, resolve).unwrap();
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            ResolvedEntry::Bundle { spec, members, .. } => {
                assert_eq!(spec.name, "acme/starter");
                let member_names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
                assert!(member_names.contains(&"acme/alpha"));
                assert!(member_names.contains(&"acme/helper"));
            }
            _ => panic!("expected bundle entry"),
        }
        let all: Vec<String> = entries
            .iter()
            .flat_map(|e| e.linkable_skills())
            .map(|s| s.name)
            .collect();
        assert!(all.contains(&"acme/alpha".to_string()));
        assert!(all.contains(&"acme/helper".to_string()));
    }

    #[test]
    fn resolve_entries_rejects_invalid_kind() {
        let root = tempfile::tempdir().unwrap();
        let bad = SkillSpec {
            name: "acme/alpha".into(),
            version: None,
            source: None,
            path: None,
            kind: Some("widget".into()),
        };
        let err = resolve_entries(&[bad], root.path())
            .unwrap_err()
            .to_string();
        assert!(err.contains("Invalid skill kind 'widget'"));
    }

    #[test]
    fn resolve_entries_rejects_bundle_with_local_path() {
        let root = tempfile::tempdir().unwrap();
        let bundle = SkillSpec {
            name: "acme/starter".into(),
            version: None,
            source: Some("default".into()),
            path: Some("./local".into()),
            kind: Some("bundle".into()),
        };
        let err = resolve_entries(&[bundle], root.path())
            .unwrap_err()
            .to_string();
        assert!(err.contains("cannot use a local path"));
    }
}
