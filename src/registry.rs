use crate::config_manager::BaseConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Registry information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryInfo {
    pub name: String,
    pub url: String,
    pub is_default: bool,
    pub is_cached: bool,
    pub is_up_to_date: bool,
    pub cache_path: Option<PathBuf>,
    pub skill_count: Option<usize>,
    pub last_updated: Option<String>,
}

/// Add a new registry
pub fn add(
    name: String,
    url: String,
    set_default: bool,
    skip_validate: bool,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate registry name
    if !is_valid_registry_name(&name) {
        return Err(format!("Invalid registry name: '{}'", name).into());
    }

    // Check if registry already exists
    let mut config = BaseConfig::load()?;
    if config.registries.contains_key(&name) {
        return Err(format!("Registry '{}' already exists", name).into());
    }

    // Validate URL if not skipped
    if !skip_validate {
        validate_registry_url(&url)?;
    }

    // Add registry
    config.registries.insert(name.clone(), url.clone());

    // Set as default if requested
    if set_default {
        config.default_registry = name.clone();
    }

    // Save config
    config.save()?;

    // Output result
    if json_output {
        println!(
            "{{ \"name\": \"{}\", \"url\": \"{}\", \"default\": {} }}",
            name, url, set_default
        );
    } else {
        println!("Added registry '{}' with URL: {}", name, url);
        if set_default {
            println!("Set as default registry");
        }
    }

    Ok(())
}

/// Remove a registry
pub fn remove(
    name: String,
    force: bool,
    yes: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = BaseConfig::load()?;

    // Check if registry exists
    if !config.registries.contains_key(&name) {
        return Err(format!("Registry '{}' not found", name).into());
    }

    // Check if it's the default registry
    let is_default = config.default_registry == name;

    if is_default && !force {
        return Err(format!(
            "Registry '{}' is the default registry. Use --force to remove it.",
            name
        )
        .into());
    }

    if dry_run {
        println!("Would remove registry: {}", name);
        if is_default {
            println!("Warning: This is the default registry");
        }
        return Ok(());
    }

    // Confirm with user if not --yes
    if !yes {
        if is_default {
            print!("Registry '{}' is the default. Remove anyway? [y/N] ", name);
        } else {
            print!("Remove registry '{}'? [y/N] ", name);
        }
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Removal cancelled.");
            return Ok(());
        }
    }

    // Remove registry
    config.registries.remove(&name);

    // If it was the default, set a new default
    if is_default {
        if config.registries.is_empty() {
            // Reset to default
            config.default_registry = "default".to_string();
            config.registries.insert(
                "default".to_string(),
                crate::config_manager::DEFAULT_REGISTRY_URL.to_string(),
            );
        } else {
            // Set first registry as default
            config.default_registry = config.registries.keys().next().unwrap().clone();
        }
    }

    // Save config
    config.save()?;

    // Also remove cache if it exists
    if let Some(path) = crate::linker::resolve_registry_path(&name) {
        if path.exists() {
            fs::remove_dir_all(&path)?;
            println!("Removed cache directory: {}", path.display());
        }
    }

    println!("Removed registry: {}", name);

    Ok(())
}

