#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

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
fn remove_rejects_manifest_edits_while_waiting_for_confirmation() {
    let fixture = Fixture::new();
    fixture.skm(&["install", "--yes"]);
    let manifest = fixture.project.join("skills.yaml");
    let lock = fixture.project.join("skills.lock.yaml");
    let lock_before = fs::read(&lock).unwrap();
    let alpha = fixture.project.join(".agents/skills/alpha");
    let helper = fixture.project.join(".agents/skills/helper");
    let alpha_before = fs::read_link(&alpha).unwrap();
    let helper_before = fs::read_link(&helper).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_skm"))
        .args(["remove", "acme/starter"])
        .current_dir(&fixture.project)
        .env("HOME", &fixture.home)
        .env("XDG_CONFIG_HOME", fixture.home.join(".config"))
        .env("XDG_CACHE_HOME", fixture.home.join(".cache"))
        .env("SKM_NO_UPDATE_CHECK", "1")
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let mut prompt = Vec::new();
    while !prompt.ends_with(b"[y/N] ") {
        let mut byte = [0];
        assert_eq!(
            stderr.read(&mut byte).unwrap(),
            1,
            "removal ended before confirmation"
        );
        prompt.push(byte[0]);
    }
    let mut edited = fs::read_to_string(&manifest).unwrap();
    edited.push_str("  - {name: acme/alpha, version: 1.0.0, source: local}\n");
    fs::write(&manifest, &edited).unwrap();
    child.stdin.take().unwrap().write_all(b"y\n").unwrap();
    let status = child.wait().unwrap();
    assert!(!status.success());
    assert_eq!(fs::read_to_string(&manifest).unwrap(), edited);
    assert_eq!(fs::read(&lock).unwrap(), lock_before);
    assert_eq!(fs::read_link(alpha).unwrap(), alpha_before);
    assert_eq!(fs::read_link(helper).unwrap(), helper_before);
}

fn confirm_removal_after(
    fixture: &Fixture,
    during_prompt: impl FnOnce(),
) -> std::process::ExitStatus {
    let mut child = Command::new(env!("CARGO_BIN_EXE_skm"))
        .args(["remove", "acme/starter"])
        .current_dir(&fixture.project)
        .env("HOME", &fixture.home)
        .env("XDG_CONFIG_HOME", fixture.home.join(".config"))
        .env("XDG_CACHE_HOME", fixture.home.join(".cache"))
        .env("SKM_NO_UPDATE_CHECK", "1")
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let mut prompt = Vec::new();
    while !prompt.ends_with(b"[y/N] ") {
        let mut byte = [0];
        assert_eq!(
            stderr.read(&mut byte).unwrap(),
            1,
            "removal ended before confirmation"
        );
        prompt.push(byte[0]);
    }
    during_prompt();
    child.stdin.take().unwrap().write_all(b"y\n").unwrap();
    child.wait().unwrap()
}

#[test]
fn remove_rejects_lock_changes_and_creation_while_waiting_for_confirmation() {
    for initially_present in [true, false] {
        let fixture = Fixture::new();
        fixture.skm(&["install", "--yes"]);
        let manifest = fixture.project.join("skills.yaml");
        let before = fs::read(&manifest).unwrap();
        let lock = fixture.project.join("skills.lock.yaml");
        let original_lock = fs::read(&lock).unwrap();
        if !initially_present {
            fs::remove_file(&lock).unwrap();
        }
        let alpha = fixture.project.join(".agents/skills/alpha");
        let helper = fixture.project.join(".agents/skills/helper");
        let alpha_before = fs::read_link(&alpha).unwrap();
        let helper_before = fs::read_link(&helper).unwrap();
        let mut changed = original_lock;
        changed.extend_from_slice(b"\n# edited during confirmation\n");
        let status = confirm_removal_after(&fixture, || fs::write(&lock, &changed).unwrap());
        assert!(!status.success());
        assert_eq!(fs::read(manifest).unwrap(), before);
        assert_eq!(fs::read(lock).unwrap(), changed);
        assert_eq!(fs::read_link(alpha).unwrap(), alpha_before);
        assert_eq!(fs::read_link(helper).unwrap(), helper_before);
    }
}

