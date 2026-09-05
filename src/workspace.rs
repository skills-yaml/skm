use crate::config::SkillsConfig;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Audit,
    Adopt,
    Upgrade,
    Repair,
}

#[derive(Debug, Serialize)]
struct WorkspacePlan {
    schema_version: u32,
    mode: Mode,
    current_version: Option<String>,
    target_version: String,
    source: Option<String>,
    resolved_source: Option<String>,
    source_authorization: String,
    source_revision: Option<String>,
    source_integrity: Option<String>,
    migration_chain: Vec<String>,
    missing_resources: Vec<String>,
    verified: bool,
    network_access: bool,
    repository_writes: Vec<String>,
    commands: Vec<String>,
    delegated_workflow: String,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    mode: Mode,
    config: &SkillsConfig,
    project_root: &Path,
    target: Option<String>,
    explicit_source: Option<String>,
    explicit_revision: Option<String>,
    explicit_integrity: Option<String>,
    apply: bool,
    yes: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_standard = target
        .or_else(|| {
            config
                .workspace
                .as_ref()
                .map(|value| value.standard.clone())
        })
        .ok_or("workspace target is required; use --target or configure workspace.standard")?;
    let target_version = parse_standard_version(&target_standard)?;
    let current_version = detect_workspace_version(project_root)?;
    if apply && !yes {
        return Err("--apply requires --yes; no repository files were changed".into());
    }
    if apply && !matches!(mode, Mode::Audit) {
        validate_handoff_target(project_root)?;
    }

    let source_request = select_source(
        config,
        project_root,
        explicit_source.as_deref(),
        explicit_revision.as_deref(),
        explicit_integrity.as_deref(),
    )?;
    let mut plan = WorkspacePlan {
        schema_version: 1,
        mode,
        current_version: current_version.clone(),
        target_version: target_version.clone(),
        source: source_request
            .as_ref()
            .map(|request| request.source.clone()),
        resolved_source: None,
        source_authorization: source_request
            .as_ref()
            .map(|request| request.authorization.clone())
            .unwrap_or_else(|| "none".to_string()),
        source_revision: source_request
            .as_ref()
            .and_then(|request| request.revision.clone()),
        source_integrity: None,
        migration_chain: Vec::new(),
        missing_resources: Vec::new(),
        verified: false,
        network_access: false,
        repository_writes: if apply {
            vec![".skm/workspace-plan.yaml".to_string()]
        } else {
            Vec::new()
        },
        commands: vec!["task check".to_string(), "task test".to_string()],
        delegated_workflow: "adopt-workspace-structure".to_string(),
    };

    if let Some(source_request) = &source_request {
        let persist_cache = apply && yes && !matches!(mode, Mode::Audit);
        let resolved = resolve_source(
            project_root,
            source_request,
            persist_cache,
            current_version.as_deref(),
            &target_version,
        )?;
        plan.resolved_source = resolved.relative.clone();
        plan.network_access = resolved.used_network;
        if resolved.used_network {
            plan.repository_writes.push(if persist_cache {
                resolved
                    .relative
                    .clone()
                    .unwrap_or_else(|| ".skm/cache/workspace".to_string())
            } else {
                ".skm/transactions/workspace-bootstrap (temporary; removed before exit)".to_string()
            });
        }
        let assessment = assess_package(
            &resolved.package_path,
            current_version.as_deref(),
            &target_version,
        )?;
        plan.migration_chain = assessment.migration_chain;
        plan.missing_resources = assessment.missing_resources;
        let actual_integrity = hash_tree(&resolved.package_path)?;
        if let Some(expected) = &source_request.integrity {
            if &actual_integrity != expected {
                return Err(format!(
                    "workspace source integrity mismatch: expected {expected}, found {actual_integrity}"
                )
                .into());
            }
        }
        plan.source_integrity = Some(actual_integrity);
        plan.verified = plan.missing_resources.is_empty();
    } else {
        plan.missing_resources
            .push("complete trusted workspace-docs source".to_string());
    }

