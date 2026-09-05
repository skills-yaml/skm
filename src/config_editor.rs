use crate::config::SkillsConfig;
use crate::config_manager::{get_base_config_path, BaseConfig};
use serde_yaml::Value;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Configuration scope
#[derive(Debug, Clone, Copy)]
pub enum ConfigScope {
    Project,
    Global,
}

impl ConfigScope {
    pub fn path(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        match self {
            ConfigScope::Project => {
                let current_dir = std::env::current_dir()?;
                Ok(current_dir.join("skills.yaml"))
            }
            ConfigScope::Global => get_base_config_path()
                .ok_or_else(|| "Could not determine global config path".into()),
        }
    }
}

/// Get a configuration value
pub fn get_value(
    key: &str,
    global: bool,
    _project: bool,
    json_output: bool,
    show_default: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = if global {
        ConfigScope::Global
    } else {
        ConfigScope::Project
    };

    let path = scope.path()?;

    if !path.exists() {
        if show_default {
            let default = get_default_value(key);
            if json_output {
                println!("{}", serde_json::to_string(&default)?);
            } else {
                match default {
                    Value::String(ref s) => println!("{}", s),
                    _ => print!("{}", serde_yaml::to_string(&default)?),
                }
            }
            return Ok(());
        }
        return Err(format!("Configuration file not found: {}", path.display()).into());
    }

    let content = fs::read_to_string(&path)?;
    let value: Value = serde_yaml::from_str(&content)?;

    let result = get_nested_value(&value, key);

    match result {
        Some(v) => {
            if json_output {
                println!("{}", serde_json::to_string(&v)?);
            } else {
                match v {
                    Value::String(ref s) => println!("{}", s),
                    _ => print!("{}", serde_yaml::to_string(&v)?),
                }
            }
            Ok(())
        }
        None => {
            if show_default {
                let default = get_default_value(key);
                if json_output {
                    println!("{}", serde_json::to_string(&default)?);
                } else {
                    match default {
                        Value::String(ref s) => println!("{}", s),
                        _ => print!("{}", serde_yaml::to_string(&default)?),
                    }
                }
                Ok(())
            } else {
                Err(format!("Key '{}' not found in configuration", key).into())
            }
        }
    }
}

/// Set a configuration value
pub fn set_value(
    key: &str,
    value: &str,
    global: bool,
    _project: bool,
    parse_json: bool,
    dry_run: bool,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = if global {
        ConfigScope::Global
    } else {
        ConfigScope::Project
    };

    let path = scope.path()?;

    // Parse value
    let parsed_value: Value = if parse_json {
        serde_json::from_str(value)?
    } else {
        serde_yaml::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()))
    };

    // Load existing config or create new
    let mut config: Value = if path.exists() {
        let content = fs::read_to_string(&path)?;
        serde_yaml::from_str(&content)?
    } else {
        Value::Mapping(serde_yaml::Mapping::new())
    };

    if dry_run {
        println!(
            "Would set '{}' to:\n{}",
            key,
            serde_yaml::to_string(&parsed_value)?
        );
        println!("In: {}", path.display());
        return Ok(());
    }

    // Confirm sensitive keys
    if is_sensitive_key(key) && !yes {
        print!(
            "Are you sure you want to set sensitive key '{}'? [y/N] ",
            key
        );
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Set the value
    set_nested_value(&mut config, key, parsed_value.clone());

    // Validate structured config before writing
    validate_config_structure(&config, false, &scope)?;

    // Save config
    let content = serde_yaml::to_string(&config)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, content)?;

    println!(
        "Set '{}' = {}",
        key,
        serde_yaml::to_string(&parsed_value)?.trim()
    );

    Ok(())
}