#[test]
fn remove_rejects_target_replacement_while_waiting_for_confirmation() {
    let fixture = Fixture::new();
    fixture.skm(&["install", "--yes"]);
    let manifest = fixture.project.join("skills.yaml");
    let lock = fixture.project.join("skills.lock.yaml");
    let manifest_before = fs::read(&manifest).unwrap();
    let lock_before = fs::read(&lock).unwrap();
    let alpha = fixture.project.join(".agents/skills/alpha");
    let helper = fixture.project.join(".agents/skills/helper");
    let helper_before = fs::read_link(&helper).unwrap();
    let outside = fixture.root.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let status = confirm_removal_after(&fixture, || {
        fs::remove_file(&alpha).unwrap();
        symlink(&outside, &alpha).unwrap();
    });
    assert!(!status.success());
    assert_eq!(fs::read(manifest).unwrap(), manifest_before);
    assert_eq!(fs::read(lock).unwrap(), lock_before);
    assert_eq!(fs::read_link(alpha).unwrap(), outside);
    assert_eq!(fs::read_link(helper).unwrap(), helper_before);
}

#[test]
fn remove_rejects_manifest_symlink_swap_during_confirmation() {
    let fixture = Fixture::new();
    fixture.skm(&["install", "--yes"]);
    let manifest = fixture.project.join("skills.yaml");
    let original = fs::read(&manifest).unwrap();
    let outside = fixture.root.path().join("outside.yaml");
    fs::write(&outside, &original).unwrap();
    let lock = fixture.project.join("skills.lock.yaml");
    let lock_before = fs::read(&lock).unwrap();
    let alpha = fixture.project.join(".agents/skills/alpha");
    let helper = fixture.project.join(".agents/skills/helper");
    let alpha_before = fs::read_link(&alpha).unwrap();
    let helper_before = fs::read_link(&helper).unwrap();
    let status = confirm_removal_after(&fixture, || {
        fs::remove_file(&manifest).unwrap();
        symlink(&outside, &manifest).unwrap();
    });
    assert!(!status.success());
    assert!(manifest.is_symlink());
    assert_eq!(fs::read(outside).unwrap(), original);
    assert_eq!(fs::read(lock).unwrap(), lock_before);
    assert_eq!(fs::read_link(alpha).unwrap(), alpha_before);
    assert_eq!(fs::read_link(helper).unwrap(), helper_before);
}

