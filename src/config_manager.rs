use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Default registry URL
pub const DEFAULT_REGISTRY_URL: &str = "git@github.com:skills-yaml/registry.git";

/// Canonical settings and manifest directory for the global-home migration.
pub fn get_user_config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join(".skm"))
}

pub fn get_user_base_config_path() -> Option<PathBuf> {
    get_user_config_dir().map(|d| d.join("config.yaml"))
}

/// Get the global base config path
pub fn get_base_config_path() -> Option<PathBuf> {
    get_user_base_config_path()
}

/// Get the user-owned global skills manifest path.
#[allow(dead_code)]
pub fn get_global_manifest_path() -> Option<PathBuf> {
    get_user_config_dir().map(|d| d.join("skills.yaml"))
}

/// Get the cache directory path
pub fn get_cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("skm"))
}

/// Canonical cache location for user-scoped SKM state.
#[allow(dead_code)]
pub fn get_user_cache_dir() -> Option<PathBuf> {
    get_user_config_dir().map(|d| d.join("cache"))
}

/// Get the registries cache directory
#[allow(dead_code)]
fn get_registries_cache_dir() -> Option<PathBuf> {
    get_cache_dir().map(|d| d.join("registries"))
}

const EMPTY_GLOBAL_MANIFEST: &str = "name: Global skills\nagents: []\nskills: []\n";

type ConfigResult<T> = Result<T, Box<dyn std::error::Error>>;

fn directory(path: &Path, create: bool) -> ConfigResult<()> {
    if create {
        match fs::create_dir(path) {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
            Err(error) => {
                return Err(format!("Cannot create directory {}: {}", path.display(), error).into())
            }
        }
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Cannot access directory {}: {}", path.display(), error))?;
    if !metadata.file_type().is_dir() {
        return Err(format!("Expected a real directory at {}", path.display()).into());
    }
    Ok(())
}

fn existing_directory(path: &Path) -> ConfigResult<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            directory(path, false)?;
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("Cannot inspect directory {}: {}", path.display(), error).into()),
    }
}

fn read_regular_file(path: &Path) -> ConfigResult<Option<String>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot inspect {}: {}", path.display(), error).into()),
    };
    if !metadata.file_type().is_file() {
        return Err(format!("Expected a regular file at {}", path.display()).into());
    }
    fs::read_to_string(path)
        .map(Some)
        .map_err(|error| format!("Cannot read {}: {}", path.display(), error).into())
}

fn validate_base(content: &str, path: &Path) -> ConfigResult<serde_yaml::Value> {
    let value: serde_yaml::Value = serde_yaml::from_str(content)
        .map_err(|error| format!("Invalid settings in {}: {}", path.display(), error))?;
    let map = value
        .as_mapping()
        .ok_or_else(|| format!("Settings in {} must be a mapping", path.display()))?;
    for (key, valid) in [
        (
            "default_registry",
            map.get("default_registry").is_none_or(|v| v.is_string()),
        ),
        (
            "check_for_updates",
            map.get("check_for_updates").is_none_or(|v| v.is_bool()),
        ),
        (
            "registries",
            map.get("registries").is_none_or(|v| {
                v.as_mapping()
                    .is_some_and(|m| m.iter().all(|(k, v)| k.is_string() && v.is_string()))
            }),
        ),
    ] {
        if !valid {
            return Err(format!("Invalid {} in {}", key, path.display()).into());
        }
    }
    Ok(value)
}

fn validate_manifest(content: &str, path: &Path) -> ConfigResult<()> {
    serde_yaml::from_str::<crate::config::SkillsConfig>(content)
        .map_err(|error| format!("Invalid skills manifest in {}: {}", path.display(), error))?;
    Ok(())
}

// Publish a complete file without ever replacing a concurrent creator's file.
fn publish_missing(path: &Path, content: &str) -> ConfigResult<()> {
    let parent = path.parent().ok_or("Missing file parent")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(content.as_bytes())?;
    temp.as_file().sync_all()?;
    match temp.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(format!("Cannot create {}: {}", path.display(), error.error).into()),
    }
}

/// Base configuration for SKM (stored in user's config directory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseConfig {
    pub default_registry: String,
    pub registries: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub check_for_updates: bool,
}

fn default_true() -> bool {
    true
}