/// Unset a configuration value
pub fn unset_value(
    key: &str,
    global: bool,
    _project: bool,
    dry_run: bool,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = if global {
        ConfigScope::Global
    } else {
        ConfigScope::Project
    };

    let path = scope.path()?;

    if !path.exists() {
        return Err(format!("Configuration file not found: {}", path.display()).into());
    }

    let content = fs::read_to_string(&path)?;
    let mut config: Value = serde_yaml::from_str(&content)?;

    // Check if key exists
    if get_nested_value(&config, key).is_none() {
        return Err(format!("Key '{}' not found in configuration", key).into());
    }

    if dry_run {
        println!("Would unset: {}", key);
        return Ok(());
    }

    // Confirm for sensitive keys
    if (is_sensitive_key(key) || is_required_key(key, &scope)) && !yes {
        print!("Are you sure you want to remove '{}'? [y/N] ", key);
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Unset cancelled.");
            return Ok(());
        }
    }

    // Remove the key
    unset_nested_value(&mut config, key);

    // Save config
    let content = serde_yaml::to_string(&config)?;
    fs::write(&path, content)?;

    println!("Unset: {}", key);

    Ok(())
}

/// Show full configuration
pub fn show_config(
    global: bool,
    project: bool,
    all: bool,
    json_output: bool,
    yaml_output: bool,
    show_paths: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scopes = if all {
        vec![ConfigScope::Global, ConfigScope::Project]
    } else {
        let mut scopes = Vec::new();
        if global {
            scopes.push(ConfigScope::Global);
        }
        if project || (!global && !all) {
            scopes.push(ConfigScope::Project);
        }
        scopes
    };

    if show_paths {
        for scope in &scopes {
            let path = scope.path()?;
            let name = match scope {
                ConfigScope::Global => "Global",
                ConfigScope::Project => "Project",
            };
            println!("{} config: {}", name, path.display());
        }
        return Ok(());
    }

    for scope in &scopes {
        let path = scope.path()?;
        let name = match scope {
            ConfigScope::Global => "Global Configuration",
            ConfigScope::Project => "Project Configuration",
        };

        if !path.exists() {
            println!("=== {} (not found) ===", name);
            continue;
        }

        let content = fs::read_to_string(&path)?;
        let config: Value = serde_yaml::from_str(&content)?;

        if json_output {
            if scopes.len() > 1 {
                println!(
                    "{{ \"scope\": \"{}\", \"config\": {} }}",
                    name.to_lowercase(),
                    serde_json::to_string(&config)?
                );
            } else {
                println!("{}", serde_json::to_string(&config)?);
            }
        } else if yaml_output {
            if scopes.len() > 1 {
                println!("# {}", name);
            }
            print!("{}", serde_yaml::to_string(&config)?);
        } else {
            if scopes.len() > 1 {
                println!("=== {} ===", name);
            }
            print_yaml_value(&config, 0);
            println!();
        }
    }

    Ok(())
}

/// Reset configuration to defaults
pub fn reset_config(
    global: bool,
    project: bool,
    yes: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scopes = {
        let mut scopes = Vec::new();
        if global {
            scopes.push(ConfigScope::Global);
        }
        if project {
            scopes.push(ConfigScope::Project);
        }
        if scopes.is_empty() {
            scopes.push(ConfigScope::Global);
            scopes.push(ConfigScope::Project);
        }
        scopes
    };

    for scope in &scopes {
        let path = scope.path()?;
        let name = match scope {
            ConfigScope::Global => "global",
            ConfigScope::Project => "project",
        };

        if dry_run {
            println!("Would reset {} configuration to defaults", name);
            continue;
        }

        if !yes {
            print!(
                "Are you sure you want to reset {} configuration to defaults? [y/N] ",
                name
            );
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("Reset cancelled for {} configuration.", name);
                continue;
            }
        }

        let default_config = match scope {
            ConfigScope::Global => {
                let config = BaseConfig::new();
                serde_yaml::to_value(config)?
            }
            ConfigScope::Project => {
                let mut config = SkillsConfig::default_init("unnamed");
                config.skills.clear();
                serde_yaml::to_value(config)?
            }
        };

        let content = serde_yaml::to_string(&default_config)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;

        println!("Reset {} configuration to defaults", name);
    }

    Ok(())
}

