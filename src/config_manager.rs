use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Default registry URL
pub const DEFAULT_REGISTRY_URL: &str = "git@github.com:skills-yaml/skills-registry.git";

/// Get the SKM config directory path
pub fn get_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("skm"))
}

/// Get the global base config path
pub fn get_base_config_path() -> Option<PathBuf> {
    get_config_dir().map(|d| d.join("config.yaml"))
}

/// Get the cache directory path
fn get_cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("skm"))
}

/// Get the registries cache directory
fn get_registries_cache_dir() -> Option<PathBuf> {
    get_cache_dir().map(|d| d.join("registries"))
}

/// Base configuration for SKM (stored in user's config directory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseConfig {
    pub default_registry: String,
    pub registries: HashMap<String, String>,
}

impl BaseConfig {
    pub fn new() -> Self {
        let mut registries = HashMap::new();
        registries.insert("default".to_string(), DEFAULT_REGISTRY_URL.to_string());

        Self {
            default_registry: "default".to_string(),
            registries,
        }
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = get_base_config_path().ok_or("Could not determine config directory")?;

        if !path.exists() {
            // Return default config if file doesn't exist
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&path)?;
        let config: serde_yaml::Value = serde_yaml::from_str(&content)?;

        let default_registry = config
            .get("default_registry")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let mut registries = HashMap::new();
        if let Some(serde_yaml::Value::Mapping(map)) = config.get("registries") {
            for (key, value) in map {
                if let (Some(k), Some(v)) = (key.as_str(), value.as_str()) {
                    registries.insert(k.to_string(), v.to_string());
                }
            }
        }

        // Ensure default registry exists
        if !registries.contains_key(&default_registry) {
            registries.insert(default_registry.clone(), DEFAULT_REGISTRY_URL.to_string());
        }

        Ok(Self {
            default_registry,
            registries,
        })
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = get_base_config_path().ok_or("Could not determine config directory")?;

        // Create config directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = fs::File::create(&path)?;
        serde_yaml::to_writer(file, self)?;

        Ok(())
    }
}

/// Initialize the base configuration in user's home
pub fn init_base_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = BaseConfig::new();
    config.save()?;
    println!(
        "Created base configuration at: {:?}",
        get_base_config_path()
    );
    Ok(())
}

/// Ensure base config exists, create if it doesn't
pub fn ensure_base_config() -> Result<BaseConfig, Box<dyn std::error::Error>> {
    let path = get_base_config_path().ok_or("Could not determine config directory")?;

    if !path.exists() {
        let config = BaseConfig::new();
        config.save()?;
        Ok(config)
    } else {
        BaseConfig::load()
    }
}

/// Update the local cache of registries
pub fn update_cache(registry_name: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let base_config = ensure_base_config()?;

    let registries_to_update: Vec<&str> = if let Some(name) = registry_name {
        if base_config.registries.contains_key(name) {
            vec![name]
        } else {
            return Err(format!("Registry '{}' not found in configuration", name).into());
        }
    } else {
        // Update all registries
        base_config.registries.keys().map(|s| s.as_str()).collect()
    };

    for reg_name in registries_to_update {
        let url = base_config
            .registries
            .get(reg_name)
            .ok_or_else(|| format!("Registry '{}' not found", reg_name))?;

        let cache_path = crate::linker::resolve_registry_path(reg_name)
            .ok_or_else(|| format!("Could not resolve cache path for registry: {}", reg_name))?;

        // Create parent directories
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if cache_path.exists() {
            println!("Updating registry '{}'...", reg_name);
            // Pull latest changes
            let output = std::process::Command::new("git")
                .args(["-C", cache_path.to_str().unwrap(), "pull"])
                .output()?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to update registry '{}': {}", reg_name, err).into());
            }
            println!("Registry '{}' updated successfully.", reg_name);
        } else {
            println!("Cloning registry '{}' from '{}'...", reg_name, url);
            let output = std::process::Command::new("git")
                .args(["clone", url, cache_path.to_str().unwrap()])
                .output()?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to clone registry '{}': {}", reg_name, err).into());
            }
            println!("Registry '{}' cloned successfully.", reg_name);
        }
    }

    Ok(())
}

/// Ensure base configuration exists, create with defaults if missing
/// This is separate from first_time_setup() which also updates cache
pub fn ensure_global_env() -> Result<(), Box<dyn std::error::Error>> {
    let path = get_base_config_path().ok_or("Could not determine config directory")?;

    if !path.exists() {
        println!("Initializing global SKM configuration...");
        init_base_config()?;
    }

    Ok(())
}

/// First-time setup: initialize base config and cache default registry
pub fn first_time_setup() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running first-time setup for SKM...");

    // Initialize base config
    ensure_global_env()?;

    // Update cache for default registry
    update_cache(Some("default"))?;

    println!("\nFirst-time setup completed!");
    println!("Run 'skm init' to create a project configuration.");

    Ok(())
}

/// Check if this is the first time running SKM (deprecated: use ensure_global_env() instead)
#[deprecated(
    note = "Use ensure_global_env() instead. This function checks both config and cache, but global env should always be ensured."
)]
pub fn is_first_time() -> bool {
    let base_config_path = get_base_config_path();
    let registries_cache_dir = get_registries_cache_dir();

    base_config_path.is_none_or(|p| !p.exists()) || registries_cache_dir.is_none_or(|p| !p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_temp_config_dir() -> tempfile::TempDir {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", temp_dir.path());
        temp_dir
    }

    #[test]
    fn test_ensure_global_env_creates_config() {
        let _temp_dir = create_temp_config_dir();

        // Ensure we're starting fresh
        let config_path = get_base_config_path().unwrap();
        let parent = config_path.parent().unwrap();
        if parent.exists() {
            fs::remove_dir_all(parent).ok();
        }

        // Run ensure_global_env
        ensure_global_env().unwrap();

        // Verify config was created
        assert!(config_path.exists());

        // Verify it has the correct default values
        let config = BaseConfig::load().unwrap();
        assert_eq!(config.default_registry, "default");
        assert_eq!(
            config.registries.get("default").unwrap(),
            DEFAULT_REGISTRY_URL
        );
    }

    #[test]
    fn test_ensure_global_env_idempotent() {
        let _temp_dir = create_temp_config_dir();

        // Create initial config
        ensure_global_env().unwrap();
        let config_path = get_base_config_path().unwrap();
        let original_content = fs::read_to_string(&config_path).unwrap();

        // Run again
        ensure_global_env().unwrap();

        // Content should be unchanged
        let new_content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(original_content, new_content);
    }

    #[test]
    fn test_ensure_global_env_with_existing_config() {
        let _temp_dir = create_temp_config_dir();

        // Create config directory and file manually
        let config_path = get_base_config_path().unwrap();
        let parent = config_path.parent().unwrap();
        fs::create_dir_all(parent).unwrap();

        let custom_config = BaseConfig {
            default_registry: "custom".to_string(),
            registries: [(
                "custom".to_string(),
                "git@github.com:custom/registry.git".to_string(),
            )]
            .iter()
            .cloned()
            .collect(),
        };
        custom_config.save().unwrap();

        // Run ensure_global_env
        ensure_global_env().unwrap();

        // Verify config was NOT overwritten
        let loaded_config = BaseConfig::load().unwrap();
        assert_eq!(loaded_config.default_registry, "custom");
        assert_eq!(
            loaded_config.registries.get("custom").unwrap(),
            "git@github.com:custom/registry.git"
        );
    }
}
