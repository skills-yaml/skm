use crate::config::SkillSpec;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

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

pub fn get_global_agent_skills_dir(agent: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let dir_name = match agent {
        "claude" => ".claude",
        "codex" => ".codex",
        "cursor" => ".cursor",
        "copilot" => ".copilot",
        "grok" => ".grok",
        "hermes" => ".hermes",
        _ => return None,
    };
    Some(home.join(dir_name).join("skills"))
}

pub fn get_project_agent_skills_dir(agent: &str, project_root: &Path) -> Option<PathBuf> {
    let rel_path = match agent {
        "claude" => ".claude/skills",
        "codex" => ".codex/skills",
        "cursor" => ".cursor/skills",
        "copilot" => ".github/skills",
        "grok" => ".grok/skills",
        "hermes" => ".hermes/skills",
        _ => return None,
    };
    Some(project_root.join(rel_path))
}

pub fn resolve_registry_path(name: &str) -> Option<PathBuf> {
    if !is_safe_registry_name(name) {
        return None;
    }

    let home = dirs::home_dir()?;
    Some(
        home.join(".cache")
            .join("skm")
            .join("registries")
            .join(name),
    )
}

pub fn validate_agents(agents: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    for agent in agents {
        if !is_supported_agent(agent) {
            return Err(format!("Unsupported agent '{}'", agent).into());
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
    let skill_path = validated_skill_path(&skill.name)?;

    if let Some(ref local_path) = skill.path {
        Ok(project_root.join(local_path))
    } else {
        let registry_name = skill.source.as_deref().unwrap_or("default");
        let reg_path = resolve_registry_path(registry_name)
            .ok_or_else(|| format!("Could not resolve path for registry: {}", registry_name))?;

        // Resolve version path
        let version_path = resolve_version_path(skill)?;

        // Append "skills" directory to registry path
        Ok(reg_path.join("skills").join(skill_path).join(version_path))
    }
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

pub fn get_agent_skills_dir(
    agent: &str,
    project_root: &Path,
    global: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if !is_supported_agent(agent) {
        return Err(format!("Unsupported agent '{}'", agent).into());
    }

    let target_base = if global {
        get_global_agent_skills_dir(agent)
    } else {
        get_project_agent_skills_dir(agent, project_root)
    };

    target_base
        .ok_or_else(|| format!("Could not determine skills directory for '{}'", agent).into())
}

pub fn get_skill_target_path(
    base_dir: &Path,
    skill_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(base_dir.join(validated_skill_path(skill_name)?))
}

fn validate_skill_target_parent(
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

    for agent in agents {
        let base_dir = get_agent_skills_dir(agent, project_root, global)?;
        validate_skill_target_parent(&base_dir, &skill.name)?;
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
    validate_agents(agents)?;
    let expected_source = resolve_skill_source_dir(skill, project_root)?;
    let mut targets = Vec::new();

    for agent in agents {
        let base_dir = get_agent_skills_dir(agent, project_root, global)?;
        validate_skill_target_parent(&base_dir, &skill.name)?;
        let skill_path = get_skill_target_path(&base_dir, &skill.name)?;

        match fs::symlink_metadata(&skill_path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                if !force && !symlink_matches_expected(&skill_path, &expected_source)? {
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
    matches!(
        agent,
        "claude" | "codex" | "cursor" | "copilot" | "grok" | "hermes"
    )
}

fn is_safe_registry_name(name: &str) -> bool {
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
        }
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
        let existing = project.join(".codex").join("skills").join("foo");
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
        let target = project.join(".codex").join("skills").join("foo");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        symlink_dir(&other, &target).unwrap();

        link_skill(&skill, &project, &agents, false).unwrap();

        assert!(symlink_points_to(&target, &project.join("source").join("foo")).unwrap());
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn rejects_unknown_agents() {
        let agents = vec!["codxe".to_string()];

        assert!(validate_agents(&agents).is_err());
    }

    #[test]
    fn plans_and_applies_skill_unlink_idempotently() {
        let project = temp_project();
        let skill = local_skill(&project, "foo");
        let agents = vec!["codex".to_string()];
        let target = project.join(".codex").join("skills").join("foo");

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
        let target = project.join(".codex").join("skills").join("foo");
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
        let namespace = project.join(".codex/skills/group");
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
}