/// Validate configuration files
pub fn validate_config(
    global: bool,
    project: bool,
    all: bool,
    strict: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scopes = if all {
        vec![ConfigScope::Global, ConfigScope::Project]
    } else {
        let mut scopes = Vec::new();
        if global {
            scopes.push(ConfigScope::Global);
        }
        if project {
            scopes.push(ConfigScope::Project);
        }
        if scopes.is_empty() {
            scopes.push(ConfigScope::Project);
        }
        scopes
    };

    let mut valid = true;

    for scope in &scopes {
        let path = scope.path()?;
        let name = match scope {
            ConfigScope::Global => "Global",
            ConfigScope::Project => "Project",
        };

        if !path.exists() {
            println!("{} configuration: NOT FOUND ({})", name, path.display());
            valid = false;
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                println!("{} configuration: INVALID (cannot read: {})", name, e);
                valid = false;
                continue;
            }
        };

        let config: Value = match serde_yaml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                println!("{} configuration: INVALID (YAML error: {})", name, e);
                valid = false;
                continue;
            }
        };

        match validate_config_structure(&config, strict, scope) {
            Ok(_) => println!("{} configuration: VALID", name),
            Err(e) => {
                println!("{} configuration: INVALID ({})", name, e);
                valid = false;
            }
        }
    }

    if !valid {
        std::process::exit(1);
    }

    Ok(())
}

// Helpers