    print_plan(&plan, json)?;
    if !plan.verified {
        return Err(format!(
            "workspace bootstrap is blocked: {}; authorize a complete source with --source or workspace.source",
            plan.missing_resources.join(", ")
        )
        .into());
    }
    if matches!(mode, Mode::Audit) || !apply {
        return Ok(());
    }
    let handoff = project_root.join(".skm/workspace-plan.yaml");
    let content = serde_yaml::to_string(&plan)?;
    if fs::read_to_string(&handoff).ok().as_deref() == Some(content.as_str()) {
        eprintln!("Verified workspace handoff is already current.");
        return Ok(());
    }
    if let Some(parent) = handoff.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = project_root.join(".skm/.workspace-plan.yaml.skm-tmp");
    if path_exists(&temporary) {
        return Err("stale workspace handoff transaction detected".into());
    }
    if let Err(error) = fs::write(&temporary, content) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    if let Err(error) = fs::rename(&temporary, &handoff) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    eprintln!(
        "Verified workspace handoff created. Run the '{}' skill to apply the structural migration.",
        plan.delegated_workflow
    );
    Ok(())
}

fn parse_standard_version(value: &str) -> Result<String, Box<dyn std::error::Error>> {
    let version = value.strip_prefix("workspace-docs@").unwrap_or(value);
    parse_semver(version)?;
    Ok(version.to_string())
}

fn detect_workspace_version(
    project_root: &Path,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let agents = project_root.join("AGENTS.md");
    match fs::symlink_metadata(&agents) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("AGENTS.md must not be a symlink".into())
        }
        Ok(metadata) if !metadata.is_file() => return Ok(None),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let content = fs::read_to_string(agents)?;
    let marker = "workspace-docs@";
    for remainder in content.split(marker).skip(1) {
        let candidate: String = remainder
            .chars()
            .take_while(|character| character.is_ascii_digit() || *character == '.')
            .collect();
        if parse_semver(&candidate).is_ok() {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

#[derive(Debug)]
struct SourceRequest {
    source: String,
    authorization: String,
    revision: Option<String>,
    integrity: Option<String>,
}

struct ResolvedSource {
    package_path: PathBuf,
    relative: Option<String>,
    used_network: bool,
    cleanup: Option<TransientCleanup>,
}

struct TransientCleanup {
    path: PathBuf,
    project_root: PathBuf,
    skm_existed: bool,
    transactions_existed: bool,
}

impl Drop for ResolvedSource {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            let _ = fs::remove_dir_all(&cleanup.path);
            cleanup_empty_transaction_parents(
                &cleanup.project_root,
                cleanup.skm_existed,
                cleanup.transactions_existed,
            );
        }
    }
}

fn select_source(
    config: &SkillsConfig,
    project_root: &Path,
    explicit: Option<&str>,
    explicit_revision: Option<&str>,
    explicit_integrity: Option<&str>,
) -> Result<Option<SourceRequest>, Box<dyn std::error::Error>> {
    if let Some(source) = explicit {
        validate_source_request(source, explicit_revision, explicit_integrity)?;
        return Ok(Some(SourceRequest {
            source: source.to_string(),
            authorization: "explicit-command".to_string(),
            revision: explicit_revision.map(str::to_string),
            integrity: explicit_integrity.map(str::to_string),
        }));
    }
    if let Some(workspace) = &config.workspace {
        if let Some(source) = &workspace.source {
            validate_source_request(
                source,
                workspace.revision.as_deref(),
                workspace.integrity.as_deref(),
            )?;
            if is_git_source(source) && !config.trusted_sources.contains(source) {
                return Err(format!(
                    "remote workspace source is not authorized by trusted_sources: {source}"
                )
                .into());
            }
            return Ok(Some(SourceRequest {
                source: source.clone(),
                authorization: "committed-manifest".to_string(),
                revision: workspace.revision.clone(),
                integrity: workspace.integrity.clone(),
            }));
        }
    }
    for candidate in [
        "workspace/instructions/standards/workspace-docs",
        "docs/standards/workspace-docs",
    ] {
        if project_root.join(candidate).exists() {
            return Ok(Some(SourceRequest {
                source: candidate.to_string(),
                authorization: "repository-local".to_string(),
                revision: None,
                integrity: None,
            }));
        }
    }
    Ok(None)
}

fn validate_source_request(
    source: &str,
    revision: Option<&str>,
    integrity: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if is_git_source(source) {
        if source.starts_with("https://") {
            let authority = source
                .strip_prefix("https://")
                .and_then(|remainder| remainder.split('/').next())
                .unwrap_or_default();
            if authority.contains('@') {
                return Err("workspace Git URLs must not contain embedded credentials".into());
            }
        }
        let revision = revision.ok_or("remote workspace sources require --revision")?;
        if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("workspace Git revision must be a full 40-character commit id".into());
        }
        let integrity = integrity.ok_or("remote workspace sources require --integrity")?;
        validate_integrity(integrity)?;
    } else {
        validate_relative_path(source)?;
        if revision.is_some() {
            return Err("--revision is valid only for a Git workspace source".into());
        }
        if let Some(integrity) = integrity {
            validate_integrity(integrity)?;
        }
    }
    Ok(())
}

