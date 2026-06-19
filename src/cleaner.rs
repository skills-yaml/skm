use crate::config::SkillsConfig;
use crate::config_manager::{get_base_config_path, get_cache_dir, BaseConfig};
use crate::linker;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Symlink status
#[derive(Debug, Clone, PartialEq)]
pub enum SymlinkStatus {
    Valid,
    Broken,
    Orphaned,
}

/// Symlink information
#[derive(Debug, Clone)]
pub struct SymlinkInfo {
    pub path: PathBuf,
    pub target: PathBuf,
    pub agent: String,
    pub skill_name: String,
    pub status: SymlinkStatus,
}

/// Clean up broken and orphaned symlinks
pub fn clean_symlinks(
    global: bool,
    broken: bool,
    orphaned: bool,
    all: bool,
    dry_run: bool,
    yes: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;

    // Determine what to clean
    let clean_broken = broken || all;
    let clean_orphaned = orphaned || all;

    if !clean_broken && !clean_orphaned {
        return Err("Must specify at least one of --broken, --orphaned, or --all".into());
    }

    // Get project configuration if not global
    let config_skills: Vec<String> = if global {
        Vec::new()
    } else {
        let config_path = current_dir.join("skills.yaml");
        if config_path.exists() {
            let config = SkillsConfig::load_from_file(&config_path)?;
            config.skills.iter().map(|s| s.name.clone()).collect()
        } else {
            Vec::new()
        }
    };

    // Get base config for global skills
    let base_config = BaseConfig::load().ok();
    let all_skills: Vec<String> = if global {
        base_config
            .as_ref()
            .map_or(Vec::new(), |c| c.registries.keys().cloned().collect())
    } else {
        config_skills
    };

    // Find all symlinks
    let agents = if global {
        ["claude", "cursor", "codex", "copilot", "grok", "hermes"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        let config_path = current_dir.join("skills.yaml");
        if config_path.exists() {
            let config = SkillsConfig::load_from_file(&config_path)?;
            config.agents
        } else {
            Vec::new()
        }
    };

    let mut symlinks_to_clean: Vec<SymlinkInfo> = Vec::new();

    for agent in &agents {
        let base_dir = if global {
            linker::get_global_agent_skills_dir(agent)
        } else {
            linker::get_project_agent_skills_dir(agent, &current_dir)
        };

        let Some(base_dir) = base_dir else {
            continue;
        };

        if !base_dir.exists() {
            if verbose {
                eprintln!("Agent directory does not exist: {}", base_dir.display());
            }
            continue;
        }

        // Recursively find all symlinks in this agent directory using walkdir
        for entry in walkdir::WalkDir::new(&base_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_symlink() {
                let target = match fs::read_link(path) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let rel_path = match path.strip_prefix(&base_dir) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let skill_name = rel_path.to_string_lossy().to_string();

                let target_abs = if target.is_absolute() {
                    target.clone()
                } else {
                    path.parent().unwrap_or(Path::new(".")).join(&target)
                };
                let is_broken = !target_abs.exists();

                let status = if is_broken {
                    SymlinkStatus::Broken
                } else if clean_orphaned && !all_skills.contains(&skill_name) {
                    SymlinkStatus::Orphaned
                } else {
                    SymlinkStatus::Valid
                };

                if (clean_broken && status == SymlinkStatus::Broken)
                    || (clean_orphaned && status == SymlinkStatus::Orphaned)
                {
                    symlinks_to_clean.push(SymlinkInfo {
                        path: path.to_path_buf(),
                        target,
                        agent: agent.clone(),
                        skill_name,
                        status,
                    });
                }
            }
        }
    }

    if symlinks_to_clean.is_empty() {
        eprintln!("No symlinks to clean");
        return Ok(());
    }

    if dry_run {
        println!("Would clean {} symlinks:", symlinks_to_clean.len());
        for info in &symlinks_to_clean {
            let status = match info.status {
                SymlinkStatus::Broken => "broken",
                SymlinkStatus::Orphaned => "orphaned",
                _ => "unknown",
            };
            println!(
                "  [{}] {} -> {} (agent: {}, skill: {})",
                status,
                info.path.display(),
                info.target.display(),
                info.agent,
                info.skill_name
            );
        }
        return Ok(());
    }

    // Confirm with user
    if !yes {
        eprintln!("Found {} symlinks to clean:", symlinks_to_clean.len());
        for info in &symlinks_to_clean {
            let status = match info.status {
                SymlinkStatus::Broken => "broken",
                SymlinkStatus::Orphaned => "orphaned",
                _ => "unknown",
            };
            eprintln!(
                "  [{}] {} -> {}",
                status,
                info.path.display(),
                info.target.display()
            );
        }
        eprint!("\nClean these symlinks? [y/N] ");
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            eprintln!("Cleanup cancelled.");
            return Ok(());
        }
    }

    // Clean the symlinks
    let mut cleaned = 0;
    for info in &symlinks_to_clean {
        if verbose {
            eprintln!("Removing: {}", info.path.display());
        }
        fs::remove_file(&info.path)?;
        cleaned += 1;
    }

    eprintln!("Cleaned {} symlinks", cleaned);

    Ok(())
}

