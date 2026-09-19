use crate::config::SkillsConfig;
use crate::config_manager::BaseConfig;
use serde::Serialize;
use std::collections::BTreeMap;
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
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Discovery {
    pub entries: Vec<Entry>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SearchMatch<'a> {
    name: &'a str,
    version: &'a str,
    registry: &'a str,
    add_command: String,
}

#[derive(Debug, Serialize)]
struct SearchOutput<'a> {
    query: &'a str,
    count: usize,
    total: usize,
    matches: Vec<SearchMatch<'a>>,
    warnings: &'a [String],
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
            Ok(entries) => discovery.entries.extend(entries),
            Err(message) => discovery.warnings.push(format!("{name}: {message}")),
        }
    }
    discovery
        .entries
        .sort_by(|a, b| (&a.name, &a.registry).cmp(&(&b.name, &b.registry)));
    Ok(discovery)
}

fn load_registry(
    name: &str,
    location: &str,
    project: &Path,
    refresh: bool,
) -> Result<Vec<Entry>, String> {
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

fn scan_local(registry: &str, root: &Path) -> Result<Vec<Entry>, String> {
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
    Ok(entries(registry, versions))
}

fn scan_remote(registry: &str, location: &str) -> Result<Vec<Entry>, String> {
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
        .arg(repository)
        .args(["ls-tree", "-r", "-z", "HEAD", "--", "skills"]);
    parse_tree(registry, &run_git(tree, Duration::from_secs(10))?)
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
        })
        .collect()
}

fn regular_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
}

fn is_git_url(location: &str) -> bool {
    ["https://", "http://", "ssh://", "git://", "file://", "git@"]
        .iter()
        .any(|prefix| location.starts_with(prefix))
}

fn cache_matches(cache: &Path, location: &str) -> bool {
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

fn run_git(mut command: Command, timeout: Duration) -> Result<Vec<u8>, String> {
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

pub fn matching_entries(entries: &[Entry], query: &str) -> Result<Vec<Entry>, String> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Err("Search query must not be empty".into());
    }
    Ok(entries
        .iter()
        .filter(|entry| entry.name.to_lowercase().contains(&query))
        .cloned()
        .collect())
}

pub fn print_results(
    query: &str,
    matches: &[Entry],
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
        .map(|entry| SearchMatch {
            name: &entry.name,
            version: &entry.version,
            registry: &entry.registry,
            add_command: add_command(entry),
        })
        .collect();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&SearchOutput {
                query,
                count: visible.len(),
                total: matches.len(),
                matches: visible,
                warnings,
            })?
        );
        return Ok(());
    }

    for warning in warnings {
        eprintln!("Warning: {warning}");
    }
    if visible.is_empty() {
        println!("No skills found matching '{}'.", query.trim());
        return Ok(());
    }
    for entry in visible {
        println!("{}  {}  [{}]", entry.name, entry.version, entry.registry);
        println!("  Add: {}", entry.add_command);
    }
    if matches.len() > limit {
        println!("Showing {} of {} matches.", limit, matches.len());
    } else {
        println!("{} skill(s) found.", matches.len());
    }
    Ok(())
}

fn add_command(entry: &Entry) -> String {
    format!("skm add {} --source {}", entry.name, entry.registry)
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
        }
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
        assert_eq!(matching_entries(&entries, "SPEC").unwrap().len(), 2);
        assert!(matching_entries(&entries, " ").is_err());
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
            }]
        );
    }

    #[test]
    fn json_shape_reports_visible_and_total_counts() {
        let entries = [
            entry("software/review", "company"),
            entry("software/spec", "default"),
        ];
        let visible = entries
            .iter()
            .take(1)
            .map(|entry| SearchMatch {
                name: &entry.name,
                version: &entry.version,
                registry: &entry.registry,
                add_command: add_command(entry),
            })
            .collect();
        let warnings = ["other: unavailable".into()];
        let json = serde_json::to_value(SearchOutput {
            query: "software",
            count: 1,
            total: entries.len(),
            matches: visible,
            warnings: &warnings,
        })
        .unwrap();
        assert_eq!(json["count"], 1);
        assert_eq!(json["total"], 2);
        assert_eq!(json["matches"][0]["name"], "software/review");
        assert_eq!(
            json["matches"][0]["add_command"],
            "skm add software/review --source company"
        );
    }
}
