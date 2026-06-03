use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillSpec {
    pub name: String,
    pub version: Option<String>,
    pub source: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillsConfig {
    pub name: String,
    pub version: Option<String>,
    pub registries: Option<HashMap<String, String>>,
    pub agents: Vec<String>,
    pub skills: Vec<SkillSpec>,
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
            "git@github.com:skills-yaml/skills-registry.git".to_string(),
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
                name: "software-development/symphony-spec-writing".to_string(),
                version: Some("latest".to_string()),
                source: Some("default".to_string()),
                path: None,
            }],
        }
    }
}