/// List all registries
pub fn list(json_output: bool, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = BaseConfig::load()?;
    let mut registries: Vec<RegistryInfo> = Vec::new();

    for (name, url) in &config.registries {
        let is_default = *name == config.default_registry;
        let cache_path = crate::linker::resolve_registry_path(name);
        let is_cached = cache_path.as_ref().is_some_and(|p| p.exists());
        let is_up_to_date = if is_cached {
            is_registry_up_to_date(cache_path.as_ref().unwrap()).unwrap_or(false)
        } else {
            false
        };
        let skill_count = if is_cached {
            count_skills(cache_path.as_ref().unwrap()).ok()
        } else {
            None
        };

        registries.push(RegistryInfo {
            name: name.clone(),
            url: url.clone(),
            is_default,
            is_cached,
            is_up_to_date,
            cache_path,
            skill_count,
            last_updated: None,
        });
    }

    // Sort to keep output deterministic
    registries.sort_by(|a, b| a.name.cmp(&b.name));

    if json_output {
        let json = serde_json::to_string_pretty(&registries)
            .map_err(|e| format!("Failed to serialize registries: {}", e))?;
        println!("{}", json);
    } else if verbose {
        for reg in &registries {
            println!("Registry: {}", reg.name);
            println!("  URL: {}", reg.url);
            println!("  Default: {}", if reg.is_default { "Yes" } else { "No" });
            println!("  Cached: {}", if reg.is_cached { "Yes" } else { "No" });
            if reg.is_cached {
                println!(
                    "  Up to date: {}",
                    if reg.is_up_to_date { "Yes" } else { "No" }
                );
            }
            if let Some(count) = reg.skill_count {
                println!("  Skills: {}", count);
            }
            println!();
        }
    } else {
        for reg in &registries {
            let default_marker = if reg.is_default { " (default)" } else { "" };
            let cached_marker = if reg.is_cached { " (cached)" } else { "" };
            println!(
                "{}{}{}: {}",
                reg.name, default_marker, cached_marker, reg.url
            );
        }
    }

    Ok(())
}

/// Update a specific registry
pub fn update(name: String, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = BaseConfig::load()?;

    if !config.registries.contains_key(&name) {
        return Err(format!("Registry '{}' not found", name).into());
    }

    let url = &config.registries[&name];
    let cache_path = crate::linker::resolve_registry_path(&name)
        .ok_or_else(|| format!("Could not resolve cache path for registry: {}", name))?;

    if !cache_path.exists() {
        println!("Cloning registry '{}' from '{}'...", name, url);
        clone_registry(url, &cache_path)?;
    } else if force {
        println!("Updating registry '{}'...", name);
        update_registry(&cache_path)?;
    } else {
        match is_registry_up_to_date(&cache_path) {
            Ok(true) => println!("Registry '{}' is already up-to-date", name),
            _ => {
                println!("Updating registry '{}'...", name);
                update_registry(&cache_path)?;
            }
        }
    }

    println!("Registry '{}' updated successfully", name);

    Ok(())
}

/// Update all registries
pub fn update_all(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = BaseConfig::load()?;
    let mut updated = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for name in config.registries.keys() {
        match update(name.clone(), force) {
            Ok(_) => updated += 1,
            Err(e) if e.to_string().contains("already up-to-date") => skipped += 1,
            Err(e) => {
                eprintln!("Failed to update registry '{}': {}", name, e);
                failed += 1;
            }
        }
    }

    println!("\nSummary:");
    println!("  Updated: {}", updated);
    println!("  Skipped: {}", skipped);
    println!("  Failed: {}", failed);

    if failed > 0 {
        return Err(format!("Failed to update {} registries", failed).into());
    }

    Ok(())
}

/// Set default registry
pub fn set_default(name: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = BaseConfig::load()?;

    if !config.registries.contains_key(&name) {
        return Err(format!("Registry '{}' not found", name).into());
    }

    config.default_registry = name;
    config.save()?;

    println!("Default registry set to: {}", config.default_registry);

    Ok(())
}

/// Show registry info
pub fn info(name: String, json_output: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = BaseConfig::load()?;

    let url = config
        .registries
        .get(&name)
        .ok_or_else(|| format!("Registry '{}' not found", name))?;

    let is_default = config.default_registry == name;
    let cache_path = crate::linker::resolve_registry_path(&name);
    let is_cached = cache_path.as_ref().is_some_and(|p| p.exists());
    let is_up_to_date = if is_cached {
        is_registry_up_to_date(cache_path.as_ref().unwrap()).unwrap_or(false)
    } else {
        false
    };
    let skill_count = if is_cached {
        count_skills(cache_path.as_ref().unwrap()).ok()
    } else {
        None
    };

    let info = RegistryInfo {
        name: name.clone(),
        url: url.clone(),
        is_default,
        is_cached,
        is_up_to_date,
        cache_path: cache_path.clone(),
        skill_count,
        last_updated: None,
    };

    if json_output {
        let json = serde_json::to_string_pretty(&info)
            .map_err(|e| format!("Failed to serialize registry info: {}", e))?;
        println!("{}", json);
    } else {
        println!("Name: {}", info.name);
        println!("URL: {}", info.url);
        println!("Default: {}", if info.is_default { "Yes" } else { "No" });
        println!("Cached: {}", if info.is_cached { "Yes" } else { "No" });
        if info.is_cached {
            println!(
                "Up to date: {}",
                if info.is_up_to_date { "Yes" } else { "No" }
            );
        }
        if let Some(ref path) = info.cache_path {
            println!("Cache Path: {}", path.display());
        }
        if let Some(count) = info.skill_count {
            println!("Skills: {}", count);
        }
    }

    Ok(())
}