#[test]
fn remove_rejects_symlinked_target_parent_swap_during_confirmation() {
    let fixture = Fixture::new();
    fixture.skm(&["install", "--yes"]);
    let manifest = fixture.project.join("skills.yaml");
    let original = fs::read(&manifest).unwrap();
    let lock = fixture.project.join("skills.lock.yaml");
    let lock_before = fs::read(&lock).unwrap();
    let targets = fixture.project.join(".agents/skills");
    let saved = fixture.project.join(".agents/skills-saved");
    let outside = fixture.root.path().join("outside-skills");
    fs::create_dir(&outside).unwrap();
    for skill in ["alpha", "helper"] {
        symlink(
            fs::read_link(targets.join(skill)).unwrap(),
            outside.join(skill),
        )
        .unwrap();
    }
    let status = confirm_removal_after(&fixture, || {
        fs::rename(&targets, &saved).unwrap();
        symlink(&outside, &targets).unwrap();
    });
    assert!(!status.success());
    assert_eq!(fs::read(manifest).unwrap(), original);
    assert_eq!(fs::read(lock).unwrap(), lock_before);
    assert!(targets.is_symlink());
    for skill in ["alpha", "helper"] {
        assert!(outside.join(skill).is_symlink());
        assert!(saved.join(skill).is_symlink());
    }
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
            .join(".skm/cache/registries/local/skills/acme/alpha/v1.0.0"),
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

#[test]
fn versioned_bundles_resolve_history_and_preserve_exact_pins() {
    let fixture = Fixture::new();
    fixture.skill("beta", "2.0.0", None);
    fixture.skill("alpha", "2.0.0", None);
    fs::write(fixture.registry.join("skills/acme/manifest.yaml"), "schema_version: 2\nnamespace: acme\npackages: {beta: 2.0.0, helper: 1.0.0}\nbundles: {starter: {packages: [beta]}, other: {packages: [helper]}}\nbundle_versions:\n  starter:\n    1.9.0: {alpha: 1.0.0}\n    1.10.0: {beta: 2.0.0}\n").unwrap();
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    fixture.skm(&["install", "--yes"]);
    assert_eq!(fixture.lock()["bundles"][0]["resolved_version"], "1.10.0");
    assert!(fixture.project.join(".agents/skills/beta").is_symlink());
    let config = fs::read(fixture.project.join("skills.yaml")).unwrap();
    let lock = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
    fixture.skm(&["skill", "use", "acme/starter@1.9.0", "--dry-run"]);
    assert_eq!(
        fs::read(fixture.project.join("skills.yaml")).unwrap(),
        config
    );
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        lock
    );
    fixture.skm(&["skill", "use", "acme/starter@1.9.0", "--yes"]);
    assert_eq!(fixture.lock()["bundles"][0]["version"], "1.9.0");
    assert_eq!(fixture.lock()["bundles"][0]["resolved_version"], "1.9.0");
    assert!(fixture.project.join(".agents/skills/alpha").is_symlink());
    assert!(fixture.project.join(".agents/skills/helper").is_symlink());
    assert!(!fixture.project.join(".agents/skills/beta").exists());
    fixture.skm(&["check"]);
    let output = fixture.skm(&["skill", "outdated", "acme/starter"]);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("1.9.0 -> 1.10.0"), "{text}");
    assert!(text.contains("added acme/beta@2.0.0"), "{text}");
    let pinned = fs::read(fixture.project.join("skills.yaml")).unwrap();
    fixture.skm(&["skill", "upgrade", "--all", "--yes"]);
    assert_eq!(
        fs::read(fixture.project.join("skills.yaml")).unwrap(),
        pinned
    );
    fixture.skm(&["check"]);
}

#[test]
fn release_drift_and_bad_history_fail_safely() {
    let fixture = Fixture::new();
    let manifest = fixture.registry.join("skills/acme/manifest.yaml");
    let history = |releases: &str| {
        format!("schema_version: 2\nnamespace: acme\npackages: {{alpha: 1.0.0, helper: 1.0.0}}\nbundles: {{starter: {{packages: [alpha]}}, other: {{packages: [helper]}}}}\nbundle_versions:\n  starter:\n{releases}")
    };
    fs::write(&manifest, history("    1.0.0: {alpha: 1.0.0}\n")).unwrap();
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    fixture.skm(&["install", "--yes"]);
    fs::write(
        &manifest,
        history("    1.0.0: {alpha: 1.0.0}\n    1.1.0: {alpha: 1.0.0}\n"),
    )
    .unwrap();
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    assert!(!fixture.command(&["check"]).status.success());
    fixture.skm(&["skill", "upgrade", "--all", "--yes"]);
    fixture.skm(&["check"]);
    fixture.skm(&["skill", "use", "acme/starter@1.0.0", "--yes"]);
    fixture.skm(&["check"]);
    let lock = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
    let config = fs::read(fixture.project.join("skills.yaml")).unwrap();
    assert!(!fixture
        .command(&["skill", "use", "acme/starter@3.0.0", "--yes"])
        .status
        .success());
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        lock
    );
    assert_eq!(
        fs::read(fixture.project.join("skills.yaml")).unwrap(),
        config
    );
    fs::write(
        &manifest,
        history("    1.0.0: {helper: 1.0.0}\n    1.1.0: {alpha: 1.0.0}\n"),
    )
    .unwrap();
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    assert!(!fixture.command(&["check"]).status.success());
    fs::write(&manifest, history("    1.1.0: {helper: 1.0.0}\n")).unwrap();
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    assert!(!fixture.command(&["install", "--yes"]).status.success());
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        lock
    );
}

