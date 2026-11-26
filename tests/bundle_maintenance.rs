#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture {
    root: tempfile::TempDir,
    project: PathBuf,
    home: PathBuf,
    registry: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        let home = root.path().join("home");
        let registry = root.path().join("registry");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&registry).unwrap();
        let fixture = Self {
            root,
            project,
            home,
            registry,
        };
        fixture.skill("alpha", "1.0.0", Some("acme/helper@1.0.0"));
        fixture.skill("helper", "1.0.0", None);
        fixture.manifest(
            "alpha: 1.0.0, helper: 1.0.0",
            "starter: {packages: [alpha]}, other: {packages: [helper]}",
        );
        git(&fixture.registry, &["init", "--quiet"]);
        fixture.commit();
        let config = fixture.home.join(".config/skm");
        fs::create_dir_all(&config).unwrap();
        fs::write(
            config.join("config.yaml"),
            format!(
                "default_registry: local\nregistries:\n  local: {}\ncheck_for_updates: false\n",
                fixture.registry.display()
            ),
        )
        .unwrap();
        fs::write(fixture.project.join("skills.yaml"), format!("name: fixture\nregistries:\n  local: {}\nagents: [codex]\nskills:\n  - {{name: acme/starter, kind: bundle, source: local}}\n", fixture.registry.display())).unwrap();
        fixture.skm(&["cache", "refresh", "local"]);
        fixture
    }

    fn skill(&self, name: &str, version: &str, dependency: Option<&str>) {
        let dir = self.registry.join(format!("skills/acme/{name}/v{version}"));
        fs::create_dir_all(&dir).unwrap();
        let dependency = dependency
            .map(|s| format!("  skm-dependencies: \"{s}\"\n"))
            .unwrap_or_default();
        fs::write(dir.join("SKILL.md"), format!("---\nname: {name}\nmetadata:\n  skm-version: \"{version}\"\n{dependency}---\n# {name}\n")).unwrap();
    }

    fn manifest(&self, packages: &str, bundles: &str) {
        fs::write(self.registry.join("skills/acme/manifest.yaml"), format!("schema_version: 2\nnamespace: acme\npackages: {{{packages}}}\nbundles: {{{bundles}}}\n")).unwrap();
    }

    fn commit(&self) {
        git(&self.registry, &["add", "."]);
        git(
            &self.registry,
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

    fn command(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_skm"))
            .args(args)
            .current_dir(&self.project)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("XDG_CACHE_HOME", self.home.join(".cache"))
            .env("SKM_NO_UPDATE_CHECK", "1")
            .output()
            .unwrap()
    }

    fn skm(&self, args: &[&str]) -> Output {
        let result = self.command(args);
        assert!(
            result.status.success(),
            "skm {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        result
    }

    fn lock(&self) -> serde_yaml::Value {
        serde_yaml::from_slice(&fs::read(self.project.join("skills.lock.yaml")).unwrap()).unwrap()
    }
}

fn git(root: &Path, args: &[&str]) {
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
}

#[test]
fn clean_preserves_bundle_members_and_removal_preserves_other_owners() {
    let fixture = Fixture::new();
    let path = fixture.project.join("skills.yaml");
    let mut manifest = fs::read_to_string(&path).unwrap();
    manifest.push_str("  - {name: acme/other, kind: bundle, source: local}\n  - {name: acme/alpha, version: 1.0.0, source: local}\n");
    fs::write(path, manifest).unwrap();
    fixture.skm(&["install", "--yes"]);
    fixture.skm(&["check"]);
    let stray = fixture.root.path().join("stray");
    fs::create_dir(&stray).unwrap();
    symlink(&stray, fixture.project.join(".agents/skills/stray")).unwrap();
    let output = fixture.skm(&["clean", "symlinks", "--all", "--dry-run"]);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("stray"));
    assert!(!text.contains("alpha"));
    assert!(!text.contains("helper"));
    fixture.skm(&["remove", "acme/starter", "--yes"]);
    assert!(fixture.project.join(".agents/skills/alpha").is_symlink());
    assert!(fixture.project.join(".agents/skills/helper").is_symlink());
    let lock = fixture.lock();
    assert_eq!(lock["bundles"].as_sequence().unwrap().len(), 1);
    assert_eq!(lock["bundles"][0]["name"], "acme/other");
    fixture.skm(&["check"]);
    fixture.skm(&["remove", "acme/alpha", "--yes"]);
    assert!(!fixture.project.join(".agents/skills/alpha").exists());
    assert!(fixture.project.join(".agents/skills/helper").is_symlink());
    fixture.skm(&["check"]);
    fixture.skm(&["remove", "acme/other", "--yes"]);
    assert!(!fixture.project.join(".agents/skills/helper").exists());
    assert!(fixture.lock().get("bundles").is_none());
    fixture.skm(&["check"]);
}

#[test]
fn refreshed_bundle_drift_is_reported_and_upgrade_reconciles_lock_and_links() {
    let fixture = Fixture::new();
    fixture.skm(&["install", "--yes"]);
    fixture.skm(&["check"]);
    fixture.skill("beta", "1.0.0", None);
    fixture.skill("alpha", "1.1.0", Some("acme/helper@1.0.0"));
    fixture.manifest(
        "alpha: 1.1.0, helper: 1.0.0, beta: 1.0.0",
        "starter: {packages: [alpha, beta]}, other: {packages: [helper]}",
    );
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    assert!(!fixture.command(&["check"]).status.success());
    let output = fixture.skm(&["skill", "outdated", "acme/starter"]);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("acme/alpha: 1.0.0 -> 1.1.0"), "{text}");
    assert!(text.contains("added acme/beta@1.0.0"), "{text}");
    let lock_before = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
    fixture.skm(&["skill", "upgrade", "acme/starter", "--dry-run"]);
    assert_eq!(
        lock_before,
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap()
    );
    assert!(!fixture.project.join(".agents/skills/beta").exists());
    fixture.skm(&["skill", "upgrade", "acme/starter", "--yes"]);
    assert!(fixture.project.join(".agents/skills/beta").is_symlink());
    fixture.skm(&["check"]);
}

