#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::{Command, Output};

fn command(home: &Path, project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_skm"))
        .current_dir(project)
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("xdg-config"))
        .env("XDG_CACHE_HOME", home.join("xdg-cache"))
        .env("SKM_NO_UPDATE_CHECK", "1")
        .output()
        .unwrap()
}

#[test]
fn imports_settings_and_preserves_opaque_fields_after_registry_edits() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let legacy = home.join("xdg-config/skm/config.yaml");
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    let contents = "default_registry: default\nregistries:\n  default: /original\ncustom:\n  nested: retained\n";
    fs::write(&legacy, contents).unwrap();
    let output = command(
        home,
        home,
        &["registry", "add", "extra", "/extra", "--skip-validate"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&legacy).unwrap(), contents);
    let config: serde_yaml::Value =
        serde_yaml::from_slice(&fs::read(home.join(".skm/config.yaml")).unwrap()).unwrap();
    assert_eq!(config["custom"]["nested"].as_str(), Some("retained"));
    assert_eq!(config["registries"]["extra"].as_str(), Some("/extra"));
    assert_eq!(
        fs::read_to_string(home.join(".skm/skills.yaml")).unwrap(),
        "name: Global skills\nagents: []\nskills: []\n"
    );
    assert!(home.join(".skm/cache").is_dir());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Initializing"));
}

#[test]
fn preview_help_identity_and_invalid_arguments_do_not_bootstrap() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    fs::write(
        home.join("skills.yaml"),
        "name: empty\nagents: []\nskills: []\n",
    )
    .unwrap();
    for args in [
        &["install", "--dry-run"][..],
        &["install", "--json"],
        &["config", "set", "name", "x", "--dry-run"],
        &["--help"],
        &["--version"],
        &["self", "version"],
        &["invalid-command"],
    ] {
        let output = command(home, home, args);
        if args == ["install", "--json"] {
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();
        }
        assert!(!home.join(".skm").exists(), "{args:?}");
    }
}

