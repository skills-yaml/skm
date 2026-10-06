use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillSpec {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ToolkitSelection {
    pub manifest: String,
    pub version: String,
}

impl SkillSpec {
    /// Parse skill spec with version (e.g., "my-skill@v1.2.0")
    pub fn parse_with_version(
        input: &str,
    ) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
        if let Some(at_pos) = input.rfind('@') {
            let name = &input[..at_pos];
            let version = Some(input[at_pos + 1..].to_string());
            Ok((name.to_string(), version))
        } else {
            Ok((input.to_string(), None))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillsConfig {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registries: Option<HashMap<String, String>>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub skills: Vec<SkillSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolkit: Option<ToolkitSelection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bundles: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<String>,
    /// Publisher-owned configuration is retained without interpretation.
    #[serde(flatten)]
    pub metadata: BTreeMap<String, serde_yaml::Value>,
}

impl SkillsConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config: SkillsConfig = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(path)?;
        serde_yaml::to_writer(file, self)?;
        Ok(())
    }

    pub fn default_init(project_name: &str) -> Self {
        let mut registries = HashMap::new();
        registries.insert(
            "default".to_string(),
            "git@github.com:skills-yaml/registry.git".to_string(),
        );

        SkillsConfig {
            name: project_name.to_string(),
            version: Some("0.1.0".to_string()),
            registries: Some(registries),
            agents: vec![
                "claude".to_string(),
                "codex".to_string(),
                "cursor".to_string(),
                "grok".to_string(),
                "hermes".to_string(),
            ],
            skills: vec![SkillSpec {
                name: "software-development/spec".to_string(),
                version: Some("latest".to_string()),
                source: Some("default".to_string()),
                path: None,
            }],
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            metadata: Default::default(),
        }
    }

    /// Remove a skill from the configuration
    pub fn remove_skill(&mut self, skill_name: &str) -> Option<SkillSpec> {
        let index = self.skills.iter().position(|s| s.name == skill_name)?;
        Some(self.skills.remove(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publisher_configuration_round_trips_without_domain_validation() {
        let raw = "name: example\nagents: [codex]\nskills: []\nworkspace: [opaque, metadata]\ntrusted_sources: {publisher: ignored}\ncompany_policy: {edition: future, active: true}\n";
        let config: SkillsConfig = serde_yaml::from_str(raw).unwrap();
        let project = tempfile::tempdir().unwrap();
        let path = project.path().join("skills.yaml");
        config.save_to_file(&path).unwrap();
        let reloaded = SkillsConfig::load_from_file(path).unwrap();
        assert_eq!(reloaded.metadata, config.metadata);
        assert_eq!(reloaded.name, "example");
    }
}