#[test]
fn removal_uses_installed_members_even_after_registry_drift() {
    let fixture = Fixture::new();
    fixture.skm(&["install", "--yes"]);
    fixture.skill("beta", "1.0.0", None);
    fixture.manifest(
        "alpha: 1.0.0, helper: 1.0.0, beta: 1.0.0",
        "starter: {packages: [beta]}, other: {packages: [helper]}",
    );
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    fs::remove_dir_all(
        fixture
            .home
            .join(".cache/skm/registries/local/skills/acme/alpha/v1.0.0"),
    )
    .unwrap();
    fixture.skm(&["remove", "acme/starter", "--yes"]);
    assert!(fs::symlink_metadata(fixture.project.join(".agents/skills/alpha")).is_err());
    assert!(!fixture.project.join(".agents/skills/helper").exists());
    assert!(!fixture.project.join(".agents/skills/beta").exists());
    assert!(fixture.lock().get("bundles").is_none());
    fixture.skm(&["check"]);
}

#[test]
fn unexpected_bundle_link_blocks_removal_before_manifest_or_lock_changes() {
    let fixture = Fixture::new();
    fixture.skm(&["install", "--yes"]);
    let config = fs::read(fixture.project.join("skills.yaml")).unwrap();
    let lock = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
    let target = fixture.project.join(".agents/skills/alpha");
    fs::remove_file(&target).unwrap();
    let unexpected = fixture.root.path().join("unexpected");
    fs::create_dir(&unexpected).unwrap();
    symlink(&unexpected, &target).unwrap();
    let result = fixture.command(&["remove", "acme/starter", "--yes"]);
    assert!(!result.status.success());
    assert_eq!(
        fs::read(fixture.project.join("skills.yaml")).unwrap(),
        config
    );
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        lock
    );
    assert_eq!(fs::read_link(target).unwrap(), unexpected);
    assert!(fixture.project.join(".agents/skills/helper").is_symlink());
}

#[test]
fn removal_handles_pending_entries_without_disturbing_installed_owners() {
    let fixture = Fixture::new();
    fixture.skm(&["install", "--yes"]);
    let manifest_path = fixture.project.join("skills.yaml");
    let installed_manifest = fs::read_to_string(&manifest_path).unwrap();
    let installed_lock = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
    let pending = "  - {name: acme/other, kind: bundle, source: local}\n  - {name: acme/pending-skill, source: local}\n";
    fs::write(&manifest_path, format!("{installed_manifest}{pending}")).unwrap();
    fixture.skm(&["remove", "acme/other", "--yes"]);
    assert!(fixture.project.join(".agents/skills/alpha").is_symlink());
    assert!(fixture.project.join(".agents/skills/helper").is_symlink());
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        installed_lock
    );
    fixture.skm(&["remove", "acme/pending-skill", "--yes"]);
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        installed_lock
    );
    fixture.skm(&["check"]);

    // An installed other bundle and standalone still own their shared members;
    // unrelated entries added since installation do not block removal.
    let append_entries = |entries: &str| {
        let mut manifest: serde_yaml::Value =
            serde_yaml::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        let extra: Vec<serde_yaml::Value> = serde_yaml::from_str(entries).unwrap();
        manifest["skills"].as_sequence_mut().unwrap().extend(extra);
        fs::write(&manifest_path, serde_yaml::to_string(&manifest).unwrap()).unwrap();
    };
    append_entries("- {name: acme/other, kind: bundle, source: local}\n- {name: acme/alpha, version: 1.0.0, source: local}\n");
    fixture.skm(&["install", "--yes"]);
    append_entries("- {name: acme/pending, kind: bundle, source: local}\n- {name: acme/pending-skill, source: local}\n");
    fixture.skm(&["remove", "acme/starter", "--yes"]);
    assert!(fixture.project.join(".agents/skills/alpha").is_symlink());
    assert!(fixture.project.join(".agents/skills/helper").is_symlink());
    let lock = fixture.lock();
    assert_eq!(lock["bundles"].as_sequence().unwrap().len(), 1);
    assert_eq!(lock["bundles"][0]["name"], "acme/other");
}

#[test]
fn clean_keeps_installed_members_after_cached_bundle_changes() {
    let fixture = Fixture::new();
    fixture.skm(&["install", "--yes"]);
    fixture.skill("beta", "1.0.0", None);
    fixture.manifest(
        "alpha: 1.0.0, helper: 1.0.0, beta: 1.0.0",
        "starter: {packages: [beta]}, other: {packages: [helper]}",
    );
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    let stray = fixture.root.path().join("stray");
    fs::create_dir(&stray).unwrap();
    symlink(&stray, fixture.project.join(".agents/skills/stray")).unwrap();
    let output = fixture.skm(&["clean", "symlinks", "--all", "--dry-run"]);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("stray"), "{text}");
    assert!(!text.contains("alpha"), "{text}");
    assert!(!text.contains("helper"), "{text}");
}