// Helper functions

fn is_valid_registry_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !name.starts_with('.')
}

fn validate_registry_url(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if url.starts_with("git@")
        || url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("ssh://")
    {
        Ok(())
    } else {
        Err(format!("Invalid Git URL: {}", url).into())
    }
}

fn clone_registry(url: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let output = std::process::Command::new("git")
        .args(["clone", url, path.to_str().unwrap()])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to clone registry: {}", err).into());
    }

    Ok(())
}

fn update_registry(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(["-C", path.to_str().unwrap(), "pull"])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to update registry: {}", err).into());
    }

    Ok(())
}

fn is_registry_up_to_date(path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    // Attempt check status porcelain
    let output = std::process::Command::new("git")
        .args(["-C", path.to_str().unwrap(), "status", "--porcelain"])
        .output();

    match output {
        Ok(output) => {
            if !output.stdout.is_empty() {
                // Modified locally
                return Ok(false);
            }
        }
        Err(_) => return Ok(false),
    }

    Ok(true)
}

fn count_skills(path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    let skills_dir = path.join("skills");
    if !skills_dir.exists() {
        return Ok(0);
    }

    let mut skill_dirs = std::collections::HashSet::new();
    for entry in walkdir::WalkDir::new(&skills_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if p.file_name().is_some_and(|name| name == "SKILL.md") {
            if let Some(version_dir) = p.parent() {
                if let Some(skill_dir) = version_dir.parent() {
                    skill_dirs.insert(skill_dir.to_path_buf());
                }
            }
        }
    }
    Ok(skill_dirs.len())
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
        fn new(new_dir: &std::path::Path) -> Self {
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
            "skm-test-registry-{}-{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    #[serial]
    fn test_registry_add_list_default_remove() {
        let temp = temp_project();
        let _guard = CurrentDirGuard::new(&temp);
        let home_temp = temp.join("mock_home");
        fs::create_dir_all(&home_temp).unwrap();
        let _home_guard = EnvVarGuard::new("HOME", home_temp.to_str().unwrap());
        let _xdg_config_guard = EnvVarGuard::new(
            "XDG_CONFIG_HOME",
            home_temp.join(".config").to_str().unwrap(),
        );

        // Initialize base config
        let base_config = BaseConfig::new();
        base_config.save().unwrap();

        // 1. Add custom registry
        add(
            "custom".to_string(),
            "https://github.com/example/custom.git".to_string(),
            false,
            true,
            false,
        )
        .unwrap();

        // Verify it was added
        let config = BaseConfig::load().unwrap();
        assert!(config.registries.contains_key("custom"));
        assert_eq!(config.default_registry, "default");

        // 2. Set default registry
        set_default("custom".to_string()).unwrap();
        let config = BaseConfig::load().unwrap();
        assert_eq!(config.default_registry, "custom");

        // 3. List registries
        list(false, false).unwrap();
        list(true, false).unwrap(); // JSON
        list(false, true).unwrap(); // Verbose

        // 4. Show info
        info("custom".to_string(), false).unwrap();
        info("custom".to_string(), true).unwrap(); // JSON

        // 5. Try removing default registry without force (should fail)
        assert!(remove("custom".to_string(), false, true, false).is_err());

        // 6. Remove with force
        remove("custom".to_string(), true, true, false).unwrap();
        let config = BaseConfig::load().unwrap();
        assert!(!config.registries.contains_key("custom"));

        fs::remove_dir_all(temp).unwrap();
    }
}
