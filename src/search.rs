use crate::config::SkillsConfig;
use crate::config_manager::BaseConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{Read, Seek};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub registry: String,
    pub version: String,
    pub description: Option<String>,
    pub dependencies: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Bundle {
    pub id: String,
    pub registry: String,
    pub members: usize,
    pub packages: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Collection {
    pub id: String,
    pub registry: String,
    pub members: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Discovery {
    pub entries: Vec<Entry>,
    pub bundles: Vec<Bundle>,
    pub collections: Vec<Collection>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SearchMatch<'a> {
    name: &'a str,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<&'a str>,
    registry: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    dependencies: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    members: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    packages: Option<&'a [String]>,
    add_command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    apply_command: Option<String>,
}

#[derive(Clone, Debug)]
pub enum SearchItem<'a> {
    Skill(&'a Entry),
    Bundle(&'a Bundle),
}

impl SearchItem<'_> {
    fn name(&self) -> &str {
        match self {
            Self::Skill(entry) => &entry.name,
            Self::Bundle(bundle) => &bundle.id,
        }
    }

    fn registry(&self) -> &str {
        match self {
            Self::Skill(entry) => &entry.registry,
            Self::Bundle(bundle) => &bundle.registry,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Skill(_) => "skill",
            Self::Bundle(_) => "bundle",
        }
    }

    fn display(&self) -> SearchMatch<'_> {
        match self {
            Self::Skill(entry) => SearchMatch {
                name: &entry.name,
                kind: "skill",
                version: Some(&entry.version),
                registry: &entry.registry,
                description: entry.description.as_deref(),
                dependencies: &entry.dependencies,
                members: None,
                packages: None,
                add_command: format!(
                    "skm add {} --source {} --kind skill",
                    entry.name, entry.registry
                ),
                apply_command: None,
            },
            Self::Bundle(bundle) => {
                let command = format!(
                    "skm add {} --source {} --kind bundle",
                    bundle.id, bundle.registry
                );
                SearchMatch {
                    name: &bundle.id,
                    kind: "bundle",
                    version: None,
                    registry: &bundle.registry,
                    description: None,
                    dependencies: &[],
                    members: Some(bundle.members),
                    packages: Some(&bundle.packages),
                    add_command: format!("{command} --dry-run"),
                    apply_command: Some(format!("{command} --yes")),
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct SearchOutput<'a> {
    query: &'a str,
    count: usize,
    total: usize,
    matches: Vec<SearchMatch<'a>>,
    bundles: &'a [Bundle],
    collections: &'a [Collection],
    warnings: &'a [String],
}

#[derive(Default)]
struct RegistryContents {
    entries: Vec<Entry>,
    bundles: Vec<Bundle>,
    collections: Vec<Collection>,
}

#[derive(Deserialize)]
struct NamespaceManifest {
    schema_version: u32,
    namespace: String,
    #[serde(default)]
    packages: BTreeMap<String, String>,
    #[serde(default)]
    bundles: BTreeMap<String, BundleMembers>,
}

#[derive(Deserialize)]
struct BundleMembers {
    packages: Vec<String>,
}

pub fn discover(
    config: Option<&SkillsConfig>,
    project: &Path,
    registry: Option<&str>,
) -> Result<Discovery, String> {
    discover_with_refresh(config, project, registry, false)
}

pub fn discover_with_refresh(
    config: Option<&SkillsConfig>,
    project: &Path,
    registry: Option<&str>,
    refresh: bool,
) -> Result<Discovery, String> {
    let base =
        BaseConfig::load().map_err(|error| format!("Cannot read global registries: {error}"))?;
    let mut registries: BTreeMap<_, _> = base.registries.into_iter().collect();
    if let Some(project_registries) = config.and_then(|config| config.registries.as_ref()) {
        registries.extend(project_registries.clone());
    }
    discover_locations(registries, project, registry, refresh)
}

fn discover_locations(
    mut registries: BTreeMap<String, String>,
    project: &Path,
    registry: Option<&str>,
    refresh: bool,
) -> Result<Discovery, String> {
    if let Some(registry) = registry {
        let location = registries
            .remove(registry)
            .ok_or_else(|| format!("Registry '{registry}' is not configured"))?;
        registries.clear();
        registries.insert(registry.to_owned(), location);
    }

    let mut discovery = Discovery::default();
    for (name, location) in registries {
        match load_registry(&name, &location, project, refresh) {
            Ok(contents) => {
                discovery.entries.extend(contents.entries);
                discovery.bundles.extend(contents.bundles);
                discovery.collections.extend(contents.collections);
            }
            Err(message) => discovery.warnings.push(format!("{name}: {message}")),
        }
    }
    discovery
        .entries
        .sort_by(|a, b| (&a.name, &a.registry).cmp(&(&b.name, &b.registry)));
    discovery
        .bundles
        .sort_by(|a, b| (&a.id, &a.registry).cmp(&(&b.id, &b.registry)));
    discovery
        .collections
        .sort_by(|a, b| (&a.id, &a.registry).cmp(&(&b.id, &b.registry)));
    Ok(discovery)
}

fn load_registry(
    name: &str,
    location: &str,
    project: &Path,
    refresh: bool,
) -> Result<RegistryContents, String> {
    if crate::linker::resolve_registry_path(name).is_none() {
        return Err("Invalid registry name".into());
    }
    if location.trim().is_empty() {
        return Err("Registry URL or local path is empty".into());
    }

    let local = project.join(location);
    if local.exists() {
        return scan_local(name, &local);
    }
    if !is_git_url(location) {
        return Err("Local registry path does not exist, or URL scheme is unsupported".into());
    }
    if !refresh {
        if let Some(cache) = crate::linker::resolve_registry_path(name) {
            if cache_matches(&cache, location) {
                return scan_local(name, &cache);
            }
        }
    }
    scan_remote(name, location)
}

fn scan_local(registry: &str, root: &Path) -> Result<RegistryContents, String> {
    let skills = root.join("skills");
    if !regular_directory(root) || !regular_directory(&skills) {
        return Err("Registry needs a real skills directory with versioned SKILL.md files".into());
    }

    let mut versions = BTreeMap::new();
    let walker = walkdir::WalkDir::new(&skills)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| entry.file_name() != ".git");
    for entry in walker {
        let entry = entry.map_err(|_| "Cannot read registry files".to_owned())?;
        if entry.file_type().is_file() && entry.file_name() == "SKILL.md" {
            let relative = entry.path().strip_prefix(&skills).unwrap();
            let parts: Option<Vec<_>> = relative.iter().map(|part| part.to_str()).collect();
            if let Some(parts) = parts {
                record_skill(&mut versions, &parts);
            }
        }
    }
    let mut entries = entries(registry, versions);
    for entry in &mut entries {
        let path = skills
            .join(&entry.name)
            .join(&entry.version)
            .join("SKILL.md");
        if let Some(details) = fs::File::open(path)
            .ok()
            .and_then(skill_details_from_reader)
        {
            entry.description = details.description;
            entry.dependencies = details.dependencies;
        }
    }
    let mut bundles = Vec::new();
    let mut collections = Vec::new();
    for namespace in fs::read_dir(&skills).map_err(|error| error.to_string())? {
        let namespace = namespace.map_err(|error| error.to_string())?;
        if !namespace.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let id = namespace.file_name();
        let Some(id) = id.to_str() else { continue };
        let manifest = namespace.path().join("manifest.yaml");
        if let Ok(file) = fs::File::open(manifest) {
            let (collection, published) = groups_from_reader(file, registry, id);
            collections.extend(collection);
            bundles.extend(published);
        }
    }
    Ok(RegistryContents {
        entries,
        bundles,
        collections,
    })
}

fn scan_remote(registry: &str, location: &str) -> Result<RegistryContents, String> {
    let temporary =
        tempfile::tempdir().map_err(|_| "Cannot create temporary registry storage".to_owned())?;
    let repository = temporary.path().join("registry");
    let mut clone = Command::new("git");
    clone
        .args([
            "-c",
            "protocol.ext.allow=never",
            "clone",
            "--depth",
            "1",
            "--no-checkout",
            "--",
            location,
        ])
        .arg(&repository);
    run_git(clone, Duration::from_secs(30))?;

    let mut tree = Command::new("git");
    tree.arg("-C")
        .arg(&repository)
        .args(["ls-tree", "-r", "-z", "HEAD", "--", "skills"]);
    let listing = run_git(tree, Duration::from_secs(10))?;
    let mut entries = parse_tree(registry, &listing)?;
    for entry in &mut entries {
        let path = format!("HEAD:skills/{}/{}/SKILL.md", entry.name, entry.version);
        if let Some(details) = git_object(&repository, &path)
            .ok()
            .and_then(|bytes| skill_details_from_reader(bytes.as_slice()))
        {
            entry.description = details.description;
            entry.dependencies = details.dependencies;
        }
    }
    let mut bundles = Vec::new();
    let mut collections = Vec::new();
    for item in listing.split(|byte| *byte == 0) {
        let Ok(item) = std::str::from_utf8(item) else {
            continue;
        };
        let Some((metadata, path)) = item.split_once('\t') else {
            continue;
        };
        if !metadata.starts_with("100644 blob ") && !metadata.starts_with("100755 blob ") {
            continue;
        }
        let Some(namespace) = path
            .strip_prefix("skills/")
            .and_then(|path| path.strip_suffix("/manifest.yaml"))
        else {
            continue;
        };
        if namespace.contains('/') {
            continue;
        }
        if let Ok(bytes) = git_object(&repository, &format!("HEAD:{path}")) {
            let (collection, published) = groups_from_reader(bytes.as_slice(), registry, namespace);
            collections.extend(collection);
            bundles.extend(published);
        }
    }
    Ok(RegistryContents {
        entries,
        bundles,
        collections,
    })
}

fn git_object(repository: &Path, object: &str) -> Result<Vec<u8>, String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(repository).arg("show").arg(object);
    run_git(command, Duration::from_secs(10))
}

struct SkillDetails {
    description: Option<String>,
    dependencies: Vec<String>,
}

fn skill_details_from_reader(reader: impl Read) -> Option<SkillDetails> {
    let mut bytes = Vec::new();
    reader.take(16 * 1024).read_to_end(&mut bytes).ok()?;
    let content = std::str::from_utf8(&bytes).ok()?;
    let frontmatter = content.strip_prefix("---\n")?.split_once("\n---")?.0;
    let yaml: serde_yaml::Value = serde_yaml::from_str(frontmatter).ok()?;
    let description = yaml.get("description").and_then(serde_yaml::Value::as_str);
    let description = description
        .map(|description| {
            description
                .chars()
                .map(|character| {
                    if character.is_control() {
                        ' '
                    } else {
                        character
                    }
                })
                .collect::<String>()
        })
        .and_then(|safe| {
            let normalized = safe.split_whitespace().collect::<Vec<_>>().join(" ");
            (!normalized.is_empty()).then(|| normalized.chars().take(1024).collect())
        });
    let dependencies = yaml
        .get("metadata")
        .and_then(|metadata| metadata.get("skm-dependencies"))
        .and_then(serde_yaml::Value::as_str)
        .and_then(|raw| crate::linker::parse_skill_dependencies(Some(raw)).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|(name, version)| format!("{name}@{version}"))
        .collect();
    Some(SkillDetails {
        description,
        dependencies,
    })
}

fn groups_from_reader(
    reader: impl Read,
    registry: &str,
    namespace: &str,
) -> (Vec<Collection>, Vec<Bundle>) {
    let mut bytes = Vec::new();
    if reader.take(64 * 1024).read_to_end(&mut bytes).is_err() {
        return (Vec::new(), Vec::new());
    }
    let Ok(manifest) = serde_yaml::from_slice::<NamespaceManifest>(&bytes) else {
        return (Vec::new(), Vec::new());
    };
    if !matches!(manifest.schema_version, 1 | 2)
        || manifest.namespace != namespace
        || manifest.packages.is_empty()
        || manifest.packages.iter().any(|(id, version)| {
            crate::linker::validate_skill_name(&format!("{namespace}/{id}")).is_err()
                || version.split('.').count() != 3
                || version
                    .split('.')
                    .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
        })
    {
        return (Vec::new(), Vec::new());
    }
    let collection = Collection {
        id: namespace.to_owned(),
        registry: registry.to_owned(),
        members: manifest.packages.len(),
    };
    if manifest.schema_version == 1 {
        return (vec![collection], Vec::new());
    }
    let bundles = manifest
        .bundles
        .into_iter()
        .filter_map(|(id, members)| {
            if !id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                || id.starts_with('-')
                || id.ends_with('-')
                || crate::linker::validate_skill_name(&format!("{namespace}/{id}")).is_err()
                || members.packages.is_empty()
                || members
                    .packages
                    .iter()
                    .any(|member| !manifest.packages.contains_key(member))
            {
                return None;
            }
            let member_set = members
                .packages
                .iter()
                .collect::<std::collections::BTreeSet<_>>();
            if member_set.len() != members.packages.len()
                || (id == "all-workspace-skills"
                    && member_set != manifest.packages.keys().collect())
            {
                return None;
            }
            let packages = member_set
                .into_iter()
                .map(|name| format!("{namespace}/{name}"))
                .collect::<Vec<_>>();
            Some(Bundle {
                id: format!("{namespace}/{id}"),
                registry: registry.to_owned(),
                members: packages.len(),
                packages,
            })
        })
        .collect();
    (vec![collection], bundles)
}

fn parse_tree(registry: &str, tree: &[u8]) -> Result<Vec<Entry>, String> {
    let mut versions = BTreeMap::new();
    for entry in tree.split(|byte| *byte == 0) {
        let Ok(entry) = std::str::from_utf8(entry) else {
            continue;
        };
        let Some((metadata, path)) = entry.split_once('\t') else {
            continue;
        };
        if !metadata.starts_with("100644 blob ") && !metadata.starts_with("100755 blob ") {
            continue;
        }
        if let Some(path) = path.strip_prefix("skills/") {
            record_skill(&mut versions, &path.split('/').collect::<Vec<_>>());
        }
    }
    Ok(entries(registry, versions))
}

fn record_skill(versions: &mut BTreeMap<String, String>, parts: &[&str]) {
    if parts.len() < 3 || parts.last() != Some(&"SKILL.md") {
        return;
    }
    let version = parts[parts.len() - 2];
    let is_version = version == "latest"
        || version == "default"
        || version
            .strip_prefix('v')
            .is_some_and(|value| value.starts_with(|character: char| character.is_ascii_digit()));
    if !is_version {
        return;
    }
    let name = parts[..parts.len() - 2].join("/");
    if crate::linker::validate_skill_name(&name).is_err() {
        return;
    }

    let rank = |value: &str| match value {
        "latest" => 2,
        "default" => 0,
        _ => 1,
    };
    let replace = versions.get(&name).is_none_or(|current| {
        rank(version)
            .cmp(&rank(current))
            .then_with(|| crate::version_manager::compare_versions(version, current))
            .is_gt()
    });
    if replace {
        versions.insert(name, version.to_owned());
    }
}

fn entries(registry: &str, versions: BTreeMap<String, String>) -> Vec<Entry> {
    versions
        .into_iter()
        .map(|(name, version)| Entry {
            name,
            registry: registry.to_owned(),
            version,
            description: None,
            dependencies: Vec::new(),
        })
        .collect()
}

fn regular_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
}

pub(crate) fn is_git_url(location: &str) -> bool {
    ["https://", "http://", "ssh://", "git://", "file://", "git@"]
        .iter()
        .any(|prefix| location.starts_with(prefix))
}

pub(crate) fn cache_matches(cache: &Path, location: &str) -> bool {
    if !regular_directory(cache) || !regular_directory(&cache.join(".git")) {
        return false;
    }
    let mut command = Command::new("git");
    command
        .arg("config")
        .arg("--file")
        .arg(cache.join(".git/config"))
        .args(["--get", "remote.origin.url"]);
    run_git(command, Duration::from_secs(2))
        .ok()
        .is_some_and(|bytes| String::from_utf8_lossy(&bytes).trim() == location)
}

pub(crate) fn run_git(mut command: Command, timeout: Duration) -> Result<Vec<u8>, String> {
    let mut output =
        tempfile::tempfile().map_err(|_| "Cannot create temporary Git output".to_owned())?;
    command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(output.try_clone().map_err(|error| error.to_string())?)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes -oConnectTimeout=10");
    let mut child = command
        .spawn()
        .map_err(|_| "Cannot run Git to search remote registries".to_owned())?;
    let started = Instant::now();
    loop {
        if output
            .metadata()
            .is_ok_and(|metadata| metadata.len() > 8 * 1024 * 1024)
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Registry listing is too large to search".into());
        }
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) => {
                return Err("Could not load registry; check its URL and Git access".into())
            }
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Registry loading timed out".into());
            }
        }
    }

    output.rewind().map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    output
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Registry listing is too large to search".into());
    }
    Ok(bytes)
}