pub(crate) fn is_git_source(source: &str) -> bool {
    source.starts_with("https://")
        || source.starts_with("ssh://")
        || source.starts_with("git@")
        || source.starts_with("file://")
}

fn validate_integrity(value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let digest = value
        .strip_prefix("sha256:")
        .ok_or("workspace integrity must use sha256:<hex>")?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("workspace integrity must contain a 64-character SHA-256 digest".into());
    }
    Ok(())
}

fn resolve_source(
    project_root: &Path,
    request: &SourceRequest,
    persist_cache: bool,
    current_version: Option<&str>,
    target_version: &str,
) -> Result<ResolvedSource, Box<dyn std::error::Error>> {
    if !is_git_source(&request.source) {
        let package_path = resolve_project_source(project_root, &request.source)?;
        return Ok(ResolvedSource {
            package_path,
            relative: Some(request.source.clone()),
            used_network: false,
            cleanup: None,
        });
    }
    fetch_git_source(
        project_root,
        request,
        persist_cache,
        current_version,
        target_version,
    )
}

fn fetch_git_source(
    project_root: &Path,
    request: &SourceRequest,
    persist_cache: bool,
    current_version: Option<&str>,
    target_version: &str,
) -> Result<ResolvedSource, Box<dyn std::error::Error>> {
    let revision = request
        .revision
        .as_deref()
        .ok_or("remote workspace source revision is missing")?;
    let expected_integrity = request
        .integrity
        .as_deref()
        .ok_or("remote workspace source integrity is missing")?;
    let cache_key = source_cache_key(&request.source, revision);
    let cache_root = project_root.join(".skm/cache/workspace").join(&cache_key);
    validate_repository_parent(
        project_root,
        Path::new(".skm/cache/workspace").join(&cache_key).as_path(),
    )?;
    match fs::symlink_metadata(&cache_root) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("workspace cache must not be a symlink".into())
        }
        Ok(metadata) if !metadata.is_dir() => {
            return Err("workspace cache must be a directory".into())
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    if cache_root.is_dir() {
        let package_path = find_package_root(&cache_root)?;
        let actual = hash_tree(&package_path)?;
        if actual != expected_integrity {
            return Err(format!(
                "verified workspace cache integrity mismatch: expected {expected_integrity}, found {actual}"
            )
            .into());
        }
        return Ok(ResolvedSource {
            relative: Some(relative_output_path(project_root, &package_path)?),
            package_path,
            used_network: false,
            cleanup: None,
        });
    }

    let transaction = project_root
        .join(".skm/transactions")
        .join(format!("workspace-bootstrap-{cache_key}"));
    validate_repository_parent(
        project_root,
        Path::new(".skm/transactions")
            .join(format!("workspace-bootstrap-{cache_key}"))
            .as_path(),
    )?;
    if path_exists(&transaction) {
        return Err(
            "stale workspace bootstrap transaction detected; no fetch was attempted".into(),
        );
    }
    let skm_existed = path_exists(&project_root.join(".skm"));
    let transactions_existed = path_exists(&project_root.join(".skm/transactions"));
    fs::create_dir_all(&transaction)?;
    let result = (|| -> Result<ResolvedSource, Box<dyn std::error::Error>> {
        let git_dir = transaction.join("source.git");
        let materialized = transaction.join("materialized");
        run_git(
            Command::new("git")
                .args(["init", "--bare", "--quiet"])
                .arg(&git_dir),
            "initialize workspace source cache",
        )?;
        run_git(
            Command::new("git")
                .arg("--git-dir")
                .arg(&git_dir)
                .args(["fetch", "--quiet", "--depth=1", "--no-tags", "--"])
                .arg(&request.source)
                .arg(revision),
            "fetch pinned workspace source",
        )?;
        let fetched = git_output(
            Command::new("git")
                .arg("--git-dir")
                .arg(&git_dir)
                .args(["rev-parse", "FETCH_HEAD"]),
            "resolve fetched workspace revision",
        )?;
        if String::from_utf8(fetched)?.trim() != revision {
            return Err("fetched workspace revision does not match the requested commit".into());
        }
        materialize_git_tree(&git_dir, &materialized)?;
        let package_path = find_package_root(&materialized)?;
        let package_subpath = package_path.strip_prefix(&materialized)?.to_path_buf();
        let actual = hash_tree(&package_path)?;
        if actual != expected_integrity {
            return Err(format!(
                "workspace source integrity mismatch: expected {expected_integrity}, found {actual}"
            )
            .into());
        }
        let assessment = assess_package(&package_path, current_version, target_version)?;
        if !assessment.missing_resources.is_empty() {
            return Err(format!(
                "workspace source is incomplete: {}",
                assessment.missing_resources.join(", ")
            )
            .into());
        }

        if persist_cache {
            if let Some(parent) = cache_root.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(&materialized, &cache_root)?;
            let persisted_package = cache_root.join(package_subpath);
            fs::remove_dir_all(&transaction)?;
            cleanup_empty_transaction_parents(project_root, skm_existed, transactions_existed);
            Ok(ResolvedSource {
                relative: Some(relative_output_path(project_root, &persisted_package)?),
                package_path: persisted_package,
                used_network: true,
                cleanup: None,
            })
        } else {
            Ok(ResolvedSource {
                package_path,
                relative: None,
                used_network: true,
                cleanup: Some(TransientCleanup {
                    path: transaction.clone(),
                    project_root: project_root.to_path_buf(),
                    skm_existed,
                    transactions_existed,
                }),
            })
        }
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&transaction);
        cleanup_empty_transaction_parents(project_root, skm_existed, transactions_existed);
    }
    result
}

