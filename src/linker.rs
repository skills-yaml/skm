use std::path::{Path, PathBuf};
use std::fs;
use std::os::unix::fs::symlink;
use crate::config::SkillSpec;

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
    // Fallback optimization for user's specific environment
    if name == "default" {
        let fallback = PathBuf::from("/home/e/work/projects/agents/skills");
        if fallback.exists() {
            return Some(fallback);
        }
    }
    
    let home = dirs::home_dir()?;
    Some(home.join(".cache").join("skm").join("registries").join(name).join("skills"))
}

pub fn link_skill(
    skill: &SkillSpec,
    project_root: &Path,
    agents: &[String],
    global: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Resolve source directory of the skill
    let source_dir = if let Some(ref local_path) = skill.path {
        project_root.join(local_path)
    } else {
        let registry_name = skill.source.as_deref().unwrap_or("default");
        let reg_path = resolve_registry_path(registry_name)
            .ok_or_else(|| format!("Could not resolve path for registry: {}", registry_name))?;
        reg_path.join(&skill.name)
    };

    if !source_dir.exists() {
        return Err(format!("Skill source path does not exist: {:?}", source_dir).into());
    }

    // Ensure it contains a SKILL.md file
    if !source_dir.join("SKILL.md").exists() {
        return Err(format!("Missing SKILL.md in: {:?}", source_dir).into());
    }

    // 2. Link to target dirs
    for agent in agents {
        let target_base = if global {
            get_global_agent_skills_dir(agent)
        } else {
            get_project_agent_skills_dir(agent, project_root)
        };

        if let Some(base_dir) = target_base {
            let skill_target = base_dir.join(&skill.name);
            
            // Ensure parent directory of skill_target exists
            if let Some(parent) = skill_target.parent() {
                fs::create_dir_all(parent)?;
            }

            // Remove existing link/file if it exists
            if skill_target.exists() || skill_target.is_symlink() {
                if skill_target.is_dir() && !skill_target.is_symlink() {
                    fs::remove_dir_all(&skill_target)?;
                } else {
                    fs::remove_file(&skill_target)?;
                }
            }

            // Create symlink
            symlink(&source_dir, &skill_target)?;
            println!("Linked {} to {:?}", skill.name, skill_target);
        }
    }

    Ok(())
}

pub fn unlink_skill(
    skill: &SkillSpec,
    project_root: &Path,
    agents: &[String],
    global: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    for agent in agents {
        let target_base = if global {
            get_global_agent_skills_dir(agent)
        } else {
            get_project_agent_skills_dir(agent, project_root)
        };

        if let Some(base_dir) = target_base {
            let skill_target = base_dir.join(&skill.name);
            if skill_target.exists() || skill_target.is_symlink() {
                if skill_target.is_dir() && !skill_target.is_symlink() {
                    fs::remove_dir_all(&skill_target)?;
                } else {
                    fs::remove_file(&skill_target)?;
                }
                println!("Unlinked {} from {:?}", skill.name, skill_target);
            }
        }
    }
    Ok(())
}