/// Clean up registry cache
#[allow(clippy::too_many_arguments)]
pub fn clean_cache(
    all: bool,
    old_versions: bool,
    keep: usize,
    dry_run: bool,
    yes: bool,
    stats: bool,
    verbose: bool,
    registry: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if stats {
        return show_cache_stats(registry);
    }

    let base_config = BaseConfig::load()?;

    let registries_to_clean: Vec<String> = if all {
        base_config.registries.keys().cloned().collect()
    } else if let Some(ref name) = registry {
        if !base_config.registries.contains_key(name) {
            return Err(format!("Registry '{}' not found", name).into());
        }
        vec![name.clone()]
    } else {
        return Err("Must specify a registry or use --all".into());
    };

    if old_versions {
        return clean_old_versions(&registries_to_clean, keep, dry_run, yes, verbose);
    }

    // Full cache cleanup
    let mut total_size = 0;
    let mut total_removed = 0;
    let mut paths_to_remove = Vec::new();

    for reg_name in &registries_to_clean {
        let cache_path = linker::resolve_registry_path(reg_name)
            .ok_or_else(|| format!("Could not resolve path for registry: {}", reg_name))?;

        if !cache_path.exists() {
            if verbose {
                eprintln!(
                    "Cache for '{}' does not exist: {}",
                    reg_name,
                    cache_path.display()
                );
            }
            continue;
        }

        // Calculate size
        let size = calculate_directory_size(&cache_path)?;
        total_size += size;
        paths_to_remove.push((reg_name.clone(), cache_path, size));
    }

    if paths_to_remove.is_empty() {
        eprintln!("No registry caches to clean");
        return Ok(());
    }

    if dry_run {
        println!("Would clean the following registry caches:");
        for (reg_name, cache_path, size) in &paths_to_remove {
            println!(
                "  - {} ({}): {} ({} MB)",
                reg_name,
                cache_path.display(),
                format_size(*size),
                format_size_mb(*size)
            );
        }
        println!(
            "\nWould clean {} registry caches ({} bytes total)",
            paths_to_remove.len(),
            total_size
        );
        return Ok(());
    }

    // Confirm with user
    if !yes {
        eprintln!("Found {} registry caches to clean:", paths_to_remove.len());
        for (reg_name, cache_path, size) in &paths_to_remove {
            eprintln!(
                "  - {}: {} ({} MB)",
                reg_name,
                cache_path.display(),
                format_size_mb(*size)
            );
        }
        eprint!("\nClean these caches? [y/N] ");
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            eprintln!("Cleanup cancelled.");
            return Ok(());
        }
    }

    for (reg_name, cache_path, _) in &paths_to_remove {
        if verbose {
            eprintln!(
                "Removing cache for '{}': {}",
                reg_name,
                cache_path.display()
            );
        }
        fs::remove_dir_all(cache_path)?;
        total_removed += 1;
    }

    eprintln!(
        "Cleaned {} registry caches ({} freed)",
        total_removed,
        format_size(total_size)
    );

    Ok(())
}