#[test]
fn legacy_bundle_exact_selection_requires_published_history() {
    let fixture = Fixture::new();
    fixture.skm(&["install", "--yes"]);
    let output = fixture.command(&["skill", "use", "acme/starter@1.0.0", "--yes"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no published version history"));
    fixture.skm(&["check"]);
}

#[test]
fn pinned_switch_preserves_shared_links_and_rolls_back_on_collision() {
    let fixture = Fixture::new();
    fixture.skill("beta", "1.0.0", None);
    fs::write(fixture.registry.join("skills/acme/manifest.yaml"), "schema_version: 2\nnamespace: acme\npackages: {beta: 1.0.0, helper: 1.0.0}\nbundles: {starter: {packages: [beta]}, other: {packages: [helper]}}\nbundle_versions:\n  starter:\n    1.0.0: {alpha: 1.0.0}\n    2.0.0: {beta: 1.0.0}\n").unwrap();
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    let config_path = fixture.project.join("skills.yaml");
    fs::write(&config_path, format!("name: fixture\nregistries:\n  local: {}\nagents: [codex]\nskills:\n  - {{name: acme/starter, kind: bundle, source: local, version: 1.0.0}}\n  - {{name: acme/helper, source: local, version: 1.0.0}}\n", fixture.registry.display())).unwrap();
    fixture.skm(&["install", "--yes"]);
    let config = fs::read(&config_path).unwrap();
    let lock = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
    let collision = fixture.project.join(".agents/skills/beta");
    fs::create_dir(&collision).unwrap();
    assert!(!fixture
        .command(&["skill", "use", "acme/starter@2.0.0", "--yes"])
        .status
        .success());
    assert_eq!(fs::read(&config_path).unwrap(), config);
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        lock
    );
    assert!(fixture.project.join(".agents/skills/alpha").is_symlink());
    fs::remove_dir(&collision).unwrap();
    fixture.skm(&["skill", "use", "acme/starter@2.0.0", "--yes"]);
    assert!(fixture.project.join(".agents/skills/helper").is_symlink());
    assert!(!fixture.project.join(".agents/skills/alpha").exists());
    fixture.skm(&["remove", "acme/starter", "--yes"]);
    assert!(fixture.project.join(".agents/skills/helper").is_symlink());
    fixture.skm(&["check"]);
}

#[test]
fn named_upgrade_reconciles_only_requested_bundle() {
    let fixture = Fixture::new();
    let path = fixture.project.join("skills.yaml");
    let mut config = fs::read_to_string(&path).unwrap();
    config.push_str("  - {name: acme/other, kind: bundle, source: local}\n");
    fs::write(&path, config).unwrap();
    fixture.skm(&["install", "--yes"]);
    let old_lock = fixture.lock();
    let old_other = old_lock["bundles"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "acme/other")
        .unwrap()
        .clone();
    fixture.skill("beta", "1.0.0", None);
    fixture.skill("gamma", "1.0.0", None);
    fixture.manifest(
        "alpha: 1.0.0, helper: 1.0.0, beta: 1.0.0, gamma: 1.0.0",
        "starter: {packages: [beta]}, other: {packages: [gamma]}",
    );
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    fixture.skm(&["skill", "upgrade", "acme/starter", "--yes"]);
    assert!(fixture.project.join(".agents/skills/beta").is_symlink());
    assert!(fixture.project.join(".agents/skills/helper").is_symlink());
    assert!(!fixture.project.join(".agents/skills/gamma").exists());
    let lock = fixture.lock();
    assert_eq!(
        *lock["bundles"]
            .as_sequence()
            .unwrap()
            .iter()
            .find(|b| b["name"] == "acme/other")
            .unwrap(),
        old_other
    );
}

#[test]
fn outdated_compares_selected_pin_to_latest_even_if_lock_is_latest() {
    let fixture = Fixture::new();
    fixture.skill("beta", "1.0.0", None);
    fs::write(fixture.registry.join("skills/acme/manifest.yaml"), "schema_version: 2\nnamespace: acme\npackages: {beta: 1.0.0, helper: 1.0.0}\nbundles: {starter: {packages: [beta]}, other: {packages: [helper]}}\nbundle_versions:\n  starter:\n    1.9.0: {alpha: 1.0.0}\n    1.11.0: {beta: 1.0.0}\n").unwrap();
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    fixture.skm(&["install", "--yes"]);
    let config_path = fixture.project.join("skills.yaml");
    let mut config: serde_yaml::Value =
        serde_yaml::from_slice(&fs::read(&config_path).unwrap()).unwrap();
    config["skills"][0]["version"] = serde_yaml::Value::String("1.9.0".into());
    fs::write(&config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
    let result = fixture.skm(&["skill", "outdated", "acme/starter"]);
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("1.9.0 -> 1.11.0"), "{text}");
    assert!(text.contains("added acme/beta@1.0.0"), "{text}");
    assert!(text.contains("skill use"), "{text}");
    assert!(!text.contains("1.11.0 -> 1.9.0"), "{text}");
}

#[test]
fn upgrading_second_bundle_keeps_lock_order_and_check_current() {
    let fixture = Fixture::new();
    let path = fixture.project.join("skills.yaml");
    let mut config = fs::read_to_string(&path).unwrap();
    config.push_str("  - {name: acme/other, kind: bundle, source: local}\n");
    fs::write(path, config).unwrap();
    let manifest = fixture.registry.join("skills/acme/manifest.yaml");
    let history = |other: &str| {
        format!("schema_version: 2\nnamespace: acme\npackages: {{alpha: 1.0.0, helper: 1.0.0}}\nbundles: {{starter: {{packages: [alpha]}}, other: {{packages: [helper]}}}}\nbundle_versions:\n  starter: {{1.0.0: {{alpha: 1.0.0}}}}\n  other: {{{other}}}\n")
    };
    fs::write(&manifest, history("1.0.0: {helper: 1.0.0}")).unwrap();
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    fixture.skm(&["install", "--yes"]);
    fs::write(
        &manifest,
        history("1.0.0: {helper: 1.0.0}, 1.1.0: {helper: 1.0.0}"),
    )
    .unwrap();
    fixture.commit();
    fixture.skm(&["cache", "refresh", "local"]);
    fixture.skm(&["skill", "upgrade", "acme/other", "--yes"]);
    let lock = fixture.lock();
    assert_eq!(lock["bundles"][0]["name"], "acme/starter");
    assert_eq!(lock["bundles"][1]["resolved_version"], "1.1.0");
    fixture.skm(&["check"]);
}

#[test]
fn skills_only_outdated_ignores_invalid_bundle_lock() {
    let fixture = Fixture::new();
    fs::write(fixture.project.join("skills.yaml"), format!("name: fixture\nregistries:\n  local: {}\nagents: [codex]\nskills:\n  - {{name: acme/alpha, version: 1.0.0, source: local}}\n", fixture.registry.display())).unwrap();
    fs::write(fixture.project.join("skills.lock.yaml"), "invalid: [\n").unwrap();
    let output = fixture.skm(&["skill", "outdated", "acme/alpha"]);
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("acme/alpha"));
}