impl BaseConfig {
    pub fn new() -> Self {
        let mut registries = HashMap::new();
        registries.insert("default".to_string(), DEFAULT_REGISTRY_URL.to_string());

        Self {
            default_registry: "default".to_string(),
            registries,
            check_for_updates: true,
        }
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = get_base_config_path().ok_or("Could not determine config directory")?;

        let Some(content) = read_regular_file(&path)? else {
            return Ok(Self::new());
        };
        let config = validate_base(&content, &path)?;

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

        // Load check_for_updates, default to true if not present
        let check_for_updates = config
            .get("check_for_updates")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        Ok(Self {
            default_registry,
            registries,
            check_for_updates,
        })
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = get_base_config_path().ok_or("Could not determine config directory")?;

        // Create config directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Keep unknown fields belonging to other publishers when settings change.
        let mut value = match read_regular_file(&path)? {
            Some(content) => validate_base(&content, &path)?,
            None => serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        };
        let fields = serde_yaml::to_value(self)?;
        let mapping = value.as_mapping_mut().ok_or("Settings must be a mapping")?;
        for (key, value) in fields.as_mapping().ok_or("Settings must be a mapping")? {
            mapping.insert(key.clone(), value.clone());
        }
        let file = fs::File::create(&path)?;
        serde_yaml::to_writer(file, &value)?;

        Ok(())
    }
}

/// Ensure base config exists, create if it doesn't.
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
            eprintln!("Updating registry '{}'...", reg_name);
            // Pull latest changes
            let output = std::process::Command::new("git")
                .args(["-C", cache_path.to_str().unwrap(), "pull"])
                .output()?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to update registry '{}': {}", reg_name, err).into());
            }
            eprintln!("Registry '{}' updated successfully.", reg_name);
        } else {
            eprintln!("Cloning registry '{}' from '{}'...", reg_name, url);
            let output = std::process::Command::new("git")
                .args(["clone", url, cache_path.to_str().unwrap()])
                .output()?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to clone registry '{}': {}", reg_name, err).into());
            }
            eprintln!("Registry '{}' cloned successfully.", reg_name);
        }
    }

    Ok(())
}

/// Ensure base configuration exists, create with defaults if missing
/// This is separate from first_time_setup() which also updates cache
pub fn ensure_global_env() -> Result<(), Box<dyn std::error::Error>> {
    bootstrap_user_home()
}