/// Clean old skill versions
pub fn clean_old_versions(
    registries: &[String],
    keep: usize,
    dry_run: bool,
    yes: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut total_freed = 0;
    let mut versions_removed = 0;
    let mut versions_to_remove = Vec::new();

    for reg_name in registries {
        let cache_path = linker::resolve_registry_path(reg_name)
            .ok_or_else(|| format!("Could not resolve path for registry: {}", reg_name))?;

        if !cache_path.exists() {
            if verbose {
                eprintln!("Cache for '{}' does not exist", reg_name);
            }
            continue;
        }

        let skills_dir = cache_path.join("skills");
        if !skills_dir.exists() {
            continue;
        }

        // Find all version directories by finding SKILL.md files
        let mut skill_to_versions: std::collections::HashMap<PathBuf, Vec<PathBuf>> =
            std::collections::HashMap::new();

        for entry in walkdir::WalkDir::new(&skills_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.file_name().is_some_and(|name| name == "SKILL.md") {
                if let Some(version_dir) = path.parent() {
                    let version_name = version_dir
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    // Skip 'latest' and 'default' from cleanup candidates
                    if version_name == "latest" || version_name == "default" {
                        continue;
                    }
                    if let Some(skill_dir) = version_dir.parent() {
                        skill_to_versions
                            .entry(skill_dir.to_path_buf())
                            .or_default()
                            .push(version_dir.to_path_buf());
                    }
                }
            }
        }

        for (_, mut versions) in skill_to_versions {
            if versions.len() > keep {
                // Sort by modification time (oldest first)
                versions.sort_by(|a, b| {
                    let a_time = a
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or_else(|_| SystemTime::now());
                    let b_time = b
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or_else(|_| SystemTime::now());
                    a_time.cmp(&b_time)
                });

                let to_remove = &versions[..versions.len() - keep];
                for version_path in to_remove {
                    let size = calculate_directory_size(version_path)?;
                    versions_to_remove.push((version_path.clone(), size));
                }
            }
        }
    }

    if versions_to_remove.is_empty() {
        eprintln!("No old versions to remove");
        return Ok(());
    }

    if dry_run {
        println!("Would remove the following old versions:");
        for (version_path, size) in &versions_to_remove {
            println!("  - {} ({} bytes)", version_path.display(), size);
        }
        println!(
            "\nWould remove {} old versions ({} bytes total)",
            versions_to_remove.len(),
            total_freed
        );
        return Ok(());
    }

    // Confirm with user
    if !yes {
        eprintln!("Found {} old versions to remove:", versions_to_remove.len());
        for (version_path, size) in &versions_to_remove {
            eprintln!("  - {} ({} bytes)", version_path.display(), size);
        }
        eprint!("\nClean these old versions? [y/N] ");
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            eprintln!("Cleanup cancelled.");
            return Ok(());
        }
    }

    for (version_path, size) in &versions_to_remove {
        if verbose {
            eprintln!("Removing old version: {}", version_path.display());
        }
        fs::remove_dir_all(version_path)?;
        total_freed += size;
        versions_removed += 1;
    }

    eprintln!(
        "Removed {} old versions ({} freed)",
        versions_removed,
        format_size(total_freed)
    );

    Ok(())
}