#[test]
fn retained_bundle_links_are_owned_for_reinstall_upgrade_and_removal() {
    for operation in ["install", "upgrade", "remove"] {
        let fixture = Fixture::new();
        let canonical = fixture.home.join(".skm/cache/registries/local");
        let legacy = fixture.home.join(".cache/skm/registries/local");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::rename(&canonical, &legacy).unwrap();
        fixture.skm(&["install", "--yes"]);
        let alpha = fixture.project.join(".agents/skills/alpha");
        assert!(fs::read_link(&alpha).unwrap().starts_with(&legacy));
        fixture.skill("beta", "1.0.0", None);
        fixture.manifest(
            "alpha: 1.0.0, helper: 1.0.0, beta: 1.0.0",
            "starter: {packages: [beta]}, other: {packages: [helper]}",
        );
        fixture.commit();
        fixture.skm(&["cache", "refresh", "local"]);
        assert!(legacy.exists());
        match operation {
            "install" => {
                fixture.skm(&["install", "--yes"]);
            }
            "upgrade" => {
                fixture.skm(&["skill", "upgrade", "acme/starter", "--yes"]);
            }
            "remove" => {
                fixture.skm(&["remove", "acme/starter", "--yes"]);
            }
            _ => unreachable!(),
        }
        assert!(!alpha.exists());
        assert!(!fixture.project.join(".agents/skills/helper").exists());
        if operation != "remove" {
            assert!(fixture.project.join(".agents/skills/beta").is_symlink());
        }
        fixture.skm(&["check"]);
        assert!(legacy.exists());
    }
}