pub fn matching_items<'a>(
    entries: &'a [Entry],
    bundles: &'a [Bundle],
    query: &str,
) -> Result<Vec<SearchItem<'a>>, String> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Err("Search query must not be empty".into());
    }
    let mut matches: Vec<_> = entries
        .iter()
        .filter(|entry| entry.name.to_lowercase().contains(&query))
        .map(SearchItem::Skill)
        .chain(
            bundles
                .iter()
                .filter(|bundle| bundle.id.to_lowercase().contains(&query))
                .map(SearchItem::Bundle),
        )
        .collect();
    matches.sort_by(|a, b| {
        (a.name(), a.registry(), a.kind()).cmp(&(b.name(), b.registry(), b.kind()))
    });
    Ok(matches)
}

pub fn print_results(
    query: &str,
    matches: &[SearchItem<'_>],
    bundles: &[Bundle],
    collections: &[Collection],
    warnings: &[String],
    limit: usize,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if limit == 0 {
        return Err("--limit must be greater than zero".into());
    }
    let visible: Vec<_> = matches
        .iter()
        .take(limit)
        .map(SearchItem::display)
        .collect();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&SearchOutput {
                query,
                count: visible.len(),
                total: matches.len(),
                matches: visible,
                bundles,
                collections,
                warnings,
            })?
        );
        return Ok(());
    }

    for warning in warnings {
        eprintln!("Warning: {warning}");
    }
    if visible.is_empty() {
        println!("No skills or bundles found matching '{}'.", query.trim());
    }
    print!("{}", format_text_results(&visible));
    if matches.len() > limit {
        println!("Showing {} of {} matches.", limit, matches.len());
    } else if !matches.is_empty() {
        println!("{} skill(s) or bundle(s) found.", matches.len());
    }
    if !collections.is_empty() {
        println!("Skill collections (browse only; group installation unavailable):");
        for collection in collections {
            println!(
                "  {}  ({} skills)  [{}]",
                collection.id, collection.members, collection.registry
            );
        }
    }
    Ok(())
}

