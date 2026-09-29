#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture {
    root: tempfile::TempDir,
    project: PathBuf,
    home: PathBuf,
    config_home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        let home = root.path().join("home");
        let config_home = root.path().join("config");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(config_home.join("skm")).unwrap();
        Self {
            root,
            project,
            home,
            config_home,
        }
    }

    fn skm(&self, args: &[&str]) -> Output {
        let output = Command::new(env!("CARGO_BIN_EXE_skm"))
            .args(args)
            .current_dir(&self.project)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env("SKM_NO_UPDATE_CHECK", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "skm {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn base_config(&self, default: &str, registry: &Path) {
        fs::write(
            self.config_home.join("skm/config.yaml"),
            format!(
                "default_registry: {default}\nregistries:\n  {default}: {}\ncheck_for_updates: false\n",
                registry.display()
            ),
        )
        .unwrap();
    }
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn registry_with_skill(root: &Path, name: &str) {
    let version = root.join(format!("skills/acme/{name}/v1.0.0"));
    fs::create_dir_all(&version).unwrap();
    fs::write(
        version.join("SKILL.md"),
        format!("---\nname: {name}\nmetadata:\n  skm-version: \"1.0.0\"\n---\n# {name}\n"),
    )
    .unwrap();
    symlink("v1.0.0", version.parent().unwrap().join("latest")).unwrap();
    git(root, &["init", "--quiet"]);
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ],
    );
}

#[test]
fn ordinary_install_previews_links_and_lock_then_repeats_without_changes() {
    let fixture = Fixture::new();
    let source = fixture.project.join("alpha");
    fs::create_dir(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: alpha\ndescription: Fixture skill.\n---\n# Alpha\n",
    )
    .unwrap();
    fs::write(
        fixture.project.join("skills.yaml"),
        "name: fixture\nagents: [codex, cursor]\nskills:\n  - name: alpha\n    path: alpha\n",
    )
    .unwrap();
    let preview = fixture.skm(&["install", "--dry-run", "--json"]);
    let plan: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(plan["actions"].as_array().unwrap().len(), 3);
    assert!(!fixture.project.join("skills.lock.yaml").exists());
    assert!(!fixture.project.join(".agents/skills/alpha").exists());
    assert!(!fixture.config_home.join("skm/config.yaml").exists());
    fixture.skm(&["install", "--yes"]);
    fixture.skm(&["check"]);
    let lock = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
    let repeat = fixture.skm(&["install", "--dry-run", "--json"]);
    let repeat_plan: serde_json::Value = serde_json::from_slice(&repeat.stdout).unwrap();
    assert!(repeat_plan["actions"].as_array().unwrap().is_empty());
    fixture.skm(&["install", "--yes"]);
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        lock
    );

    let beta = fixture.project.join("beta");
    fs::create_dir(&beta).unwrap();
    fs::write(
        beta.join("SKILL.md"),
        "---\nname: beta\ndescription: Fixture skill.\n---\n# Beta\n",
    )
    .unwrap();
    fixture.skm(&["add", "beta", "--path", "beta", "--yes"]);
    fixture.skm(&["check"]);
    let expanded = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
    assert_ne!(expanded, lock);
    fixture.skm(&["remove", "beta", "--yes"]);
    fixture.skm(&["check"]);
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        lock
    );
    fixture.skm(&["install", "--global", "--yes"]);
    assert!(fixture.home.join(".agents/skills/alpha").is_symlink());
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        lock
    );
}

#[test]
fn project_registry_override_and_selected_default_drive_new_additions() {
    let fixture = Fixture::new();
    let global_repo = fixture.root.path().join("global-repo");
    let project_repo = fixture.root.path().join("project-repo");
    fs::create_dir(&global_repo).unwrap();
    fs::create_dir(&project_repo).unwrap();
    registry_with_skill(&global_repo, "wrong");
    registry_with_skill(&project_repo, "right");
    fixture.base_config("custom", &global_repo);
    fs::write(
        fixture.project.join("skills.yaml"),
        format!(
            "name: fixture\nregistries:\n  custom: {}\nagents: [codex]\nskills: []\n",
            project_repo.display()
        ),
    )
    .unwrap();
    fixture.skm(&["registry", "set-default", "custom"]);
    fixture.skm(&["cache", "refresh", "custom"]);
    let cache = fixture.home.join(".cache/skm/registries/custom");
    assert_eq!(
        git(&cache, &["remote", "get-url", "origin"]),
        project_repo.display().to_string()
    );
    let versions = fixture.skm(&["skill", "versions", "acme/right", "--json"]);
    let version_output: serde_json::Value = serde_json::from_slice(&versions.stdout).unwrap();
    assert_eq!(version_output["registry"], "custom");
    fixture.skm(&["add", "acme/right", "--kind", "skill", "--yes"]);
    let manifest = fs::read_to_string(fixture.project.join("skills.yaml")).unwrap();
    assert!(manifest.contains("source: custom"));
    assert!(fixture.project.join(".agents/skills/right").is_symlink());
}

#[test]
fn bulk_upgrade_previews_and_updates_all_exact_pins() {
    let fixture = Fixture::new();
    let cache = fixture.home.join(".cache/skm/registries/default");
    fixture.base_config("default", &cache);
    for name in ["alpha", "beta"] {
        for version in ["1.0.0", "1.1.0"] {
            let path = cache.join(format!("skills/acme/{name}/v{version}"));
            fs::create_dir_all(&path).unwrap();
            fs::write(
                path.join("SKILL.md"),
                format!(
                    "---\nname: {name}\nmetadata:\n  skm-version: \"{version}\"\n---\n# {name}\n"
                ),
            )
            .unwrap();
        }
    }
    let local = fixture.project.join("local");
    fs::create_dir(&local).unwrap();
    fs::write(
        local.join("SKILL.md"),
        "---\nname: local\ndescription: Fixture skill.\n---\n# Local\n",
    )
    .unwrap();
    let manifest_path = fixture.project.join("skills.yaml");
    fs::write(
        &manifest_path,
        "name: fixture\nagents: [codex]\nskills:\n  - name: acme/alpha\n    source: default\n    version: v1.0.0\n  - name: acme/beta\n    source: default\n    version: v1.0.0\n  - name: local\n    path: local\n",
    )
    .unwrap();
    let before = fs::read(&manifest_path).unwrap();
    fixture.skm(&["skill", "upgrade", "--all", "--dry-run"]);
    assert_eq!(fs::read(&manifest_path).unwrap(), before);
    assert!(!fixture.project.join("skills.lock.yaml").exists());
    fixture.skm(&["skill", "upgrade", "--all", "--yes"]);
    fixture.skm(&["check"]);
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    assert_eq!(manifest.matches("version: v1.1.0").count(), 2);
    assert!(manifest.contains("path: local"));
    assert!(fixture.project.join(".agents/skills/alpha").is_symlink());
    assert!(fixture.project.join("skills.lock.yaml").exists());
}