/// Reset SKM to clean state
#[allow(clippy::too_many_arguments)]
pub fn reset(
    config: bool,
    cache: bool,
    symlinks: bool,
    all: bool,
    backup: bool,
    backup_dir: Option<String>,
    dry_run: bool,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;

    // Determine what to reset
    let reset_config = config || all;
    let reset_cache = cache || all;
    let reset_symlinks = symlinks || all;

    if !reset_config && !reset_cache && !reset_symlinks {
        return Err("Must specify at least one of --config, --cache, --symlinks, or --all".into());
    }

    // Determine backup directory
    let backup_dir = if backup {
        let dir = match backup_dir {
            Some(path_str) => PathBuf::from(path_str),
            None => dirs::home_dir()
                .map(|d| d.join(".skm-backups"))
                .unwrap_or_else(|| PathBuf::from(".skm-backups")),
        };
        fs::create_dir_all(&dir)?;
        Some(dir)
    } else {
        None
    };

    // Collect items to reset
    let mut items: Vec<(String, PathBuf, String)> = Vec::new();

    if reset_config {
        if let Some(path) = get_base_config_path() {
            if path.exists() {
                items.push((
                    "Global configuration".to_string(),
                    path,
                    "config".to_string(),
                ));
            }
        }

        let project_config = current_dir.join("skills.yaml");
        if project_config.exists() {
            items.push((
                "Project configuration".to_string(),
                project_config,
                "config".to_string(),
            ));
        }
    }

    if reset_cache {
        if let Some(cache_dir) = get_cache_dir() {
            if cache_dir.exists() {
                items.push((
                    "Cache directory".to_string(),
                    cache_dir,
                    "cache".to_string(),
                ));
            }
        }
    }

    if reset_symlinks {
        let agents = ["claude", "cursor", "codex", "copilot", "grok", "hermes"];
        for agent in &agents {
            if let Some(dir) = linker::get_global_agent_skills_dir(agent) {
                if dir.exists() {
                    items.push((
                        format!("Global {} symlinks", agent),
                        dir,
                        "symlinks".to_string(),
                    ));
                }
            }
        }

        let project_config = current_dir.join("skills.yaml");
        if project_config.exists() {
            let config = SkillsConfig::load_from_file(&project_config).ok();
            if let Some(config) = config {
                for agent in &config.agents {
                    if let Some(dir) = linker::get_project_agent_skills_dir(agent, &current_dir) {
                        if dir.exists() {
                            items.push((
                                format!("Project {} symlinks", agent),
                                dir,
                                "symlinks".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }

    if items.is_empty() {
        eprintln!("No items to reset");
        return Ok(());
    }

    if dry_run {
        println!("Would reset the following items:");
        for (name, path, _) in &items {
            println!("  - {}: {}", name, path.display());
        }
        if let Some(ref dir) = backup_dir {
            println!("\nBackups would be created in: {}", dir.display());
        }
        return Ok(());
    }

    // Confirm with user
    if !yes {
        eprintln!("About to reset the following items:");
        for (name, path, _) in &items {
            eprintln!("  - {}: {}", name, path.display());
        }
        if let Some(ref dir) = backup_dir {
            eprintln!("\nBackups will be created in: {}", dir.display());
        }
        eprint!("\nReset these items? [y/N] ");
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            eprintln!("Reset cancelled.");
            return Ok(());
        }
    }

    // Perform reset with optional backup
    let mut reset_count = 0;
    for (name, path, _) in &items {
        if let Some(ref backup_dir) = backup_dir {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let backup_path = backup_dir.join(format!(
                "{}_{}.backup",
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown"),
                timestamp
            ));

            if path.is_dir() {
                copy_dir_all(path, &backup_path)?;
            } else {
                fs::copy(path, &backup_path)?;
            }
            eprintln!("Backed up {} to {}", name, backup_path.display());
        }

        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        eprintln!("Reset: {}", name);
        reset_count += 1;
    }

    eprintln!("\nReset {} items", reset_count);

    if reset_config {
        eprintln!("\nTo reinitialize, run:");
        eprintln!("  skm setup");
    }

    Ok(())
}

/// Show cache statistics
pub fn show_cache_stats(registry: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let base_config = BaseConfig::load()?;

    let registries: Vec<String> = if let Some(ref name) = registry {
        if !base_config.registries.contains_key(name) {
            return Err(format!("Registry '{}' not found", name).into());
        }
        vec![name.clone()]
    } else {
        base_config.registries.keys().cloned().collect()
    };

    let mut total_size = 0;
    let mut total_skills = 0;
    let mut total_versions = 0;

    println!("Cache Statistics:");
    println!("{}", "=".repeat(50));

    for reg_name in &registries {
        let cache_path = linker::resolve_registry_path(reg_name)
            .ok_or_else(|| format!("Could not resolve path for registry: {}", reg_name))?;

        let (size, skill_count, version_count) = if cache_path.exists() {
            let size = calculate_directory_size(&cache_path)?;
            let skills_dir = cache_path.join("skills");

            let mut skill_dirs = std::collections::HashSet::new();
            let mut version_dirs = std::collections::HashSet::new();

            if skills_dir.exists() {
                for entry in walkdir::WalkDir::new(&skills_dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let path = entry.path();
                    if path.file_name().is_some_and(|name| name == "SKILL.md") {
                        if let Some(version_dir) = path.parent() {
                            version_dirs.insert(version_dir.to_path_buf());
                            if let Some(skill_dir) = version_dir.parent() {
                                skill_dirs.insert(skill_dir.to_path_buf());
                            }
                        }
                    }
                }
            }

            (size, skill_dirs.len(), version_dirs.len())
        } else {
            (0, 0, 0)
        };

        total_size += size;
        total_skills += skill_count;
        total_versions += version_count;

        println!("\nRegistry: {}", reg_name);
        println!("  Path: {}", cache_path.display());
        println!(
            "  Size: {} ({} MB)",
            format_size(size),
            format_size_mb(size)
        );
        println!(
            "  Cached: {}",
            if cache_path.exists() { "Yes" } else { "No" }
        );
        if cache_path.exists() {
            println!("  Skills: {}", skill_count);
            println!("  Versions: {}", version_count);
        }
    }

    println!("\n{}", "-".repeat(50));
    println!(
        "Total: {} registries, {} MB, {} skills, {} versions",
        registries.len(),
        format_size_mb(total_size),
        total_skills,
        total_versions
    );

    Ok(())
}

// Helper functions

/// Calculate total size of a directory
fn calculate_directory_size(path: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    let mut size = 0;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            size += calculate_directory_size(&entry.path())?;
        } else {
            size += metadata.len();
        }
    }

    Ok(size)
}

/// Format size in human-readable format
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Format size in MB
fn format_size_mb(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    format!("{:.2}", bytes as f64 / MB as f64)
}

/// Copy a directory recursively
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !src.exists() {
        return Ok(());
    }

    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            copy_dir_all(&src_path, &dst_path)?;
        }
    } else {
        fs::copy(src, dst)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct CurrentDirGuard {
        original: PathBuf,
    }

    impl CurrentDirGuard {
        fn new(new_dir: &Path) -> Self {
            let original = std::env::current_dir().unwrap();
            std::env::set_current_dir(new_dir).unwrap();
            Self { original }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }

    struct EnvVarGuard {
        key: String,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn new(key: &str, new_value: &str) -> Self {
            let original = std::env::var(key).ok();
            std::env::set_var(key, new_value);
            Self {
                key: key.to_string(),
                original,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(ref val) = self.original {
                std::env::set_var(&self.key, val);
            } else {
                std::env::remove_var(&self.key);
            }
        }
    }

    fn temp_project() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "skm-test-cleaner-{}-{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    #[serial]
    fn test_clean_symlinks() {
        let temp = temp_project();
        let _guard = CurrentDirGuard::new(&temp);
        let home_temp = temp.join("mock_home");
        fs::create_dir_all(&home_temp).unwrap();
        let _home_guard = EnvVarGuard::new("HOME", home_temp.to_str().unwrap());

        // Create skills.yaml in project dir
        let config = SkillsConfig {
            name: "test-project".to_string(),
            version: Some("0.1.0".to_string()),
            agents: vec!["claude".to_string()],
            skills: vec![crate::config::SkillSpec {
                name: "software-development/spec".to_string(),
                version: Some("latest".to_string()),
                source: None,
                path: None,
            }],
            registries: None,
        };
        config.save_to_file(&temp.join("skills.yaml")).unwrap();

        // Create simulated agent skill directories
        let agent_dir = temp
            .join(".claude")
            .join("skills")
            .join("software-development");
        fs::create_dir_all(&agent_dir).unwrap();

        // 1. Create a valid symlink target (simulated skill source dir)
        let skill_src = temp
            .join("src_skills")
            .join("software-development")
            .join("spec");
        fs::create_dir_all(&skill_src).unwrap();
        fs::write(skill_src.join("SKILL.md"), "# Test Skill").unwrap();

        // Create valid symlink
        let valid_link = agent_dir.join("spec");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&skill_src, &valid_link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&skill_src, &valid_link).unwrap();

        // 2. Create a broken symlink (points to non-existent path)
        let broken_link = agent_dir.join("broken");
        let non_existent = temp.join("non_existent");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&non_existent, &broken_link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&non_existent, &broken_link).unwrap();

        // 3. Create an orphaned symlink (points to valid path, but skill name not in config)
        let orphaned_src = temp
            .join("src_skills")
            .join("software-development")
            .join("orphaned");
        fs::create_dir_all(&orphaned_src).unwrap();
        fs::write(orphaned_src.join("SKILL.md"), "# Orphaned").unwrap();
        let orphaned_link = agent_dir.join("orphaned");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&orphaned_src, &orphaned_link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&orphaned_src, &orphaned_link).unwrap();

        // Run dry run clean on broken symlinks
        clean_symlinks(false, true, false, false, true, true, false).unwrap();
        assert!(broken_link.exists() || broken_link.is_symlink());

        // Clean broken symlinks (actually delete)
        clean_symlinks(false, true, false, false, false, true, false).unwrap();
        assert!(!broken_link.exists() && !broken_link.is_symlink());
        assert!(orphaned_link.exists() || orphaned_link.is_symlink());

        // Clean orphaned symlinks
        clean_symlinks(false, false, true, false, false, true, false).unwrap();
        assert!(!orphaned_link.exists() && !orphaned_link.is_symlink());
        assert!(valid_link.exists() || valid_link.is_symlink());

        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    #[serial]
    fn test_clean_cache() {
        let temp = temp_project();
        let _guard = CurrentDirGuard::new(&temp);
        let home_temp = temp.join("mock_home");
        fs::create_dir_all(&home_temp).unwrap();
        let _home_guard = EnvVarGuard::new("HOME", home_temp.to_str().unwrap());
        let _xdg_config_guard = EnvVarGuard::new(
            "XDG_CONFIG_HOME",
            home_temp.join(".config").to_str().unwrap(),
        );
        let _xdg_cache_guard =
            EnvVarGuard::new("XDG_CACHE_HOME", home_temp.join(".cache").to_str().unwrap());

        // Create base config to register registry
        let base_config = BaseConfig::new();
        base_config.save().unwrap();

        // Create default registry cache directory
        let cache_dir = get_cache_dir().unwrap().join("registries").join("default");
        let skills_dir = cache_dir
            .join("skills")
            .join("software-development")
            .join("spec");
        fs::create_dir_all(&skills_dir).unwrap();

        // Create a few version directories
        let v1 = skills_dir.join("v1");
        let v2 = skills_dir.join("v2");
        let v3 = skills_dir.join("v3");
        fs::create_dir_all(&v1).unwrap();
        fs::create_dir_all(&v2).unwrap();
        fs::create_dir_all(&v3).unwrap();
        fs::write(v1.join("SKILL.md"), "# Version 1").unwrap();
        fs::write(v2.join("SKILL.md"), "# Version 2").unwrap();
        fs::write(v3.join("SKILL.md"), "# Version 3").unwrap();

        // Verify cache stats works and doesn't fail
        show_cache_stats(None).unwrap();

        // Clean cache - keep 2 versions (v1, v2, v3). We keep the 2 most recent.
        clean_cache(true, true, 2, false, true, false, false, None).unwrap();
        let versions_count = fs::read_dir(&skills_dir).unwrap().count();
        assert_eq!(versions_count, 2);

        // Now clean all caches
        clean_cache(true, false, 0, false, true, false, false, None).unwrap();
        assert!(!cache_dir.exists());

        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    #[serial]
    fn test_reset() {
        let temp = temp_project();
        let _guard = CurrentDirGuard::new(&temp);
        let home_temp = temp.join("mock_home");
        fs::create_dir_all(&home_temp).unwrap();
        let _home_guard = EnvVarGuard::new("HOME", home_temp.to_str().unwrap());
        let _xdg_config_guard = EnvVarGuard::new(
            "XDG_CONFIG_HOME",
            home_temp.join(".config").to_str().unwrap(),
        );
        let _xdg_cache_guard =
            EnvVarGuard::new("XDG_CACHE_HOME", home_temp.join(".cache").to_str().unwrap());

        // Create base config
        let base_config = BaseConfig::new();
        base_config.save().unwrap();
        let global_config_path = get_base_config_path().unwrap();
        assert!(global_config_path.exists());

        // Create skills.yaml in project dir
        let config = SkillsConfig {
            name: "test-project".to_string(),
            version: Some("0.1.0".to_string()),
            agents: vec!["claude".to_string()],
            skills: vec![],
            registries: None,
        };
        let project_config_path = temp.join("skills.yaml");
        config.save_to_file(&project_config_path).unwrap();
        assert!(project_config_path.exists());

        // Create cache dir
        let cache_dir = get_cache_dir().unwrap();
        fs::create_dir_all(&cache_dir).unwrap();
        assert!(cache_dir.exists());

        // Create global & project agent symlink directory (simulated)
        let global_agent_dir = home_temp.join(".claude").join("skills");
        fs::create_dir_all(&global_agent_dir).unwrap();
        assert!(global_agent_dir.exists());

        let project_agent_dir = temp.join(".claude").join("skills");
        fs::create_dir_all(&project_agent_dir).unwrap();
        assert!(project_agent_dir.exists());

        // Run reset on config files (with backup)
        let backup_dir = temp.join("backups");
        reset(
            true,
            false,
            false,
            false,
            true,
            Some(backup_dir.to_str().unwrap().to_string()),
            false,
            true,
        )
        .unwrap();

        // Check configs are deleted
        assert!(!global_config_path.exists());
        assert!(!project_config_path.exists());
        // Backup files should exist in backup_dir
        assert!(backup_dir.exists());

        // Run reset on remaining (cache and symlinks)
        reset(false, true, true, false, false, None, false, true).unwrap();
        assert!(!cache_dir.exists());

        fs::remove_dir_all(temp).unwrap();
    }
}