#[test]
fn invalid_and_unsafe_state_fails_before_command() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let outside = home.join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, home.join(".skm")).unwrap();
    let output = command(home, home, &["config", "show", "--global"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Expected a real directory"));
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    fs::remove_file(home.join(".skm")).unwrap();
    fs::create_dir_all(home.join("xdg-config/skm")).unwrap();
    fs::write(
        home.join("xdg-config/skm/config.yaml"),
        "registries: [invalid",
    )
    .unwrap();
    let output = command(home, home, &["config", "show", "--global"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid settings"));
    assert!(!home.join(".skm/config.yaml").exists());
}

#[test]
fn simultaneous_startup_preserves_complete_files() {
    let home = tempfile::tempdir().unwrap();
    let mut children = Vec::new();
    for _ in 0..8 {
        children.push(
            Command::new(env!("CARGO_BIN_EXE_skm"))
                .current_dir(home.path())
                .args(["config", "show", "--global"])
                .env("HOME", home.path())
                .env("XDG_CONFIG_HOME", home.path().join("xdg-config"))
                .env("SKM_NO_UPDATE_CHECK", "1")
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
    }
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    let config = fs::read(home.path().join(".skm/config.yaml")).unwrap();
    let manifest = fs::read(home.path().join(".skm/skills.yaml")).unwrap();
    serde_yaml::from_slice::<serde_yaml::Value>(&config).unwrap();
    serde_yaml::from_slice::<serde_yaml::Value>(&manifest).unwrap();
    assert!(
        command(home.path(), home.path(), &["config", "show", "--global"])
            .status
            .success()
    );
    assert_eq!(
        config,
        fs::read(home.path().join(".skm/config.yaml")).unwrap()
    );
    assert_eq!(
        manifest,
        fs::read(home.path().join(".skm/skills.yaml")).unwrap()
    );
}

#[test]
fn invalid_base_edits_are_rejected_and_reset_recovers_malformed_settings() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert!(command(home, home, &["config", "show", "--global"])
        .status
        .success());
    let path = home.join(".skm/config.yaml");
    let original = fs::read(&path).unwrap();
    for (key, value) in [
        ("check_for_updates", "weird"),
        ("default_registry", "123"),
        ("registries", "[]"),
        ("registries.default", "false"),
    ] {
        let result = command(
            home,
            home,
            &["config", "set", key, value, "--global", "--yes"],
        );
        assert!(!result.status.success(), "{key}");
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    assert!(command(home, home, &["config", "show", "--global"])
        .status
        .success());
    fs::write(&path, "registries: [broken").unwrap();
    assert!(!command(home, home, &["config", "show", "--global"])
        .status
        .success());
    assert!(
        command(home, home, &["config", "reset", "--global", "--yes"])
            .status
            .success()
    );
    assert!(command(home, home, &["config", "show", "--global"])
        .status
        .success());
}

#[test]
fn reset_keeps_defaults_instead_of_reimporting_legacy_settings() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let legacy = home.join("xdg-config/skm/config.yaml");
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    let custom = "default_registry: custom\nregistries:\n  custom: /custom\n";
    fs::write(&legacy, custom).unwrap();
    assert!(command(home, home, &["config", "show", "--global"])
        .status
        .success());
    let output = command(
        home,
        home,
        &["clean", "reset", "--config", "--backup", "--yes"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&legacy).unwrap(), custom);
    let config = home.join(".skm/config.yaml");
    let defaults = fs::read(&config).unwrap();
    let value: serde_yaml::Value = serde_yaml::from_slice(&defaults).unwrap();
    assert_eq!(value["default_registry"].as_str(), Some("default"));
    assert!(command(home, home, &["config", "show", "--global"])
        .status
        .success());
    assert_eq!(fs::read(&config).unwrap(), defaults);
    assert_eq!(fs::read_dir(home.join(".skm-backups")).unwrap().count(), 1);
}

#[test]
fn reset_preview_and_unsafe_recovery_do_not_write() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let output = command(
        home,
        home,
        &["clean", "reset", "--config", "--backup", "--dry-run"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!home.join(".skm-backups").exists());
    assert!(!home.join(".skm").exists());
    let outside = home.join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, home.join(".skm")).unwrap();
    for args in [
        &["config", "reset", "--global", "--yes"][..],
        &["clean", "reset", "--config", "--backup", "--yes"],
    ] {
        assert!(!command(home, home, args).status.success());
    }
    assert!(!home.join(".skm-backups").exists());
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
}

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn visit(root: &Path, dir: &Path, entries: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.file_type().is_symlink() {
                entries.push((
                    name,
                    fs::read_link(&path)
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else if metadata.is_dir() {
                entries.push((name, Vec::new()));
                visit(root, &path, entries);
            } else {
                entries.push((name, fs::read(path).unwrap()));
            }
        }
    }
    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort();
    entries
}

#[test]
fn global_manifest_is_shared_across_projects_and_previews_do_not_write() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let first = root.path().join("first");
    let second = root.path().join("second");
    for dir in [&home, &first, &second] {
        fs::create_dir(dir).unwrap();
    }
    let project =
        "name: project\nagents: [codex]\nskills: []\nregistries: {default: project-registry}\n";
    fs::write(first.join("skills.yaml"), project).unwrap();
    fs::write(second.join("skills.yaml"), project).unwrap();
    success(command(
        &home,
        &first,
        &["init", "--global", "--non-interactive", "--name", "User"],
    ));
    let manifest = home.join(".skm/skills.yaml");
    let initialized = fs::read(&manifest).unwrap();
    assert!(!command(
        &home,
        &second,
        &[
            "init",
            "--global",
            "--non-interactive",
            "--name",
            "Overwrite"
        ]
    )
    .status
    .success());
    assert_eq!(fs::read(&manifest).unwrap(), initialized);
    let local = first.join("local");
    fs::create_dir(&local).unwrap();
    fs::write(
        local.join("SKILL.md"),
        "---\nname: helper\ndescription: test\n---\n",
    )
    .unwrap();
    fs::write(&manifest, "name: User\nagents: [codex]\nskills: []\nregistries: {default: global-registry}\nextension: retained\n").unwrap();
    success(command(
        &home,
        &first,
        &["add", "helper", "--path", "local", "--global", "--yes"],
    ));
    let value: serde_yaml::Value = serde_yaml::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(value["skills"][0]["path"].as_str(), local.to_str());
    assert_eq!(value["extension"].as_str(), Some("retained"));
    let listed = success(command(&home, &second, &["list", "--global"]));
    assert!(String::from_utf8_lossy(&listed.stdout).contains("helper"));
    let project_list = success(command(&home, &second, &["list"]));
    assert!(!String::from_utf8_lossy(&project_list.stdout).contains("helper"));
    let before = snapshot(root.path());
    for args in [
        &["install", "--global", "--dry-run"][..],
        &["install", "--global", "--json"],
        &["add", "another", "--path", "local", "--global", "--json"],
        &["remove", "helper", "--global", "--dry-run", "--yes"],
        &["skill", "outdated", "--global"],
    ] {
        let result = success(command(&home, &second, args));
        if args.contains(&"--json") {
            serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap();
        }
        assert_eq!(snapshot(root.path()), before, "{args:?}");
    }
    assert_eq!(
        fs::read_to_string(first.join("skills.yaml")).unwrap(),
        project
    );
    assert_eq!(
        fs::read_to_string(second.join("skills.yaml")).unwrap(),
        project
    );
}

#[test]
fn global_add_uses_user_registry_override_and_inherited_base_from_either_directory() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let first = root.path().join("one");
    let second = root.path().join("two");
    for dir in [&home, &first, &second] {
        fs::create_dir(dir).unwrap();
    }
    success(command(&home, &first, &["config", "show", "--global"]));
    for (base, name) in [
        (&home.join(".skm/global-registry"), "global-only"),
        (&first.join("project-registry"), "project-only"),
        (&home.join(".skm/base-registry"), "base-only"),
    ] {
        let skill = base.join("skills/team").join(name).join("v1.0.0");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: test\n---\n"),
        )
        .unwrap();
    }
    fs::write(home.join(".skm/config.yaml"), "default_registry: default\nregistries: {default: base-registry, inherited: base-registry}\n").unwrap();
    let global = home.join(".skm/skills.yaml");
    fs::write(
        &global,
        "name: User\nagents: [codex]\nskills: []\nregistries: {default: global-registry}\n",
    )
    .unwrap();
    let project = "name: Project\nagents: [codex]\nskills: []\nregistries: {default: project-registry, inherited: project-registry}\n";
    fs::write(first.join("skills.yaml"), project).unwrap();
    fs::write(second.join("skills.yaml"), project).unwrap();
    for dir in [&first, &second] {
        let before = snapshot(root.path());
        let preview = success(command(
            &home,
            dir,
            &["add", "team/global-only", "--global", "--json"],
        ));
        serde_json::from_slice::<serde_json::Value>(&preview.stdout).unwrap();
        assert_eq!(snapshot(root.path()), before);
        assert!(!command(
            &home,
            dir,
            &["add", "team/project-only", "--global", "--json"]
        )
        .status
        .success());
        success(command(
            &home,
            dir,
            &[
                "add",
                "team/base-only",
                "--registry",
                "inherited",
                "--global",
                "--json",
            ],
        ));
        assert_eq!(snapshot(root.path()), before);
    }
    fs::write(&global, "name: User\nagents: [codex]\nskills: []\nregistries: {default: 'https://example.invalid/no-fetch.git'}\n").unwrap();
    let before = snapshot(root.path());
    assert!(!command(
        &home,
        &first,
        &["add", "team/missing", "--global", "--json"]
    )
    .status
    .success());
    assert_eq!(snapshot(root.path()), before);
}

