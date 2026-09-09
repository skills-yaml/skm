use crate::config::SkillsConfig;
use serde_yaml::{Mapping, Value};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Keep the YAML tree so fields from newer versions survive editing.
pub struct Document {
    pub value: Value,
    pub path: PathBuf,
    original: Option<String>,
    initial: Value,
}

impl Document {
    pub fn load(path: &Path, name: Option<&str>, non_interactive: bool) -> Result<Self> {
        let original = read_manifest(path)?;
        if non_interactive && original.is_some() {
            return Err("skills.yaml already exists; run 'skm init' to review and edit it".into());
        }
        let initial = if let Some(content) = &original {
            let value: Value = serde_yaml::from_str(content)
                .map_err(|e| format!("Cannot parse skills.yaml; the file was not changed: {e}"))?;
            // Check types without discarding extension fields or inserting defaults.
            serde_yaml::from_value::<SkillsConfig>(value.clone())
                .map_err(|e| format!("Cannot load skills.yaml; the file was not changed: {e}"))?;
            value
        } else {
            let project_name = path
                .parent()
                .and_then(Path::file_name)
                .and_then(|s| s.to_str())
                .unwrap_or("my-project");
            let mut config = SkillsConfig::default_init(project_name);
            if !non_interactive {
                config.skills.clear();
            }
            serde_yaml::to_value(config)?
        };
        let mut document = Self {
            value: initial.clone(),
            initial,
            original,
            path: path.to_path_buf(),
        };
        if let Some(name) = name {
            document.value["name"] = Value::String(name.to_owned());
        }
        Ok(document)
    }

    pub fn exists(&self) -> bool {
        self.original.is_some()
    }

    pub fn changed(&self) -> bool {
        !self.exists() || self.value != self.initial
    }

    pub fn preview(&self) -> Result<String> {
        if self.value == self.initial {
            if let Some(original) = &self.original {
                return Ok(original.clone());
            }
        }
        Ok(serde_yaml::to_string(&self.value)?)
    }

    pub fn validate(&self, global: bool) -> Result<()> {
        let config: SkillsConfig = serde_yaml::from_value(self.value.clone())?;
        if config.name.trim().is_empty() {
            return Err("Project name cannot be empty (Project step)".into());
        }
        crate::validate_config(&config)?;
        if let Some(registries) = &config.registries {
            for (name, url) in registries {
                if !crate::linker::is_safe_registry_name(name) {
                    return Err(format!("Invalid registry name '{name}' (Registries step)").into());
                }
                if url.trim().is_empty() {
                    return Err(format!("Registry '{name}' needs a URL or path").into());
                }
            }
        }
        let mut names = std::collections::HashSet::new();
        for skill in &config.skills {
            if !names.insert(&skill.name) {
                return Err(format!("Duplicate skill '{}' (Skills step)", skill.name).into());
            }
            if let Some(source) = &skill.source {
                if !crate::linker::is_safe_registry_name(source) {
                    return Err(
                        format!("Invalid registry source '{source}' for '{}'", skill.name).into(),
                    );
                }
            }
            if skill.path.as_ref().is_some_and(|p| p.trim().is_empty()) {
                return Err(format!("Local path for '{}' cannot be empty", skill.name).into());
            }
        }
        if let Some(toolkit) = &config.toolkit {
            if global {
                return Err(
                    "Toolkit initialization is project-scoped; --global is not supported".into(),
                );
            }
            validate_relative_path(&toolkit.manifest)?;
            if toolkit.version.trim().is_empty() {
                return Err("Toolkit version cannot be empty".into());
            }
        } else if !config.bundles.is_empty() || !config.profiles.is_empty() {
            return Err("Bundles and profiles require a toolkit manifest (Workspace step)".into());
        }
        if let Some(workspace) = &config.workspace {
            if workspace.standard.trim().is_empty() {
                return Err("Workspace standard cannot be empty".into());
            }
        }
        Ok(())
    }