fn source_cache_key(source: &str, revision: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update([0]);
    hasher.update(revision.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    digest[..24].to_string()
}

fn run_git(command: &mut Command, operation: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = command.status()?;
    if !status.success() {
        return Err(format!("failed to {operation}; Git exited with {status}").into());
    }
    Ok(())
}

fn git_output(
    command: &mut Command,
    operation: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!("failed to {operation}; Git exited with {}", output.status).into());
    }
    Ok(output.stdout)
}

fn materialize_git_tree(
    git_dir: &Path,
    destination: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let listing = git_output(
        Command::new("git").arg("--git-dir").arg(git_dir).args([
            "ls-tree",
            "-rz",
            "-r",
            "--full-tree",
            "FETCH_HEAD",
        ]),
        "inspect fetched workspace tree",
    )?;
    fs::create_dir(destination)?;
    let mut count = 0usize;
    let mut total_bytes = 0usize;
    for raw_entry in listing
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        count += 1;
        if count > 10_000 {
            return Err("workspace source exceeds the 10,000-file safety limit".into());
        }
        let entry = std::str::from_utf8(raw_entry)?;
        let (metadata, raw_path) = entry.split_once('\t').ok_or("malformed Git tree entry")?;
        let mut fields = metadata.split_whitespace();
        let mode = fields.next().ok_or("Git tree entry mode is missing")?;
        let object_type = fields.next().ok_or("Git tree entry type is missing")?;
        let object_id = fields.next().ok_or("Git tree entry object id is missing")?;
        if object_type != "blob" {
            return Err(format!("unsupported Git tree object at {raw_path}: {object_type}").into());
        }
        let relative = Path::new(raw_path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err("fetched workspace source contains an unsafe path".into());
        }
        let content = git_output(
            Command::new("git")
                .arg("--git-dir")
                .arg(git_dir)
                .args(["cat-file", "blob", object_id]),
            "read fetched workspace object",
        )?;
        total_bytes = total_bytes.saturating_add(content.len());
        if total_bytes > 64 * 1024 * 1024 {
            return Err("workspace source exceeds the 64 MiB safety limit".into());
        }
        if mode == "120000" {
            if is_ignorable_standard_pointer_path(relative, &content) {
                continue;
            }
            return Err(format!("workspace Git source contains symlink: {raw_path}").into());
        }
        if !matches!(mode, "100644" | "100755") {
            return Err(format!("unsupported Git file mode {mode} at {raw_path}").into());
        }
        let output_path = destination.join(relative);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output_path, content)?;
        #[cfg(unix)]
        if mode == "100755" {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&output_path, fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}

