use crate::config::SkillSpec;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

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
            .join(name)
            .join("skills"),
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
        Ok(reg_path.join(skill_path))
    }
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
        let skill_target = get_skill_target_path(&base_dir, &skill.name)?;

        if let Some(parent) = skill_target.parent() {
            fs::create_dir_all(parent)?;
        }

        match fs::symlink_metadata(&skill_target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                if symlink_points_to(&skill_target, &source_dir)? {
                    println!("Already linked {} to {:?}", skill.name, skill_target);
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
        println!("Linked {} to {:?}", skill.name, skill_target);
    }

    Ok(())
}

fn is_supported_agent(agent: &str) -> bool {
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

fn validated_skill_path(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
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
fn symlink_dir(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(windows)]
fn symlink_dir(source: &Path, target: &Path) -> io::Result<()> {
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
}