fn format_text_results(entries: &[SearchMatch<'_>]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let names: Vec<_> = entries
        .iter()
        .map(|entry| match entry.version {
            Some(version) => format!("{}@{} ({})", entry.name, version, entry.kind),
            None => format!("{} ({})", entry.name, entry.kind),
        })
        .collect();
    let name_width = names.iter().map(String::len).max().unwrap_or(0).max(34);
    let description_width = 100_usize.saturating_sub(name_width + 2).max(30);
    let mut output = String::new();
    writeln!(output, "{:<name_width$}  DESCRIPTION", "NAME (TYPE)").unwrap();
    writeln!(
        output,
        "{}  {}",
        "─".repeat(name_width),
        "─".repeat(description_width)
    )
    .unwrap();

    for (index, (entry, name)) in entries.iter().zip(names).enumerate() {
        if index > 0 {
            output.push('\n');
        }
        let summary = match (entry.description, entry.packages) {
            (Some(description), _) => description.to_string(),
            (_, Some(packages)) => format!(
                "Includes {} skill{}: {}.",
                entry.members.unwrap_or(packages.len()),
                if packages.len() == 1 { "" } else { "s" },
                packages.join(", ")
            ),
            _ => "Description unavailable.".to_string(),
        };
        let mut details = wrap_search_text(&summary, description_width);
        details.push(format!("Registry: {}", entry.registry));
        if !entry.dependencies.is_empty() {
            details.extend(wrap_search_text(
                &format!("Dependencies: {}", entry.dependencies.join(", ")),
                description_width,
            ));
        }
        for (line_index, detail) in details.iter().enumerate() {
            let left = if line_index == 0 { name.as_str() } else { "" };
            writeln!(output, "{left:<name_width$}  {detail}").unwrap();
        }
    }
    output
}

fn wrap_search_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(root: &Path, name: &str, version: &str) {
        let directory = root.join("skills").join(name).join(version);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("SKILL.md"), "# Fixture skill\n").unwrap();
    }

    fn entry(name: &str, registry: &str) -> Entry {
        Entry {
            name: name.into(),
            registry: registry.into(),
            version: "v1.0.0".into(),
            description: None,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn text_results_show_two_columns_without_add_instructions() {
        let skill = Entry {
            name: "software/spec".into(),
            registry: "default".into(),
            version: "1.2.0".into(),
            description: Some("Write clear, reviewable software specs.".into()),
            dependencies: vec!["software/review@2.0.0".into()],
        };
        let bundle = Bundle {
            id: "software/starter".into(),
            registry: "company".into(),
            members: 2,
            packages: vec!["software/spec".into(), "software/review".into()],
        };
        let skill_item = SearchItem::Skill(&skill);
        let bundle_item = SearchItem::Bundle(&bundle);
        let displayed = [skill_item.display(), bundle_item.display()];

        let output = format_text_results(&displayed);
        let lines: Vec<_> = output.lines().collect();
        assert!(lines[0].starts_with("NAME (TYPE)"));
        assert!(lines[0].ends_with("DESCRIPTION"));
        let description_column = lines[0].find("DESCRIPTION").unwrap();
        assert_eq!(
            &lines[2][description_column..],
            "Write clear, reviewable software specs."
        );
        assert!(lines[2].starts_with("software/spec@1.2.0 (skill)"));
        assert_eq!(&lines[3][description_column..], "Registry: default");
        assert_eq!(
            &lines[4][description_column..],
            "Dependencies: software/review@2.0.0"
        );
        assert!(lines[6].starts_with("software/starter (bundle)"));
        assert_eq!(
            &lines[6][description_column..],
            "Includes 2 skills: software/spec, software/review."
        );
        assert_eq!(&lines[7][description_column..], "Registry: company");
        assert!(!output.contains("skm add"));
        assert!(!output.contains("Preview:"));
    }

    #[test]
    fn text_results_handle_missing_description_and_wrap_long_details() {
        let missing = entry("software/undocumented", "default");
        let bundle = Bundle {
            id: "software/large-bundle".into(),
            registry: "default".into(),
            members: 3,
            packages: vec![
                "software/first-long-skill".into(),
                "software/second-long-skill".into(),
                "software/third-long-skill".into(),
            ],
        };
        let skill_item = SearchItem::Skill(&missing);
        let bundle_item = SearchItem::Bundle(&bundle);
        let output = format_text_results(&[skill_item.display(), bundle_item.display()]);
        assert!(output.contains("Description unavailable."));
        assert!(output.contains("Includes 3 skills: software/first-long-skill,"));
        assert!(output.contains("software/third-long-skill."));
        assert_eq!(format_text_results(&[]), "");
    }

    #[test]
    fn discovery_filters_registries_and_keeps_partial_results() {
        let project = tempfile::tempdir().unwrap();
        skill(&project.path().join("available"), "software/spec", "v1.2.0");
        let registries = BTreeMap::from([
            ("available".into(), "available".into()),
            ("missing".into(), "missing".into()),
        ]);
        let all = discover_locations(registries.clone(), project.path(), None, false).unwrap();
        assert_eq!(all.entries.len(), 1);
        assert_eq!(all.entries[0].name, "software/spec");
        assert_eq!(all.entries[0].version, "v1.2.0");
        assert_eq!(all.warnings.len(), 1);
        let filtered =
            discover_locations(registries.clone(), project.path(), Some("available"), false)
                .unwrap();
        assert_eq!(filtered.entries.len(), 1);
        assert!(filtered.warnings.is_empty());
        assert!(discover_locations(registries, project.path(), Some("unknown"), false).is_err());
    }

    #[test]
    fn name_search_is_case_insensitive() {
        let company = entry("software/spec", "company");
        let default = entry("software/spec", "default");
        let entries = vec![
            company.clone(),
            default.clone(),
            entry("ai/review", "default"),
        ];
        let bundles = [Bundle {
            id: "software/spec-kit".into(),
            registry: "default".into(),
            members: 1,
            packages: vec!["software/spec".into()],
        }];
        let matches = matching_items(&entries, &bundles, "SPEC").unwrap();
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].kind(), "skill");
        assert_eq!(matches[2].kind(), "bundle");
        assert_eq!(matching_items(&entries, &bundles, "kit").unwrap().len(), 1);
        assert!(matching_items(&entries, &bundles, " ").is_err());
    }

    #[test]
    fn git_tree_ignores_symlinks_and_unsafe_or_unversioned_paths() {
        let tree = b"100644 blob a\tskills/dev/spec/v2.0.0/SKILL.md\0\
                     120000 blob b\tskills/link/v1.0.0/SKILL.md\0\
                     100644 blob c\tskills/../outside/v1.0.0/SKILL.md\0\
                     100644 blob d\tskills/unversioned/SKILL.md\0";
        let found = parse_tree("default", tree).unwrap();
        assert_eq!(
            found,
            [Entry {
                name: "dev/spec".into(),
                registry: "default".into(),
                version: "v2.0.0".into(),
                description: None,
                dependencies: Vec::new(),
            }]
        );
    }

    #[test]
    fn json_shape_reports_visible_and_total_counts() {
        let entries = [
            entry("software/review", "company"),
            entry("software/spec", "default"),
        ];
        let bundles = [Bundle {
            id: "software/starter".into(),
            registry: "company".into(),
            members: 1,
            packages: vec!["software/review".into()],
        }];
        let matches = matching_items(&entries, &bundles, "software").unwrap();
        let visible = matches.iter().map(SearchItem::display).collect();
        let warnings = ["other: unavailable".into()];
        let json = serde_json::to_value(SearchOutput {
            query: "software",
            count: 3,
            total: matches.len(),
            matches: visible,
            bundles: &bundles,
            collections: &[],
            warnings: &warnings,
        })
        .unwrap();
        assert_eq!(json["count"], 3);
        assert_eq!(json["total"], 3);
        assert_eq!(json["matches"][0]["name"], "software/review");
        assert_eq!(json["matches"][0]["kind"], "skill");
        assert_eq!(
            json["matches"][0]["add_command"],
            "skm add software/review --source company --kind skill"
        );
        assert_eq!(json["matches"][2]["kind"], "bundle");
        assert_eq!(json["matches"][2]["members"], 1);
        assert_eq!(
            json["matches"][2]["add_command"],
            "skm add software/starter --source company --kind bundle --dry-run"
        );
        assert_eq!(
            json["matches"][2]["apply_command"],
            "skm add software/starter --source company --kind bundle --yes"
        );
        assert_eq!(json["bundles"].as_array().unwrap().len(), 1);
        assert!(json["matches"][0]["dependencies"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(json["collections"].as_array().unwrap().is_empty());
    }

    #[test]
    fn local_discovery_reads_description_and_published_bundles() {
        let project = tempfile::tempdir().unwrap();
        let registry = project.path().join("registry");
        let directory = registry.join("skills/workspace/spec/v1.2.0");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("SKILL.md"),
            "---\nname: spec\ndescription: >-\n  Write a clear\n  specification.\nmetadata:\n  skm-dependencies: \"workspace/write-spec@0.2.0\"\n---\n# Spec\n",
        )
        .unwrap();
        let manifest = registry.join("skills/workspace/manifest.yaml");
        fs::write(&manifest, "schema_version: 2\nnamespace: workspace\npackages:\n  spec: 1.2.0\nbundles:\n  all-workspace-skills:\n    packages: [spec]\n").unwrap();
        let found = scan_local("default", &registry).unwrap();
        assert_eq!(
            found.entries[0].description.as_deref(),
            Some("Write a clear specification.")
        );
        assert_eq!(
            found.entries[0].dependencies,
            ["workspace/write-spec@0.2.0"]
        );
        assert_eq!(found.collections[0].id, "workspace");
        assert_eq!(
            found.bundles,
            [Bundle {
                id: "workspace/all-workspace-skills".into(),
                registry: "default".into(),
                members: 1,
                packages: vec!["workspace/spec".into()],
            }]
        );
        fs::write(
            manifest,
            "schema_version: 1\nnamespace: workspace\npackages:\n  spec: 1.2.0\n",
        )
        .unwrap();
        let found = scan_local("default", &registry).unwrap();
        assert!(found.bundles.is_empty());
        assert_eq!(found.collections[0].id, "workspace");
        assert_eq!(found.collections[0].members, 1);
    }

    #[test]
    fn malformed_metadata_does_not_invent_a_description_or_bundle() {
        assert!(skill_details_from_reader("# no frontmatter".as_bytes()).is_none());
        let manifest = "schema_version: 2\nnamespace: workspace\npackages:\n  spec: 1.2.0\nbundles:\n  all-workspace-skills:\n    packages: [missing]\n";
        assert!(
            groups_from_reader(manifest.as_bytes(), "default", "workspace")
                .1
                .is_empty()
        );
        let incomplete = "schema_version: 2\nnamespace: workspace\npackages:\n  spec: 1.2.0\n  plan: 1.0.0\nbundles:\n  all-workspace-skills:\n    packages: [spec]\n";
        assert!(
            groups_from_reader(incomplete.as_bytes(), "default", "workspace")
                .1
                .is_empty()
        );
        let injected = "---\nname: spec\ndescription: \"Describe \\u001b[31m safely\"\n---\n";
        assert_eq!(
            skill_details_from_reader(injected.as_bytes())
                .unwrap()
                .description
                .as_deref(),
            Some("Describe [31m safely")
        );
        let invalid = "---\nname: spec\nmetadata:\n  skm-dependencies: \"workspace/write-spec@latest\"\n---\n";
        assert!(skill_details_from_reader(invalid.as_bytes())
            .unwrap()
            .dependencies
            .is_empty());
        let no_bundle = "schema_version: 1\nnamespace: workspace\npackages:\n  spec: 1.2.0\n";
        let (collections, bundles) =
            groups_from_reader(no_bundle.as_bytes(), "default", "workspace");
        assert_eq!(collections[0].members, 1);
        assert!(bundles.is_empty());
    }

    #[test]
    fn remote_discovery_reads_descriptions_and_bundles_without_a_checkout() {
        let temporary = tempfile::tempdir().unwrap();
        let registry = temporary.path().join("registry");
        let skill = registry.join("skills/workspace/spec/v1.0.0");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: spec\ndescription: Write specifications.\nmetadata:\n  skm-dependencies: \"workspace/write-spec@0.2.0\"\n---\n# Spec\n",
        )
        .unwrap();
        fs::write(registry.join("skills/workspace/manifest.yaml"), "schema_version: 2\nnamespace: workspace\npackages:\n  spec: 1.0.0\nbundles:\n  all-workspace-skills:\n    packages: [spec]\n").unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
        ] {
            let status = Command::new("git")
                .arg("-C")
                .arg(&registry)
                .args(args)
                .status()
                .unwrap();
            assert!(status.success());
        }
        let found = scan_remote("default", &format!("file://{}", registry.display())).unwrap();
        assert_eq!(
            found.entries[0].description.as_deref(),
            Some("Write specifications.")
        );
        assert_eq!(
            found.entries[0].dependencies,
            ["workspace/write-spec@0.2.0"]
        );
        assert_eq!(found.collections[0].id, "workspace");
        assert_eq!(found.bundles[0].id, "workspace/all-workspace-skills");
        assert_eq!(found.bundles[0].packages, ["workspace/spec"]);
    }
}