fn is_ignorable_standard_pointer_path(path: &Path, content: &[u8]) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if !matches!(name, "default" | "latest") {
        return false;
    }
    let parent_is_standard = path
        .parent()
        .is_some_and(|parent| parent.ends_with("standards/workspace-docs"));
    let target = std::str::from_utf8(content).unwrap_or_default().trim();
    parent_is_standard
        && target.starts_with('v')
        && parse_semver(target.trim_start_matches('v')).is_ok()
}

fn find_package_root(root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    for candidate in [
        root.to_path_buf(),
        root.join("workspace/instructions/standards/workspace-docs"),
        root.join("docs/standards/workspace-docs"),
    ] {
        if candidate.join("AGENT_MIGRATION.md").is_file() {
            return Ok(candidate);
        }
    }
    Err("fetched source does not contain a complete workspace-docs package root".into())
}

fn relative_output_path(
    project_root: &Path,
    path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(path
        .strip_prefix(project_root)?
        .to_str()
        .ok_or("workspace cache path contains non-UTF-8 data")?
        .replace('\\', "/"))
}

fn cleanup_empty_transaction_parents(
    project_root: &Path,
    skm_existed: bool,
    transactions_existed: bool,
) {
    if !transactions_existed {
        let _ = fs::remove_dir(project_root.join(".skm/transactions"));
    }
    if !skm_existed {
        let _ = fs::remove_dir(project_root.join(".skm"));
    }
}

struct PackageAssessment {
    migration_chain: Vec<String>,
    missing_resources: Vec<String>,
}

fn assess_package(
    source: &Path,
    current: Option<&str>,
    target: &str,
) -> Result<PackageAssessment, Box<dyn std::error::Error>> {
    let mut missing_resources = Vec::new();
    if !source.join("AGENT_MIGRATION.md").is_file() {
        missing_resources.push("AGENT_MIGRATION.md".to_string());
    }
    let current_tuple = current.map(parse_semver).transpose()?;
    let target_tuple = parse_semver(target)?;
    if current_tuple.is_some_and(|value| value > target_tuple) {
        return Err("workspace downgrade is not supported".into());
    }

    let mut versions = Vec::new();
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(version) = name.strip_prefix('v') else {
            continue;
        };
        let Ok(parsed) = parse_semver(version) else {
            continue;
        };
        if parsed <= target_tuple && current_tuple.is_none_or(|current| parsed > current) {
            versions.push((parsed, version.to_string(), entry.path()));
        }
    }
    versions.sort_by_key(|(parsed, _, _)| *parsed);
    let migration_chain = versions
        .iter()
        .map(|(_, version, _)| version.clone())
        .collect();

    let target_directory = source.join(format!("v{target}"));
    if !target_directory.is_dir() {
        missing_resources.push(format!("v{target}/ package directory"));
    }
    for resource in ["manifest.yaml", "agents-template.md", "audit-checklist.md"] {
        if !target_directory.join(resource).is_file() {
            missing_resources.push(format!("v{target}/{resource}"));
        }
    }
    if current != Some(target) {
        if versions.is_empty() {
            missing_resources.push(format!(
                "migration chain from {} to {target}",
                current.unwrap_or("fresh")
            ));
        }
        for (_, version, directory) in &versions {
            if !directory.join("migration.md").is_file() {
                missing_resources.push(format!("v{version}/migration.md"));
            }
        }
        if !target_directory.join("migration.md").is_file() {
            let missing = format!("v{target}/migration.md");
            if !missing_resources.contains(&missing) {
                missing_resources.push(missing);
            }
        }
    }
    missing_resources.sort();
    missing_resources.dedup();
    Ok(PackageAssessment {
        migration_chain,
        missing_resources,
    })
}

fn resolve_project_source(
    project_root: &Path,
    relative: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_relative_path(relative)?;
    let joined = project_root.join(relative);
    let root = fs::canonicalize(project_root)?;
    let canonical = fs::canonicalize(&joined)?;
    canonical
        .strip_prefix(&root)
        .map_err(|_| "workspace source escapes the repository")?;
    if !canonical.is_dir() {
        return Err("workspace source must be a directory".into());
    }
    let mut current = project_root.to_path_buf();
    for component in Path::new(relative).components() {
        if let Component::Normal(part) = component {
            current.push(part);
            if fs::symlink_metadata(&current)?.file_type().is_symlink() {
                return Err("workspace source path contains a symlink".into());
            }
        }
    }
    for entry in WalkDir::new(&canonical).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_symlink() && !is_allowed_standard_pointer(&canonical, entry.path())
        {
            return Err("workspace package contains an unsafe symlink".into());
        }
    }
    Ok(canonical)
}