/// Bootstrap canonical user-home state without replacing existing files.
pub fn bootstrap_user_home() -> ConfigResult<()> {
    let home = dirs::home_dir().ok_or("Could not determine user home directory")?;
    directory(&home, false)?;
    let config_dir = get_user_config_dir().ok_or("Could not determine user home directory")?;
    directory(&config_dir, true)?;
    let config_path = config_dir.join("config.yaml");
    let manifest_path = config_dir.join("skills.yaml");
    let cache_path = config_dir.join("cache");

    // Validate existing state before creating anything else or importing legacy data.
    let current = read_regular_file(&config_path)?;
    if let Some(content) = &current {
        validate_base(content, &config_path)?;
    }
    let manifest = read_regular_file(&manifest_path)?;
    if let Some(content) = &manifest {
        validate_manifest(content, &manifest_path)?;
    }
    existing_directory(&cache_path)?;

    if current.is_none() {
        let legacy = dirs::config_dir().map(|d| d.join("skm").join("config.yaml"));
        let imported = match legacy {
            Some(path) if path != config_path => {
                let parent = path.parent().ok_or("Missing legacy settings directory")?;
                if existing_directory(parent)? {
                    match read_regular_file(&path)? {
                        Some(content) => {
                            validate_base(&content, &path)?;
                            Some(content)
                        }
                        None => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        };
        let content = match imported {
            Some(content) => content,
            None => serde_yaml::to_string(&BaseConfig::new())?,
        };
        publish_missing(&config_path, &content)?;
    }
    if manifest.is_none() {
        publish_missing(&manifest_path, EMPTY_GLOBAL_MANIFEST)?;
    }
    directory(&cache_path, true)?;

    // A concurrent creator may have won; validate the actual published files.
    let content =
        read_regular_file(&config_path)?.ok_or("Settings disappeared during bootstrap")?;
    validate_base(&content, &config_path)?;
    let content =
        read_regular_file(&manifest_path)?.ok_or("Skills manifest disappeared during bootstrap")?;
    validate_manifest(&content, &manifest_path)?;
    Ok(())
}

/// First-time setup: initialize base config and cache default registry
pub fn first_time_setup() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Running first-time setup for SKM...");

    // Initialize base config
    ensure_global_env()?;

    // Update cache for default registry
    update_cache(Some("default"))?;

    eprintln!("\nFirst-time setup completed!");
    eprintln!("Run 'skm init' to create a project configuration.");

    Ok(())
}

/// Check if this is the first time running SKM (deprecated: use ensure_global_env() instead)
#[deprecated(
    note = "Use ensure_global_env() instead. This function checks both config and cache, but global env should always be ensured."
)]
#[allow(dead_code)]
pub fn is_first_time() -> bool {
    let base_config_path = get_base_config_path();
    let registries_cache_dir = get_registries_cache_dir();

    base_config_path.is_none_or(|p| !p.exists()) || registries_cache_dir.is_none_or(|p| !p.exists())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;

    /// Helper struct to temporarily set HOME and XDG_CONFIG_HOME and restore them on drop
    struct TempHome {
        _temp_dir: tempfile::TempDir,
        original_home: Option<String>,
        original_xdg_config_home: Option<String>,
    }

    impl TempHome {
        fn new() -> Self {
            let original_home = std::env::var("HOME").ok();
            let original_xdg_config_home = std::env::var("XDG_CONFIG_HOME").ok();
            let temp_dir = tempfile::tempdir().unwrap();
            let temp_config_dir = temp_dir.path().join(".config");
            fs::create_dir_all(&temp_config_dir).ok();
            std::env::set_var("HOME", temp_dir.path());
            std::env::set_var("XDG_CONFIG_HOME", temp_config_dir);
            Self {
                _temp_dir: temp_dir,
                original_home,
                original_xdg_config_home,
            }
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            if let Some(ref home) = self.original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
            if let Some(ref xdg) = self.original_xdg_config_home {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
        }
    }

    fn create_temp_config_dir() -> TempHome {
        TempHome::new()
    }

    #[test]
    #[serial]
    fn test_ensure_global_env_creates_config() {
        let _temp_home = create_temp_config_dir();

        // Clear any environment variables that might affect the test
        std::env::remove_var("SKM_CHECK_UPDATE");

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
        assert!(config.check_for_updates); // Should default to true
    }

    #[test]
    #[serial]
    fn test_ensure_global_env_idempotent() {
        let _temp_home = create_temp_config_dir();

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
    #[serial]
    fn test_ensure_global_env_with_existing_config() {
        let _temp_home = create_temp_config_dir();

        // Clear any environment variables that might affect the test
        std::env::remove_var("SKM_CHECK_UPDATE");

        // Remove any existing config first
        let config_path = get_base_config_path().unwrap();
        let parent = config_path.parent().unwrap();
        if parent.exists() {
            fs::remove_dir_all(parent).ok();
        }

        // Create config directory and file manually
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
            check_for_updates: true,
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
    #[test]
    #[serial]
    fn dormant_cache_resolver_selects_ordered_roots_and_rejects_conflicts() {
        use std::os::unix::fs::symlink;
        let home = create_temp_config_dir();
        let root = home._temp_dir.path();
        let old_cache = std::env::var_os("XDG_CACHE_HOME");
        std::env::set_var("XDG_CACHE_HOME", root.join("xdg-cache"));
        let source = root.join("source");
        let wrong = root.join("wrong");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&wrong).unwrap();
        let linker = root.join(".cache/skm/registries/local");
        let os = root.join("xdg-cache/skm/registries/local");
        let canonical = get_user_cache_dir().unwrap().join("registries/local");
        fs::create_dir_all(linker.parent().unwrap()).unwrap();
        fs::create_dir_all(os.parent().unwrap()).unwrap();
        symlink(&source, &os).unwrap();
        assert_eq!(
            crate::linker::validated_registry_path("local", "source", root).unwrap(),
            os
        );
        symlink(&wrong, &linker).unwrap();
        assert!(crate::linker::validated_registry_path("local", "source", root).is_err());
        fs::remove_file(&linker).unwrap();
        symlink(&source, &linker).unwrap();
        assert_eq!(
            crate::linker::validated_registry_path("local", "source", root).unwrap(),
            linker
        );
        fs::create_dir_all(canonical.parent().unwrap()).unwrap();
        symlink(&wrong, &canonical).unwrap();
        assert!(crate::linker::validated_registry_path("local", "source", root).is_err());
        assert!(linker.exists());
        assert!(os.exists());
        match old_cache {
            Some(value) => std::env::set_var("XDG_CACHE_HOME", value),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
    }

    #[test]
    #[serial]
    fn bootstrap_creates_empty_manifest_and_cache_and_preserves_bytes() {
        let _home = create_temp_config_dir();
        bootstrap_user_home().unwrap();
        let manifest = get_global_manifest_path().unwrap();
        assert_eq!(
            fs::read_to_string(&manifest).unwrap(),
            EMPTY_GLOBAL_MANIFEST
        );
        let parsed = crate::config::SkillsConfig::load_from_file(&manifest).unwrap();
        assert!(parsed.agents.is_empty() && parsed.skills.is_empty());
        assert!(get_user_cache_dir().unwrap().is_dir());
        let custom = "name: Personal\nagents: [codex]\nskills: []\npublisher: retained\n";
        fs::write(&manifest, custom).unwrap();
        let settings = fs::read(get_user_base_config_path().unwrap()).unwrap();
        bootstrap_user_home().unwrap();
        assert_eq!(fs::read_to_string(manifest).unwrap(), custom);
        assert_eq!(
            fs::read(get_user_base_config_path().unwrap()).unwrap(),
            settings
        );
    }

    #[test]
    #[serial]
    fn bootstrap_imports_legacy_bytes_and_preserves_unknown_fields() {
        let _home = create_temp_config_dir();
        let legacy = dirs::config_dir().unwrap().join("skm").join("config.yaml");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        let content = "default_registry: custom\nregistries:\n  custom: custom-url\nopaque:\n  nested: retained\n";
        fs::write(&legacy, content).unwrap();
        bootstrap_user_home().unwrap();
        let path = get_user_base_config_path().unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
        assert_eq!(fs::read_to_string(legacy).unwrap(), content);
    }

    #[test]
    #[serial]
    fn bootstrap_new_settings_win_over_invalid_legacy() {
        let _home = create_temp_config_dir();
        bootstrap_user_home().unwrap();
        let legacy = dirs::config_dir().unwrap().join("skm").join("config.yaml");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(legacy, "invalid: [").unwrap();
        bootstrap_user_home().unwrap();
    }

    #[test]
    #[serial]
    fn bootstrap_rejects_invalid_yaml_and_unexpected_types_without_overwrite() {
        let _home = create_temp_config_dir();
        let legacy = dirs::config_dir().unwrap().join("skm").join("config.yaml");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, "invalid: [").unwrap();
        assert!(bootstrap_user_home().is_err());
        assert!(!get_user_base_config_path().unwrap().exists());
        fs::write(&legacy, "registries: []\n").unwrap();
        assert!(bootstrap_user_home().is_err());
        fs::remove_file(legacy).unwrap();
        let manifest = get_global_manifest_path().unwrap();
        fs::write(&manifest, "name: [\n").unwrap();
        assert!(bootstrap_user_home().is_err());
        assert_eq!(fs::read_to_string(&manifest).unwrap(), "name: [\n");
        fs::remove_file(&manifest).unwrap();
        fs::create_dir(&manifest).unwrap();
        assert!(bootstrap_user_home().is_err());
    }

    #[test]
    #[serial]
    fn bootstrap_concurrent_creators_publish_complete_documents() {
        let _home = create_temp_config_dir();
        let threads: Vec<_> = (0..12)
            .map(|_| std::thread::spawn(|| bootstrap_user_home().map_err(|e| e.to_string())))
            .collect();
        for thread in threads {
            thread.join().unwrap().unwrap();
        }
        bootstrap_user_home().unwrap();
        assert_eq!(
            fs::read_to_string(get_global_manifest_path().unwrap()).unwrap(),
            EMPTY_GLOBAL_MANIFEST
        );
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn bootstrap_rejects_symlink_paths() {
        use std::os::unix::fs::symlink;
        let home = create_temp_config_dir();
        let target = home._temp_dir.path().join("target");
        fs::create_dir(&target).unwrap();
        symlink(&target, get_user_config_dir().unwrap()).unwrap();
        assert!(bootstrap_user_home().is_err());
        fs::remove_file(get_user_config_dir().unwrap()).unwrap();
        fs::create_dir(get_user_config_dir().unwrap()).unwrap();
        let external = home._temp_dir.path().join("external");
        fs::write(&external, EMPTY_GLOBAL_MANIFEST).unwrap();
        symlink(&external, get_global_manifest_path().unwrap()).unwrap();
        assert!(bootstrap_user_home().is_err());
        assert_eq!(fs::read_to_string(external).unwrap(), EMPTY_GLOBAL_MANIFEST);
    }
}