#[test]
fn global_versions_and_inspection_use_user_registry_not_project_registry() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let project = root.path().join("project");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&project).unwrap();
    success(command(&home, &project, &["config", "show", "--global"]));
    let global_registry = home.join(".skm/registry");
    for version in ["v1.0.0", "v2.0.0"] {
        let path = global_registry.join("skills/team/user").join(version);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("SKILL.md"),
            "---\nname: user\ndescription: test\n---\n",
        )
        .unwrap();
    }
    let cache = home.join(".skm/cache/registries/default");
    fs::create_dir_all(cache.parent().unwrap()).unwrap();
    symlink(&global_registry, &cache).unwrap();
    fs::write(home.join(".skm/skills.yaml"), "name: User\nagents: [codex]\nregistries: {default: registry}\nskills:\n  - {name: team/user, version: v1.0.0, source: default}\n").unwrap();
    fs::write(
        project.join("skills.yaml"),
        "name: Project\nagents: [codex]\nregistries: {default: wrong}\nskills: []\n",
    )
    .unwrap();
    let before = snapshot(root.path());
    let output = success(command(&home, &project, &["skill", "outdated", "--global"]));
    assert!(String::from_utf8_lossy(&output.stdout).contains("team/user: v1.0.0 -> v2.0.0"));
    success(command(
        &home,
        &project,
        &["skill", "use", "team/user@v2.0.0", "--global", "--dry-run"],
    ));
    success(command(
        &home,
        &project,
        &["skill", "upgrade", "team/user", "--global", "--dry-run"],
    ));
    let refresh = command(
        &home,
        &project,
        &["skill", "outdated", "--global", "--refresh"],
    );
    assert!(!refresh.status.success());
    assert!(String::from_utf8_lossy(&refresh.stderr).contains("from ~/.skm"));
    assert_eq!(snapshot(root.path()), before);
}