pub(crate) fn source_integrity(
    project_root: &Path,
    relative: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let source = resolve_project_source(project_root, relative)?;
    hash_tree(&source)
}

fn validate_relative_path(value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("workspace source must be a repository-relative path".into());
    }
    Ok(())
}

fn validate_repository_parent(
    project_root: &Path,
    relative: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("workspace-managed path escapes the repository".into());
    }
    let components: Vec<_> = relative.components().collect();
    let mut current = project_root.to_path_buf();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        let Component::Normal(part) = component else {
            return Err("workspace-managed path escapes the repository".into());
        };
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "workspace-managed parent is a symlink: {}",
                    current.strip_prefix(project_root)?.display()
                )
                .into());
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(format!(
                    "workspace-managed parent is not a directory: {}",
                    current.strip_prefix(project_root)?.display()
                )
                .into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn validate_handoff_target(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let handoff = project_root.join(".skm/workspace-plan.yaml");
    validate_repository_parent(project_root, Path::new(".skm/workspace-plan.yaml"))?;
    match fs::symlink_metadata(&handoff) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err("workspace handoff must not be a symlink".into())
        }
        Ok(metadata) if !metadata.is_file() => {
            Err("workspace handoff must be a regular file".into())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn path_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn hash_tree(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_symlink() && !is_allowed_standard_pointer(path, entry.path()) {
            return Err("workspace package contains an unsafe symlink".into());
        }
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    let mut hasher = Sha256::new();
    for file in files {
        let relative = file.strip_prefix(path)?.to_string_lossy();
        hasher.update((relative.len() as u64).to_be_bytes());
        hasher.update(relative.as_bytes());
        let content = fs::read(file)?;
        hasher.update((content.len() as u64).to_be_bytes());
        hasher.update(content);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn is_allowed_standard_pointer(root: &Path, path: &Path) -> bool {
    if path.parent() != Some(root) {
        return false;
    }
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if !matches!(name, "default" | "latest") {
        return false;
    }
    fs::canonicalize(path)
        .ok()
        .and_then(|target| target.strip_prefix(root).ok().map(Path::to_path_buf))
        .is_some_and(|relative| {
            relative.components().count() == 1
                && relative
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.starts_with('v'))
        })
}

fn parse_semver(value: &str) -> Result<(u64, u64, u64), Box<dyn std::error::Error>> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("invalid workspace-docs version: {value}").into());
    }
    Ok((parts[0].parse()?, parts[1].parse()?, parts[2].parse()?))
}