fn get_nested_value<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;

    for part in parts {
        match current {
            Value::Mapping(map) => {
                current = map.get(Value::String(part.to_string()))?;
            }
            Value::Sequence(vec) => {
                if let Ok(index) = part.parse::<usize>() {
                    current = vec.get(index)?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(current)
}

fn set_nested_value(value: &mut Value, key: &str, new_value: Value) {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            match current {
                Value::Mapping(map) => {
                    map.insert(Value::String(part.to_string()), new_value);
                    return;
                }
                Value::Sequence(vec) => {
                    if let Ok(index) = part.parse::<usize>() {
                        if index < vec.len() {
                            vec[index] = new_value;
                        } else {
                            vec.push(new_value);
                        }
                    }
                    return;
                }
                _ => {
                    *current = new_value;
                    return;
                }
            }
        } else {
            if current.is_mapping() {
                let map = current.as_mapping_mut().unwrap();
                let key_val = Value::String(part.to_string());
                if !map.contains_key(&key_val) {
                    map.insert(key_val.clone(), Value::Mapping(serde_yaml::Mapping::new()));
                }
                current = map.get_mut(&key_val).unwrap();
            } else if current.is_sequence() {
                // To avoid double mutable borrowing current, we check sequence mutability
                let index_opt = part.parse::<usize>().ok();
                if let Some(index) = index_opt {
                    let vec = current.as_sequence_mut().unwrap();
                    if index >= vec.len() {
                        vec.resize(index + 1, Value::Null);
                    }
                    current = &mut vec[index];
                } else {
                    let mut new_map = serde_yaml::Mapping::new();
                    let key_val = Value::String(part.to_string());
                    new_map.insert(key_val.clone(), Value::Mapping(serde_yaml::Mapping::new()));
                    *current = Value::Mapping(new_map);
                    current = current.as_mapping_mut().unwrap().get_mut(&key_val).unwrap();
                }
            } else {
                let mut new_map = serde_yaml::Mapping::new();
                let key_val = Value::String(part.to_string());
                new_map.insert(key_val.clone(), Value::Mapping(serde_yaml::Mapping::new()));
                *current = Value::Mapping(new_map);
                current = current.as_mapping_mut().unwrap().get_mut(&key_val).unwrap();
            }
        }
    }
}

fn unset_nested_value(value: &mut Value, key: &str) -> bool {
    let parts: Vec<&str> = key.split('.').collect();
    unset_nested_value_rec(value, &parts)
}

fn unset_nested_value_rec(value: &mut Value, parts: &[&str]) -> bool {
    if parts.is_empty() {
        return false;
    }

    let current_part = parts[0];
    if parts.len() == 1 {
        match value {
            Value::Mapping(map) => map
                .remove(Value::String(current_part.to_string()))
                .is_some(),
            Value::Sequence(vec) => {
                if let Ok(index) = current_part.parse::<usize>() {
                    if index < vec.len() {
                        vec.remove(index);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    } else {
        match value {
            Value::Mapping(map) => {
                if let Some(child) = map.get_mut(Value::String(current_part.to_string())) {
                    unset_nested_value_rec(child, &parts[1..])
                } else {
                    false
                }
            }
            Value::Sequence(vec) => {
                if let Ok(index) = current_part.parse::<usize>() {
                    if index < vec.len() {
                        unset_nested_value_rec(&mut vec[index], &parts[1..])
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

fn print_yaml_value(value: &Value, indent: usize) {
    let prefix = " ".repeat(indent);

    match value {
        Value::Null => println!("{}null", prefix),
        Value::Bool(b) => println!("{}{}", prefix, b),
        Value::Number(n) => println!("{}{}", prefix, n),
        Value::String(s) => println!("{}{}", prefix, s),
        Value::Sequence(vec) => {
            println!("{}[", prefix);
            for item in vec {
                print_yaml_value(item, indent + 2);
            }
            println!("{}]", prefix);
        }
        Value::Mapping(map) => {
            println!("{}{{", prefix);
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort_by(|a, b| {
                let a_str = a.as_str().unwrap_or("");
                let b_str = b.as_str().unwrap_or("");
                a_str.cmp(b_str)
            });
            for key in keys {
                if let Some(k) = key.as_str() {
                    print!("{}  {}: ", prefix, k);
                }
                print_yaml_value(map.get(key).unwrap(), indent + 4);
            }
            println!("{}{{", prefix);
        }
        _ => println!("{}<unknown>", prefix),
    }
}

fn get_default_value(key: &str) -> Value {
    match key {
        "default_registry" => Value::String("default".to_string()),
        "check_for_updates" => Value::Bool(true),
        "version" => Value::String("0.1.0".to_string()),
        "agents" => Value::Sequence(vec![
            Value::String("claude".to_string()),
            Value::String("cursor".to_string()),
            Value::String("codex".to_string()),
            Value::String("copilot".to_string()),
            Value::String("grok".to_string()),
            Value::String("hermes".to_string()),
        ]),
        _ => Value::Null,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    matches!(key, "default_registry" | "registries")
}

fn is_required_key(key: &str, scope: &ConfigScope) -> bool {
    match scope {
        ConfigScope::Global => matches!(key, "default_registry" | "registries"),
        ConfigScope::Project => matches!(key, "name" | "agents"),
    }
}

fn validate_config_structure(
    config: &Value,
    strict: bool,
    scope: &ConfigScope,
) -> Result<(), Box<dyn std::error::Error>> {
    match scope {
        ConfigScope::Global => {
            if let Value::Mapping(map) = config {
                if strict {
                    if !map.contains_key(Value::String("default_registry".to_string())) {
                        return Err("Missing required field: default_registry".into());
                    }
                    if !map.contains_key(Value::String("registries".to_string())) {
                        return Err("Missing required field: registries".into());
                    }
                }

                if let Some(Value::Mapping(registries)) =
                    map.get(Value::String("registries".to_string()))
                {
                    for (key, value) in registries {
                        if let Some(name) = key.as_str() {
                            if name.is_empty() {
                                return Err("Registry name cannot be empty".into());
                            }
                        }
                        if let Some(url) = value.as_str() {
                            if url.is_empty() {
                                return Err("Registry URL cannot be empty".into());
                            }
                        } else {
                            return Err("Registry URL must be a string".into());
                        }
                    }
                }
            } else {
                return Err("Global configuration must be a mapping".into());
            }
        }
        ConfigScope::Project => {
            if let Value::Mapping(map) = config {
                if strict {
                    if !map.contains_key(Value::String("name".to_string())) {
                        return Err("Missing required field: name".into());
                    }
                    if !map.contains_key(Value::String("agents".to_string())) {
                        return Err("Missing required field: agents".into());
                    }
                }

                if let Some(Value::Sequence(agents)) = map.get(Value::String("agents".to_string()))
                {
                    for agent in agents {
                        if let Some(name) = agent.as_str() {
                            if !crate::linker::is_supported_agent(name) {
                                return Err(format!("Unsupported agent: {}", name).into());
                            }
                        }
                    }
                }
            } else {
                return Err("Project configuration must be a mapping".into());
            }
        }
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
        let path =
            std::env::temp_dir().join(format!("skm-test-config-{}-{}", std::process::id(), unique));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    #[serial]
    fn test_config_get_and_set() {
        let temp = temp_project();
        let _guard = CurrentDirGuard::new(&temp);
        let home_temp = temp.join("mock_home");
        fs::create_dir_all(&home_temp).unwrap();
        let _home_guard = EnvVarGuard::new("HOME", home_temp.to_str().unwrap());
        let _xdg_config_guard = EnvVarGuard::new(
            "XDG_CONFIG_HOME",
            home_temp.join(".config").to_str().unwrap(),
        );

        // Test global set and get
        set_value(
            "default_registry",
            "company",
            true,
            false,
            false,
            false,
            true,
        )
        .unwrap();

        let path = ConfigScope::Global.path().unwrap();
        assert!(path.exists());

        // Retrieve value
        get_value("default_registry", true, false, false, false).unwrap();

        // Retrieve default fallback
        get_value("check_for_updates", true, false, false, true).unwrap();

        // Test project set and get
        // First create skills.yaml
        let proj_config = SkillsConfig::default_init("unnamed");
        proj_config.save_to_file(temp.join("skills.yaml")).unwrap();

        set_value("name", "my-new-project", false, true, false, false, true).unwrap();
        get_value("name", false, true, false, false).unwrap();

        // Test nested key setting
        set_value(
            "registries.company",
            "git@github.com:company/skills.git",
            true,
            false,
            false,
            false,
            true,
        )
        .unwrap();
        get_value("registries.company", true, false, false, false).unwrap();

        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    #[serial]
    fn test_config_unset() {
        let temp = temp_project();
        let _guard = CurrentDirGuard::new(&temp);
        let home_temp = temp.join("mock_home");
        fs::create_dir_all(&home_temp).unwrap();
        let _home_guard = EnvVarGuard::new("HOME", home_temp.to_str().unwrap());

        // Create skills.yaml in project dir
        let proj_config = SkillsConfig::default_init("test-unset");
        proj_config.save_to_file(temp.join("skills.yaml")).unwrap();

        // Check name exists
        get_value("name", false, true, false, false).unwrap();

        // Unset name
        unset_value("name", false, true, false, true).unwrap();

        // Verify name is gone (get should return Err)
        assert!(get_value("name", false, true, false, false).is_err());

        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    #[serial]
    fn test_config_reset_and_validate() {
        let temp = temp_project();
        let _guard = CurrentDirGuard::new(&temp);
        let home_temp = temp.join("mock_home");
        fs::create_dir_all(&home_temp).unwrap();
        let _home_guard = EnvVarGuard::new("HOME", home_temp.to_str().unwrap());
        let _xdg_config_guard = EnvVarGuard::new(
            "XDG_CONFIG_HOME",
            home_temp.join(".config").to_str().unwrap(),
        );

        // Create base config
        let base_config = BaseConfig::new();
        base_config.save().unwrap();

        // Create project config
        let proj_config = SkillsConfig::default_init("validate-test");
        proj_config.save_to_file(temp.join("skills.yaml")).unwrap();

        // Validate
        validate_config(true, true, true, true).unwrap();

        // Reset configuration
        reset_config(true, true, true, false).unwrap();

        // Validate again after reset
        validate_config(true, true, true, true).unwrap();

        fs::remove_dir_all(temp).unwrap();
    }
}