#[test]
fn conflicting_retained_bundle_origin_blocks_removal_without_writes() {
    let fixture = Fixture::new();
    let canonical = fixture.home.join(".skm/cache/registries/local");
    let legacy = fixture.home.join(".cache/skm/registries/local");
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::rename(&canonical, &legacy).unwrap();
    fixture.skm(&["install", "--yes"]);
    fixture.skm(&["cache", "refresh", "local"]);
    git(&legacy, &["remote", "set-url", "origin", "/wrong/origin"]);
    let manifest = fs::read(fixture.project.join("skills.yaml")).unwrap();
    let lock = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
    let link = fixture.project.join(".agents/skills/alpha");
    let target = fs::read_link(&link).unwrap();
    let output = fixture.command(&["remove", "acme/starter", "--yes"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("different or missing origin"));
    assert_eq!(
        fs::read(fixture.project.join("skills.yaml")).unwrap(),
        manifest
    );
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        lock
    );
    assert_eq!(fs::read_link(link).unwrap(), target);
}

#[test]
fn retained_cache_symlink_escape_is_not_accepted_as_owned_bundle_link() {
    let fixture = Fixture::new();
    let canonical = fixture.home.join(".skm/cache/registries/local");
    let legacy = fixture.home.join(".cache/skm/registries/local");
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::rename(&canonical, &legacy).unwrap();
    fixture.skm(&["install", "--yes"]);
    fixture.skm(&["cache", "refresh", "local"]);
    let old_version = legacy.join("skills/acme/alpha/v1.0.0");
    fs::remove_dir_all(&old_version).unwrap();
    let external = fixture.root.path().join("external");
    fs::create_dir(&external).unwrap();
    fs::write(external.join("SKILL.md"), "# external\n").unwrap();
    symlink(&external, &old_version).unwrap();
    let manifest = fs::read(fixture.project.join("skills.yaml")).unwrap();
    let lock = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
    let link = fixture.project.join(".agents/skills/alpha");
    let output = fixture.command(&["remove", "acme/starter", "--yes"]);
    assert!(!output.status.success());
    assert_eq!(
        fs::read(fixture.project.join("skills.yaml")).unwrap(),
        manifest
    );
    assert_eq!(
        fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
        lock
    );
    assert!(link.is_symlink());
    assert!(external.join("SKILL.md").exists());
}

#[test]
fn outdated_refresh_preserves_project_relative_bundle_registry() {
    let fixture = Fixture::new();
    fs::write(fixture.registry.join("skills/acme/manifest.yaml"), "schema_version: 2\nnamespace: acme\npackages: {alpha: 1.0.0, helper: 1.0.0}\nbundles: {starter: {packages: [alpha]}, other: {packages: [helper]}}\nbundle_versions:\n  starter:\n    1.0.0: {alpha: 1.0.0}\n    2.0.0: {alpha: 1.0.0}\n").unwrap();
    fixture.commit();
    fs::remove_dir_all(fixture.home.join(".skm/cache/registries/local")).unwrap();
    let manifest = "name: fixture\nregistries: {local: ../registry}\nagents: [codex]\nskills:\n  - {name: acme/starter, kind: bundle, version: 1.0.0, source: local}\n";
    fs::write(fixture.project.join("skills.yaml"), manifest).unwrap();
    let result = fixture.skm(&["skill", "outdated", "acme/starter", "--refresh"]);
    assert!(String::from_utf8_lossy(&result.stdout).contains("1.0.0 -> 2.0.0"));
    assert_eq!(
        fs::read_to_string(fixture.project.join("skills.yaml")).unwrap(),
        manifest
    );
    assert!(!fixture.project.join("skills.lock.yaml").exists());
}