fn print_plan(plan: &WorkspacePlan, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(plan)?);
        return Ok(());
    }
    println!("Workspace mode: {:?}", plan.mode);
    println!(
        "Version: {} -> {}",
        plan.current_version.as_deref().unwrap_or("unmanaged"),
        plan.target_version
    );
    println!("Source: {}", plan.source.as_deref().unwrap_or("none"));
    println!("Source authorization: {}", plan.source_authorization);
    println!("Verified: {}", plan.verified);
    if !plan.migration_chain.is_empty() {
        println!("Migration chain: {}", plan.migration_chain.join(" -> "));
    }
    for missing in &plan.missing_resources {
        println!("- missing: {missing}");
    }
    for write in &plan.repository_writes {
        println!("- write: {write}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ToolkitSelection, WorkspaceSelection};
    use std::process::Command;

    fn config(source: Option<&str>) -> SkillsConfig {
        SkillsConfig {
            name: "test".to_string(),
            version: None,
            registries: None,
            agents: vec!["codex".to_string()],
            skills: Vec::new(),
            toolkit: Some(ToolkitSelection {
                manifest: "toolkit.yaml".to_string(),
                version: "0.1.0".to_string(),
            }),
            bundles: Vec::new(),
            profiles: Vec::new(),
            workspace: Some(WorkspaceSelection {
                standard: "workspace-docs@3.0.0".to_string(),
                source: source.map(str::to_string),
                revision: None,
                integrity: None,
            }),
            trusted_sources: Vec::new(),
        }
    }

    #[test]
    fn partial_local_package_blocks_without_writes() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("AGENTS.md"), "workspace-docs@1.2.0\n").unwrap();
        let package = temp.path().join("docs/standards/workspace-docs");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("README.md"), "placeholder\n").unwrap();

        let error = run(
            Mode::Upgrade,
            &config(None),
            temp.path(),
            None,
            None,
            None,
            None,
            true,
            true,
            true,
        )
        .unwrap_err();
        assert!(error.to_string().contains("bootstrap is blocked"));
        assert!(!temp.path().join(".skm").exists());
    }

    #[test]
    fn explicit_complete_source_creates_verified_handoff() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("AGENTS.md"), "workspace-docs@1.2.0\n").unwrap();
        let package = temp.path().join("vendor/workspace-docs");
        fs::create_dir_all(package.join("v3.0.0")).unwrap();
        fs::write(package.join("AGENT_MIGRATION.md"), "migration\n").unwrap();
        for resource in [
            "manifest.yaml",
            "agents-template.md",
            "audit-checklist.md",
            "migration.md",
        ] {
            fs::write(package.join("v3.0.0").join(resource), "verified\n").unwrap();
        }

        run(
            Mode::Upgrade,
            &config(None),
            temp.path(),
            None,
            Some("vendor/workspace-docs".to_string()),
            None,
            None,
            true,
            true,
            true,
        )
        .unwrap();
        assert!(temp.path().join(".skm/workspace-plan.yaml").is_file());
    }

    #[test]
    fn fetches_pinned_git_source_and_persists_verified_cache() {
        let source = tempfile::tempdir().unwrap();
        let package = source
            .path()
            .join("workspace/instructions/standards/workspace-docs");
        fs::create_dir_all(package.join("v3.0.0")).unwrap();
        fs::write(package.join("AGENT_MIGRATION.md"), "migration\n").unwrap();
        for resource in [
            "manifest.yaml",
            "agents-template.md",
            "audit-checklist.md",
            "migration.md",
        ] {
            fs::write(package.join("v3.0.0").join(resource), "verified\n").unwrap();
        }
        git(source.path(), &["init", "--quiet"]);
        git(
            source.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        git(source.path(), &["config", "user.name", "SKM Test"]);
        git(source.path(), &["add", "."]);
        git(source.path(), &["commit", "--quiet", "-m", "fixture"]);
        let revision = git_output_for_test(source.path(), &["rev-parse", "HEAD"]);
        let integrity = hash_tree(&package).unwrap();

        let project = tempfile::tempdir().unwrap();
        fs::write(project.path().join("AGENTS.md"), "workspace-docs@1.2.0\n").unwrap();
        let source_url = format!("file://{}", source.path().display());
        run(
            Mode::Upgrade,
            &config(None),
            project.path(),
            None,
            Some(source_url),
            Some(revision),
            Some(integrity),
            true,
            true,
            true,
        )
        .unwrap();
        assert!(project.path().join(".skm/workspace-plan.yaml").is_file());
        assert!(project.path().join(".skm/cache/workspace").is_dir());
        assert!(!project.path().join(".skm/transactions").exists());

        let offline_source = source.path().with_extension("offline");
        fs::rename(source.path(), &offline_source).unwrap();
        let cached_url = format!("file://{}", source.path().display());
        let cached_revision = git_output_for_test(&offline_source, &["rev-parse", "HEAD"]);
        let cached_integrity =
            hash_tree(&offline_source.join("workspace/instructions/standards/workspace-docs"))
                .unwrap();
        run(
            Mode::Audit,
            &config(None),
            project.path(),
            None,
            Some(cached_url),
            Some(cached_revision),
            Some(cached_integrity),
            false,
            false,
            true,
        )
        .unwrap();
        fs::remove_dir_all(offline_source).unwrap();
    }

    #[test]
    fn rejects_unauthorized_committed_git_source_without_writes() {
        let project = tempfile::tempdir().unwrap();
        let mut configured = config(Some("https://example.invalid/workspace.git"));
        let workspace = configured.workspace.as_mut().unwrap();
        workspace.revision = Some("a".repeat(40));
        workspace.integrity = Some(format!("sha256:{}", "b".repeat(64)));
        let error = run(
            Mode::Upgrade,
            &configured,
            project.path(),
            None,
            None,
            None,
            None,
            false,
            false,
            true,
        )
        .unwrap_err();
        assert!(error.to_string().contains("not authorized"));
        assert!(!project.path().join(".skm").exists());
    }

    #[test]
    fn rejects_git_source_integrity_mismatch_and_cleans_transaction() {
        let source = tempfile::tempdir().unwrap();
        let package = source
            .path()
            .join("workspace/instructions/standards/workspace-docs");
        fs::create_dir_all(package.join("v3.0.0")).unwrap();
        fs::write(package.join("AGENT_MIGRATION.md"), "migration\n").unwrap();
        git(source.path(), &["init", "--quiet"]);
        git(
            source.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        git(source.path(), &["config", "user.name", "SKM Test"]);
        git(source.path(), &["add", "."]);
        git(source.path(), &["commit", "--quiet", "-m", "fixture"]);
        let revision = git_output_for_test(source.path(), &["rev-parse", "HEAD"]);

        let project = tempfile::tempdir().unwrap();
        let error = run(
            Mode::Audit,
            &config(None),
            project.path(),
            None,
            Some(format!("file://{}", source.path().display())),
            Some(revision),
            Some(format!("sha256:{}", "0".repeat(64))),
            false,
            false,
            true,
        )
        .unwrap_err();
        assert!(error.to_string().contains("integrity mismatch"));
        assert!(!project.path().join(".skm").exists());
    }

    #[test]
    fn integrity_failure_preserves_preexisting_transaction_directories() {
        let source = tempfile::tempdir().unwrap();
        let package = source
            .path()
            .join("workspace/instructions/standards/workspace-docs");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("AGENT_MIGRATION.md"), "migration\n").unwrap();
        git(source.path(), &["init", "--quiet"]);
        git(
            source.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        git(source.path(), &["config", "user.name", "SKM Test"]);
        git(source.path(), &["add", "."]);
        git(source.path(), &["commit", "--quiet", "-m", "fixture"]);
        let revision = git_output_for_test(source.path(), &["rev-parse", "HEAD"]);

        let project = tempfile::tempdir().unwrap();
        fs::create_dir_all(project.path().join(".skm/transactions")).unwrap();
        let error = run(
            Mode::Audit,
            &config(None),
            project.path(),
            None,
            Some(format!("file://{}", source.path().display())),
            Some(revision),
            Some(format!("sha256:{}", "0".repeat(64))),
            false,
            false,
            true,
        )
        .unwrap_err();
        assert!(error.to_string().contains("integrity mismatch"));
        assert!(project.path().join(".skm/transactions").is_dir());
    }

    #[test]
    fn incomplete_remote_source_is_not_persisted() {
        let source = tempfile::tempdir().unwrap();
        let package = source
            .path()
            .join("workspace/instructions/standards/workspace-docs");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("AGENT_MIGRATION.md"), "migration\n").unwrap();
        git(source.path(), &["init", "--quiet"]);
        git(
            source.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        git(source.path(), &["config", "user.name", "SKM Test"]);
        git(source.path(), &["add", "."]);
        git(source.path(), &["commit", "--quiet", "-m", "fixture"]);
        let revision = git_output_for_test(source.path(), &["rev-parse", "HEAD"]);
        let integrity = hash_tree(&package).unwrap();

        let project = tempfile::tempdir().unwrap();
        let error = run(
            Mode::Upgrade,
            &config(None),
            project.path(),
            None,
            Some(format!("file://{}", source.path().display())),
            Some(revision),
            Some(integrity),
            true,
            true,
            true,
        )
        .unwrap_err();
        assert!(error.to_string().contains("source is incomplete"));
        assert!(!project.path().join(".skm").exists());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_workspace_parent_without_external_writes() {
        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), project.path().join(".skm")).unwrap();
        let error = run(
            Mode::Upgrade,
            &config(None),
            project.path(),
            None,
            Some("vendor/workspace-docs".to_string()),
            None,
            None,
            true,
            true,
            true,
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("workspace-managed parent is a symlink"));
        assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
    }

    #[test]
    fn apply_without_confirmation_performs_no_fetch_or_writes() {
        let project = tempfile::tempdir().unwrap();
        let error = run(
            Mode::Upgrade,
            &config(None),
            project.path(),
            None,
            Some("file:///does/not/exist".to_string()),
            Some("a".repeat(40)),
            Some(format!("sha256:{}", "b".repeat(64))),
            true,
            false,
            true,
        )
        .unwrap_err();
        assert!(error.to_string().contains("--apply requires --yes"));
        assert!(!project.path().join(".skm").exists());
    }

    fn git(directory: &Path, arguments: &[&str]) {
        let status = Command::new("git")
            .args(arguments)
            .current_dir(directory)
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn git_output_for_test(directory: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(directory)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }
}