    /// Recheck the original before replacing the manifest in the same directory.
    pub fn save(&self, global: bool) -> Result<bool> {
        self.validate(global)?;
        self.check_unchanged()?;
        if !self.changed() {
            return Ok(false);
        }
        let parent = self.path.parent().unwrap_or(Path::new("."));
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        if self.exists() {
            temporary
                .as_file()
                .set_permissions(fs::metadata(&self.path)?.permissions())?;
        }
        temporary.write_all(self.preview()?.as_bytes())?;
        temporary.as_file().sync_all()?;
        self.check_unchanged()?;
        if self.exists() {
            temporary.persist(&self.path)?;
        } else {
            temporary.persist_noclobber(&self.path)?;
        }
        Ok(true)
    }

    fn check_unchanged(&self) -> Result<()> {
        if read_manifest(&self.path)? != self.original {
            return Err(
                "skills.yaml changed outside the wizard; cancel and reopen it to reload".into(),
            );
        }
        Ok(())
    }
}

fn read_manifest(path: &Path) -> Result<Option<String>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
            Err("skills.yaml must be a regular file, not a directory or symlink".into())
        }
        Ok(_) => Ok(Some(fs::read_to_string(path)?)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn validate_relative_path(value: &str) -> Result<()> {
    let path = Path::new(value);
    if value.trim().is_empty()
        || path.is_absolute()
        || path.components().any(|p| {
            !matches!(
                p,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return Err("Toolkit manifest must be a repository-relative path without '..'".into());
    }
    Ok(())
}

#[derive(Clone)]
pub enum Target {
    Field(&'static str),
    Nested(&'static str, &'static str),
    Registry(usize, bool), // index, editing the name rather than the URL
    Skill(usize, &'static str),
    Agent(String),
}

pub struct Field {
    pub label: String,
    pub hint: &'static str,
    pub target: Target,
}

impl Field {
    fn new(label: impl Into<String>, hint: &'static str, target: Target) -> Self {
        Self {
            label: label.into(),
            hint,
            target,
        }
    }
}

pub fn fields(value: &Value, step: usize) -> Vec<Field> {
    match step {
        0 => vec![
            Field::new(
                "Project name",
                "Required. Give this configuration a name.",
                Target::Field("name"),
            ),
            Field::new(
                "Project version",
                "Optional project version, for example 0.1.0.",
                Target::Field("version"),
            ),
        ],
        1 => {
            let mut names: Vec<String> =
                super::KNOWN_AGENTS.iter().map(|s| s.to_string()).collect();
            if let Some(agents) = value["agents"].as_sequence() {
                for agent in agents.iter().filter_map(Value::as_str) {
                    if !names.iter().any(|n| n == agent) {
                        names.push(agent.to_owned());
                    }
                }
            }
            names
                .into_iter()
                .map(|name| {
                    Field::new(
                        name.clone(),
                        "Space toggles this agent. All supported agents are available.",
                        Target::Agent(name),
                    )
                })
                .collect()
        }
        2 => value["registries"]
            .as_mapping()
            .map(|registries| {
                registries
                    .iter()
                    .enumerate()
                    .flat_map(|(i, _)| {
                        [
                            Field::new(
                                format!("Registry {} name", i + 1),
                                "Use letters, numbers, hyphens, or underscores.",
                                Target::Registry(i, true),
                            ),
                            Field::new(
                                "  URL / path",
                                "Git registry URL or local registry path.",
                                Target::Registry(i, false),
                            ),
                        ]
                    })
                    .collect()
            })
            .unwrap_or_default(),
        3 => value["skills"]
            .as_sequence()
            .map(|skills| {
                skills
                    .iter()
                    .enumerate()
                    .flat_map(|(i, _)| {
                        [
                            Field::new(
                                format!("Skill {} name", i + 1),
                                "Skill name, for example software-development/spec.",
                                Target::Skill(i, "name"),
                            ),
                            Field::new(
                                "  Version",
                                "Optional version or latest.",
                                Target::Skill(i, "version"),
                            ),
                            Field::new(
                                "  Registry source",
                                "Registry name; blank uses default. Local path takes precedence.",
                                Target::Skill(i, "source"),
                            ),
                            Field::new(
                                "  Local path",
                                "Optional local skill directory; leave blank for registry skills.",
                                Target::Skill(i, "path"),
                            ),
                        ]
                    })
                    .collect()
            })
            .unwrap_or_default(),
        4 => vec![
            Field::new(
                "Toolkit manifest",
                "Optional repository-relative manifest. Clear to remove the toolkit selection.",
                Target::Nested("toolkit", "manifest"),
            ),
            Field::new(
                "Toolkit version",
                "Pin the selected toolkit version.",
                Target::Nested("toolkit", "version"),
            ),
            Field::new(
                "Bundles",
                "Comma-separated bundle IDs; requires a toolkit manifest.",
                Target::Field("bundles"),
            ),
            Field::new(
                "Profiles",
                "Comma-separated profile IDs; requires a toolkit manifest.",
                Target::Field("profiles"),
            ),
            Field::new(
                "Workspace standard",
                "Optional, e.g. workspace-docs@5.0.0. Clear to remove workspace selection.",
                Target::Nested("workspace", "standard"),
            ),
            Field::new(
                "Workspace source",
                "Local package path or Git URL.",
                Target::Nested("workspace", "source"),
            ),
            Field::new(
                "Workspace revision",
                "Full Git commit for a remote package.",
                Target::Nested("workspace", "revision"),
            ),
            Field::new(
                "Workspace integrity",
                "Expected sha256: digest for a remote package.",
                Target::Nested("workspace", "integrity"),
            ),
            Field::new(
                "Trusted sources",
                "Comma-separated authorized package paths or Git URLs.",
                Target::Field("trusted_sources"),
            ),
        ],
        _ => Vec::new(),
    }
}

impl Target {
    pub fn read(&self, value: &Value) -> String {
        let field = match self {
            Self::Field(key) => &value[*key],
            Self::Nested(parent, key) => &value[*parent][*key],
            Self::Skill(i, key) => &value["skills"][*i][*key],
            Self::Registry(i, name) => {
                return value["registries"]
                    .as_mapping()
                    .and_then(|m| m.iter().nth(*i))
                    .map(|(k, v)| if *name { k } else { v })
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
            }
            Self::Agent(name) => {
                return if value["agents"]
                    .as_sequence()
                    .is_some_and(|a| a.contains(&Value::String(name.clone())))
                {
                    "[x]"
                } else {
                    "[ ]"
                }
                .into();
            }
        };
        if let Some(values) = field.as_sequence() {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            field.as_str().unwrap_or("").to_owned()
        }
    }

    pub fn write(&self, value: &mut Value, text: &str) -> Result<()> {
        let string = Value::String(text.to_owned());
        match self {
            Self::Field(key) if matches!(*key, "bundles" | "profiles" | "trusted_sources") => {
                let items: Vec<Value> = text
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| Value::String(s.to_owned()))
                    .collect();
                if items.is_empty() {
                    mapping(value).remove(*key);
                } else {
                    value[*key] = Value::Sequence(items);
                }
            }
            Self::Field(key) => set_optional(value, key, string, *key == "name"),
            Self::Nested(parent, key) => {
                if text.is_empty() && matches!(*key, "manifest" | "standard") {
                    mapping(value).remove(*parent);
                } else if !text.is_empty() || value[*parent].is_mapping() {
                    if value[*parent].is_null() {
                        value[*parent] = Value::Mapping(Mapping::new());
                    }
                    set_optional(&mut value[*parent], key, string, false);
                    if *parent == "toolkit" && value[*parent]["version"].is_null() {
                        value[*parent]["version"] = Value::String("0.1.0".into());
                    }
                }
            }
            Self::Skill(i, key) => {
                set_optional(&mut value["skills"][*i], key, string, *key == "name")
            }
            Self::Registry(index, rename) => {
                let registries = mapping(&mut value["registries"]);
                let mut entries: Vec<_> = registries.clone().into_iter().collect();
                if *rename
                    && entries
                        .iter()
                        .enumerate()
                        .any(|(i, (k, _))| i != *index && *k == string)
                {
                    return Err("A registry with that name already exists".into());
                }
                if let Some((key, url)) = entries.get_mut(*index) {
                    if *rename {
                        *key = string;
                    } else {
                        *url = string;
                    }
                }
                *registries = entries.into_iter().collect();
            }
            Self::Agent(name) => {
                if !value["agents"].is_sequence() {
                    value["agents"] = Value::Sequence(Vec::new());
                }
                let agents = value["agents"].as_sequence_mut().unwrap();
                let agent = Value::String(name.clone());
                if agents.contains(&agent) {
                    agents.retain(|a| a != &agent);
                } else {
                    agents.push(agent);
                }
            }
        }
        Ok(())
    }
}

fn mapping(value: &mut Value) -> &mut Mapping {
    if !value.is_mapping() {
        *value = Value::Mapping(Mapping::new());
    }
    value.as_mapping_mut().unwrap()
}

fn set_optional(value: &mut Value, key: &str, text: Value, required: bool) {
    if !required && text.as_str() == Some("") {
        mapping(value).remove(key);
    } else {
        value[key] = text;
    }
}

pub fn add_entry(value: &mut Value, step: usize) -> usize {
    if step == 2 {
        let registries = mapping(&mut value["registries"]);
        let mut number = registries.len() + 1;
        while registries.contains_key(format!("registry-{number}")) {
            number += 1;
        }
        let selected = registries.len() * 2;
        registries.insert(
            Value::String(format!("registry-{number}")),
            Value::String(String::new()),
        );
        selected
    } else {
        if !value["skills"].is_sequence() {
            value["skills"] = Value::Sequence(Vec::new());
        }
        let skills = value["skills"].as_sequence_mut().unwrap();
        let selected = skills.len() * 4;
        let mut skill = Mapping::new();
        skill.insert(Value::String("name".into()), Value::String(String::new()));
        skill.insert(
            Value::String("version".into()),
            Value::String("latest".into()),
        );
        skills.push(Value::Mapping(skill));
        selected
    }
}

pub fn remove_entry(value: &mut Value, target: &Target) {
    match target {
        Target::Registry(i, _) => {
            let registries = mapping(&mut value["registries"]);
            if let Some(key) = registries.keys().nth(*i).cloned() {
                registries.remove(key);
            }
        }
        Target::Skill(i, _) => {
            value["skills"].as_sequence_mut().unwrap().remove(*i);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXISTING: &str = "# keep this comment on no-op save\nname: existing\nversion: '1.2'\nagents: [codex, copilot]\nregistries:\n  company: /offline/registry\nskills:\n  - name: local/helper\n    version: v2\n    source: company\n    path: ./local/helper\n    extension: keep-skill-metadata\ntoolkit:\n  manifest: toolkit/manifest.yaml\n  version: '2.0'\n  extension: keep-toolkit-metadata\nbundles: [core]\nprofiles: [reviewer]\nworkspace:\n  standard: workspace-docs@5.0.0\n  source: workspace/standards\ntrusted_sources: [workspace/standards]\nextension:\n  enabled: true\n";

    fn existing() -> (tempfile::TempDir, Document) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("skills.yaml");
        fs::write(&path, EXISTING).unwrap();
        let document = Document::load(&path, None, false).unwrap();
        (dir, document)
    }

    #[test]
    fn fresh_draft_and_script_defaults_are_not_written_until_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("skills.yaml");
        let document = Document::load(&path, Some("my-project"), false).unwrap();
        assert_eq!(document.value["name"], "my-project");
        assert!(document.value["skills"].as_sequence().unwrap().is_empty());
        assert!(document.changed());
        assert!(!path.exists());
        let scripted = Document::load(&path, None, true).unwrap();
        assert_eq!(
            scripted.value["skills"][0]["name"],
            "software-development/spec"
        );
        assert!(document.save(false).unwrap());
        assert_eq!(
            SkillsConfig::load_from_file(path).unwrap().name,
            "my-project"
        );
    }

    #[test]
    fn no_op_keeps_exact_yaml_and_preview() {
        let (_dir, document) = existing();
        assert!(document.exists());
        assert!(!document.changed());
        assert_eq!(document.preview().unwrap(), EXISTING);
        assert!(!document.save(false).unwrap());
        assert_eq!(fs::read_to_string(&document.path).unwrap(), EXISTING);
    }

    #[test]
    fn edits_preserve_unrelated_and_unknown_values_at_every_depth() {
        let (_dir, mut document) = existing();
        let mut expected = document.value.clone();
        Target::Field("name")
            .write(&mut document.value, "edited")
            .unwrap();
        Target::Skill(0, "version")
            .write(&mut document.value, "v3")
            .unwrap();
        Target::Nested("toolkit", "version")
            .write(&mut document.value, "3.0")
            .unwrap();
        expected["name"] = Value::String("edited".into());
        expected["skills"][0]["version"] = Value::String("v3".into());
        expected["toolkit"]["version"] = Value::String("3.0".into());
        let preview = document.preview().unwrap();
        assert_eq!(fs::read_to_string(&document.path).unwrap(), EXISTING);
        assert!(document.save(false).unwrap());
        assert_eq!(fs::read_to_string(&document.path).unwrap(), preview);
        let saved: Value = serde_yaml::from_str(&preview).unwrap();
        assert_eq!(saved, expected);
    }

    #[test]
    fn external_edits_creation_and_deletion_block_save() {
        let (_dir, mut document) = existing();
        document.value["name"] = Value::String("draft".into());
        fs::write(&document.path, "name: changed-elsewhere\n").unwrap();
        assert!(document
            .save(false)
            .unwrap_err()
            .to_string()
            .contains("changed outside"));
        assert_eq!(
            fs::read_to_string(&document.path).unwrap(),
            "name: changed-elsewhere\n"
        );
        fs::remove_file(&document.path).unwrap();
        assert!(document.save(false).is_err());
        let fresh = Document::load(&document.path, None, false).unwrap();
        fs::write(&document.path, "name: appeared\n").unwrap();
        assert!(fresh.save(false).is_err());
        assert_eq!(
            fs::read_to_string(&document.path).unwrap(),
            "name: appeared\n"
        );
    }

    #[test]
    fn malformed_yaml_wrong_types_and_directories_are_not_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("skills.yaml");
        for text in [
            "name: [",
            "name: test\nskills: false\n",
            "name: one\nname: two\n",
            "- not-a-manifest",
        ] {
            fs::write(&path, text).unwrap();
            assert!(Document::load(&path, None, false).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), text);
        }
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(Document::load(&path, None, false).is_err());
    }

    #[test]
    fn non_interactive_init_refuses_existing_file() {
        let (_dir, document) = existing();
        assert!(Document::load(&document.path, Some("replacement"), true).is_err());
        assert_eq!(fs::read_to_string(&document.path).unwrap(), EXISTING);
    }

    #[test]
    fn registry_and_skill_edits_keep_order_and_metadata() {
        let (_dir, mut document) = existing();
        let original_skill = document.value["skills"][0].clone();
        let index = add_entry(&mut document.value, 2);
        assert_eq!(index, 2);
        let before = document.value.clone();
        assert!(Target::Registry(1, true)
            .write(&mut document.value, "company")
            .is_err());
        assert_eq!(document.value, before);
        Target::Registry(1, true)
            .write(&mut document.value, "personal")
            .unwrap();
        Target::Registry(1, false)
            .write(&mut document.value, "/my/registry")
            .unwrap();
        assert_eq!(Target::Registry(1, true).read(&document.value), "personal");
        assert_eq!(add_entry(&mut document.value, 3), 4);
        Target::Skill(1, "name")
            .write(&mut document.value, "local/new")
            .unwrap();
        Target::Skill(1, "path")
            .write(&mut document.value, "./my skill")
            .unwrap();
        document.validate(false).unwrap();
        remove_entry(&mut document.value, &Target::Registry(1, false));
        remove_entry(&mut document.value, &Target::Skill(1, "path"));
        assert_eq!(
            document.value["skills"].as_sequence().unwrap(),
            &[original_skill]
        );
        assert_eq!(document.value["registries"].as_mapping().unwrap().len(), 1);
    }

    #[test]
    fn optional_fields_can_be_added_and_cleared_without_removing_siblings() {
        let (_dir, mut document) = existing();
        Target::Nested("workspace", "revision")
            .write(&mut document.value, "abc")
            .unwrap();
        Target::Nested("workspace", "revision")
            .write(&mut document.value, "")
            .unwrap();
        assert!(document.value["workspace"]["revision"].is_null());
        assert_eq!(
            document.value["workspace"]["standard"],
            "workspace-docs@5.0.0"
        );
        Target::Field("profiles")
            .write(&mut document.value, " security, reviewer, ")
            .unwrap();
        assert_eq!(
            Target::Field("profiles").read(&document.value),
            "security, reviewer"
        );
        Target::Skill(0, "source")
            .write(&mut document.value, "")
            .unwrap();
        assert!(document.value["skills"][0]["source"].is_null());
        assert_eq!(document.value["skills"][0]["path"], "./local/helper");
        Target::Nested("toolkit", "manifest")
            .write(&mut document.value, "")
            .unwrap();
        assert!(document.value["toolkit"].is_null());
        assert!(document.validate(false).is_err()); // bundles still need a toolkit
    }

    #[test]
    fn invalid_names_sources_agents_and_incomplete_entries_cannot_save() {
        let (_dir, mut document) = existing();
        let original = document.value.clone();
        for (target, text) in [
            (Target::Field("name"), "  "),
            (Target::Registry(0, true), "../escape"),
            (Target::Registry(0, false), "  "),
            (Target::Skill(0, "name"), "../escape"),
            (Target::Skill(0, "source"), "../escape"),
            (Target::Skill(0, "path"), "  "),
            (Target::Nested("toolkit", "manifest"), "/absolute/path"),
            (Target::Nested("workspace", "standard"), "  "),
        ] {
            document.value = original.clone();
            target.write(&mut document.value, text).unwrap();
            assert!(
                document.save(false).is_err(),
                "invalid input was saved: {text}"
            );
            assert_eq!(fs::read_to_string(&document.path).unwrap(), EXISTING);
        }
        document.value = original.clone();
        Target::Agent("unsupported".into())
            .write(&mut document.value, "")
            .unwrap();
        assert!(document.validate(false).is_err());
        document.value = original;
        assert!(document.validate(true).is_err());
        add_entry(&mut document.value, 3);
        assert!(document.validate(false).is_err());
        Target::Skill(1, "name")
            .write(&mut document.value, "local/helper")
            .unwrap();
        assert!(document
            .validate(false)
            .unwrap_err()
            .to_string()
            .contains("Duplicate"));
    }

    #[test]
    fn agents_include_all_supported_choices_and_keep_existing_selection() {
        let (_dir, mut document) = existing();
        let choices = fields(&document.value, 1);
        assert_eq!(choices.len(), 6);
        assert_eq!(Target::Agent("copilot".into()).read(&document.value), "[x]");
        let target = Target::Agent("claude".into());
        target.write(&mut document.value, "").unwrap();
        assert_eq!(target.read(&document.value), "[x]");
        target.write(&mut document.value, "").unwrap();
        assert_eq!(target.read(&document.value), "[ ]");
        assert_eq!(document.value["agents"], document.initial["agents"]);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_rejected_at_load_and_save_and_permissions_survive() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let (dir, mut document) = existing();
        fs::set_permissions(&document.path, fs::Permissions::from_mode(0o640)).unwrap();
        document.value["name"] = Value::String("updated".into());
        document.save(false).unwrap();
        assert_eq!(
            fs::metadata(&document.path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        let target = dir.path().join("target.yaml");
        fs::rename(&document.path, &target).unwrap();
        symlink(&target, &document.path).unwrap();
        assert!(Document::load(&document.path, None, false).is_err());
        assert!(document.save(false).is_err());
        fs::remove_file(&target).unwrap();
        assert!(Document::load(&document.path, None, false).is_err());
        assert!(!target.exists());
    }
}
