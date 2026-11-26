#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path;
use std::process::{Command, Output, Stdio};

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

fn command_with_input(home: &Path, project: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_skm"))
        .current_dir(project)
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("xdg-config"))
        .env("XDG_CACHE_HOME", home.join("xdg-cache"))
        .env("SKM_NO_UPDATE_CHECK", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
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
    assert!(home.join(".skm/cache/registries").is_dir());
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
    assert!(home.path().join(".skm/cache/registries").is_dir());
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
fn bootstrap_repairs_missing_registry_directory_without_replacing_files() {
    let home = tempfile::tempdir().unwrap();
    let skm = home.path().join(".skm");
    fs::create_dir_all(skm.join("cache")).unwrap();
    let config = "default_registry: custom\nregistries:\n  custom: /custom\n";
    let manifest = "name: Personal\nagents: [codex]\nskills: []\n";
    fs::write(skm.join("config.yaml"), config).unwrap();
    fs::write(skm.join("skills.yaml"), manifest).unwrap();
    fs::write(skm.join("cache/retained"), "keep").unwrap();

    let before = snapshot(home.path());
    for args in [
        &["install", "--global", "--dry-run"][..],
        &["install", "--global", "--json"],
    ] {
        let preview = command(home.path(), home.path(), args);
        assert!(
            preview.status.success(),
            "{}",
            String::from_utf8_lossy(&preview.stderr)
        );
        assert_eq!(snapshot(home.path()), before);
    }

    let output = command(home.path(), home.path(), &["config", "show", "--global"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert!(skm.join("cache/registries").is_dir());
    assert_eq!(fs::read_to_string(skm.join("config.yaml")).unwrap(), config);
    assert_eq!(
        fs::read_to_string(skm.join("skills.yaml")).unwrap(),
        manifest
    );
    assert_eq!(
        fs::read_to_string(skm.join("cache/retained")).unwrap(),
        "keep"
    );
}

#[test]
fn bootstrap_rejects_unsafe_registry_directory_before_writing_files() {
    for symlinked in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let skm = home.path().join(".skm");
        let registries = skm.join("cache/registries");
        fs::create_dir_all(registries.parent().unwrap()).unwrap();
        let outside = home.path().join("outside");
        fs::create_dir(&outside).unwrap();
        if symlinked {
            symlink(&outside, &registries).unwrap();
        } else {
            fs::write(&registries, "keep").unwrap();
        }

        let output = command(home.path(), home.path(), &["config", "show", "--global"]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("Expected a real directory"));
        assert!(!skm.join("config.yaml").exists());
        assert!(!skm.join("skills.yaml").exists());
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
        if symlinked {
            assert!(fs::symlink_metadata(&registries)
                .unwrap()
                .file_type()
                .is_symlink());
        } else {
            assert_eq!(fs::read_to_string(&registries).unwrap(), "keep");
        }
    }
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
fn declined_settings_resets_do_not_bootstrap_or_write() {
    for args in [
        &["config", "reset", "--global"][..],
        &["config", "reset"],
        &["clean", "reset", "--config"],
    ] {
        for malformed in [false, true] {
            let home = tempfile::tempdir().unwrap();
            let project = tempfile::tempdir().unwrap();
            if malformed {
                fs::create_dir(home.path().join(".skm")).unwrap();
                fs::write(home.path().join(".skm/config.yaml"), "registries: [broken").unwrap();
            }
            let home_before = snapshot(home.path());
            let project_before = snapshot(project.path());
            success(command_with_input(
                home.path(),
                project.path(),
                args,
                "n\nn\n",
            ));
            assert_eq!(snapshot(home.path()), home_before, "{args:?}");
            assert_eq!(snapshot(project.path()), project_before, "{args:?}");
        }
    }
}

#[test]
fn confirmed_interactive_resets_bootstrap_only_when_global_settings_are_reset() {
    for args in [
        &["config", "reset", "--global"][..],
        &["clean", "reset", "--config"],
    ] {
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        success(command_with_input(home.path(), project.path(), args, "y\n"));
        assert!(home.path().join(".skm/skills.yaml").is_file());
        assert!(home.path().join(".skm/cache/registries").is_dir());
    }
    for (input, global_reset) in [("y\nn\n", true), ("n\ny\n", false)] {
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        success(command_with_input(
            home.path(),
            project.path(),
            &["config", "reset"],
            input,
        ));
        assert_eq!(home.path().join(".skm").exists(), global_reset);
        assert_eq!(project.path().join("skills.yaml").exists(), !global_reset);
    }
}

#[test]
fn settings_reset_preflights_unrelated_global_paths_before_reset_writes() {
    for args in [
        &["config", "reset", "--global", "--yes"][..],
        &["clean", "reset", "--config", "--yes"],
    ] {
        for unsafe_cache in [false, true] {
            let home = tempfile::tempdir().unwrap();
            let project = tempfile::tempdir().unwrap();
            let skm = home.path().join(".skm");
            fs::create_dir(&skm).unwrap();
            fs::write(skm.join("config.yaml"), "registries: [broken").unwrap();
            fs::write(project.path().join("skills.yaml"), "project bytes").unwrap();
            if unsafe_cache {
                fs::create_dir(home.path().join("outside")).unwrap();
                symlink(home.path().join("outside"), skm.join("cache")).unwrap();
            } else {
                fs::write(skm.join("skills.yaml"), "skills: [broken").unwrap();
            }
            let home_before = snapshot(home.path());
            let project_before = snapshot(project.path());
            assert!(!command(home.path(), project.path(), args).status.success());
            assert_eq!(snapshot(home.path()), home_before, "{args:?}");
            assert_eq!(snapshot(project.path()), project_before, "{args:?}");
        }
    }
}

#[test]
fn unwritable_cache_blocks_reset_before_settings_and_project_changes() {
    for args in [
        &["config", "reset", "--global", "--yes"][..],
        &["clean", "reset", "--config", "--yes"],
    ] {
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let skm = home.path().join(".skm");
        let cache = skm.join("cache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(skm.join("config.yaml"), "registries: [broken").unwrap();
        fs::write(project.path().join("skills.yaml"), "project bytes").unwrap();
        fs::set_permissions(&cache, fs::Permissions::from_mode(0o500)).unwrap();
        // Privileged processes can bypass mode bits, so this fixture cannot
        // establish the intended permission failure in that environment.
        let probe = cache.join("write-probe");
        if fs::create_dir(&probe).is_ok() {
            fs::remove_dir(probe).unwrap();
            fs::set_permissions(&cache, fs::Permissions::from_mode(0o700)).unwrap();
            continue;
        }
        let result = command(home.path(), project.path(), args);
        fs::set_permissions(&cache, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(!result.status.success(), "{args:?}");
        assert_eq!(
            fs::read(skm.join("config.yaml")).unwrap(),
            b"registries: [broken"
        );
        assert_eq!(
            fs::read(project.path().join("skills.yaml")).unwrap(),
            b"project bytes"
        );
        assert!(!skm.join("skills.yaml").exists());
        assert!(!cache.join("registries").exists());
    }
}

#[test]
fn unsafe_reset_with_backup_preserves_settings_and_project() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let skm = home.path().join(".skm");
    fs::create_dir(&skm).unwrap();
    fs::write(skm.join("config.yaml"), "registries: [broken").unwrap();
    fs::write(skm.join("skills.yaml"), "skills: [broken").unwrap();
    fs::write(project.path().join("skills.yaml"), "project bytes").unwrap();
    let skm_before = snapshot(&skm);
    let project_before = snapshot(project.path());
    assert!(!command(
        home.path(),
        project.path(),
        &["clean", "reset", "--config", "--backup", "--yes"]
    )
    .status
    .success());
    assert_eq!(snapshot(&skm), skm_before);
    assert_eq!(snapshot(project.path()), project_before);
    assert_eq!(
        fs::read_dir(home.path().join(".skm-backups"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn settings_resets_bootstrap_fresh_home_and_preserve_repeat_bytes() {
    for args in [
        &["config", "reset", "--global", "--yes"][..],
        &["clean", "reset", "--config", "--yes"],
    ] {
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        fs::write(project.path().join("untouched"), "project data").unwrap();
        success(command(home.path(), project.path(), args));
        let skm = home.path().join(".skm");
        let settings = fs::read(skm.join("config.yaml")).unwrap();
        let manifest = fs::read(skm.join("skills.yaml")).unwrap();
        serde_yaml::from_slice::<serde_yaml::Value>(&settings).unwrap();
        assert_eq!(manifest, b"name: Global skills\nagents: []\nskills: []\n");
        assert!(skm.join("cache/registries").is_dir());
        let project_before = snapshot(project.path());
        fs::write(skm.join("cache/registries/retained"), "cache data").unwrap();

        success(command(home.path(), project.path(), args));
        assert_eq!(fs::read(skm.join("config.yaml")).unwrap(), settings);
        assert_eq!(fs::read(skm.join("skills.yaml")).unwrap(), manifest);
        assert_eq!(
            fs::read(skm.join("cache/registries/retained")).unwrap(),
            b"cache data"
        );
        assert_eq!(snapshot(project.path()), project_before);
    }
}

#[test]
fn settings_resets_recover_malformed_settings_and_bootstrap_missing_layout() {
    for args in [
        &["config", "reset", "--global", "--yes"][..],
        &["clean", "reset", "--config", "--yes"],
    ] {
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let skm = home.path().join(".skm");
        fs::create_dir(&skm).unwrap();
        fs::write(skm.join("config.yaml"), "registries: [broken").unwrap();
        success(command(home.path(), project.path(), args));
        let settings = fs::read(skm.join("config.yaml")).unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_slice(&settings).unwrap();
        assert_eq!(parsed["default_registry"].as_str(), Some("default"));
        assert_eq!(
            fs::read(skm.join("skills.yaml")).unwrap(),
            b"name: Global skills\nagents: []\nskills: []\n"
        );
        assert!(skm.join("cache/registries").is_dir());
        assert_eq!(fs::read_dir(project.path()).unwrap().count(), 0);
    }
}

#[test]
fn settings_reset_previews_and_cache_reset_keep_their_side_effects() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let before = snapshot(home.path());
    for args in [
        &["config", "reset", "--global", "--dry-run"][..],
        &["clean", "reset", "--config", "--dry-run"],
    ] {
        success(command(home.path(), project.path(), args));
        assert_eq!(snapshot(home.path()), before);
        assert_eq!(fs::read_dir(project.path()).unwrap().count(), 0);
    }
    success(command(
        home.path(),
        project.path(),
        &["config", "show", "--global"],
    ));
    success(command(
        home.path(),
        project.path(),
        &["clean", "reset", "--config", "--cache", "--yes"],
    ));
    assert!(!home.path().join(".skm/cache").exists());
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
    let refresh = success(command(
        &home,
        &project,
        &["skill", "outdated", "--global", "--refresh"],
    ));
    assert!(String::from_utf8_lossy(&refresh.stdout).contains("team/user: v1.0.0 -> v2.0.0"));
    assert_eq!(snapshot(root.path()), before);
    let manifest = home.join(".skm/skills.yaml");
    fs::write(&manifest, "name: User\nagents: [codex]\nregistries: {default: registry}\nskills:\n  - {name: team/user, version: v1.0.0, source: default, publisher: {custom: retained}}\nextension: retained\n").unwrap();
    success(command(
        &home,
        &project,
        &["skill", "use", "team/user@v2.0.0", "--global", "--yes"],
    ));
    let value: serde_yaml::Value = serde_yaml::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(value["skills"][0]["version"].as_str(), Some("v2.0.0"));
    assert_eq!(
        value["skills"][0]["publisher"]["custom"].as_str(),
        Some("retained")
    );
    assert_eq!(value["extension"].as_str(), Some("retained"));
    assert_eq!(
        fs::canonicalize(home.join(".agents/skills/user")).unwrap(),
        fs::canonicalize(global_registry.join("skills/team/user/v2.0.0")).unwrap()
    );
}

#[test]
fn global_previews_reject_symlinked_home_state_without_reading_or_writing_it() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let project = root.path().join("project");
    let external = root.path().join("external");
    for dir in [&home, &project, &external] {
        fs::create_dir(dir).unwrap();
    }
    fs::write(
        external.join("skills.yaml"),
        "name: External\nagents: []\nskills: []\n",
    )
    .unwrap();
    symlink(&external, home.join(".skm")).unwrap();
    let before = snapshot(root.path());
    for args in [
        &["install", "--global", "--json"][..],
        &["add", "local", "--path", ".", "--global", "--json"],
    ] {
        let output = command(&home, &project, args);
        assert!(!output.status.success(), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("Expected a real directory"));
        assert_eq!(snapshot(root.path()), before, "{args:?}");
    }
}

#[test]
fn global_add_rejects_project_only_manifest_before_discovery_or_write() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let project = root.path().join("project");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&project).unwrap();
    success(command(&home, &project, &["config", "show", "--global"]));
    fs::write(home.join(".skm/skills.yaml"), "name: User\nagents: [codex]\nskills: []\ntoolkit: {manifest: toolkit.yaml, version: '1.0.0'}\n").unwrap();
    let before = snapshot(root.path());
    for args in [
        &["add", "local", "--path", ".", "--global", "--json"][..],
        &["add", "local", "--path", ".", "--global", "--yes"],
        &["add", "team/remote", "--global", "--json"],
        &["add", "team/remote", "--global", "--yes"],
    ] {
        let output = command(&home, &project, args);
        assert!(!output.status.success(), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("project-only"));
        assert_eq!(snapshot(root.path()), before, "{args:?}");
    }
}

#[test]
fn global_manifest_guidance_names_the_selected_scope() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let project = root.path().join("project");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&project).unwrap();
    for args in [
        &["install", "--global", "--json"][..],
        &["add", "local", "--path", ".", "--global", "--json"],
    ] {
        let output = command(&home, &project, args);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("skm init --global"));
        assert!(!home.join(".skm").exists());
    }
    success(command(
        &home,
        &project,
        &["init", "--global", "--non-interactive"],
    ));
    let output = success(command(
        &home,
        &project,
        &["add", "local", "--path", ".", "--global", "--yes"],
    ));
    assert!(String::from_utf8_lossy(&output.stderr).contains("skm install --global"));
}

#[test]
fn global_install_preview_rejects_symlinked_cache_ancestors() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let project = root.path().join("project");
    let external = root.path().join("external");
    for dir in [&home, &project, &external] {
        fs::create_dir(dir).unwrap();
    }
    let skm = home.join(".skm");
    fs::create_dir(&skm).unwrap();
    let registry = external.join("registries/default");
    fs::create_dir_all(registry.join("skills/team/skill/v1.0.0")).unwrap();
    fs::write(
        registry.join("skills/team/skill/v1.0.0/SKILL.md"),
        "---\nname: skill\ndescription: test\n---\n",
    )
    .unwrap();
    fs::write(
        skm.join("config.yaml"),
        format!(
            "default_registry: default\nregistries:\n  default: {}\n",
            registry.display()
        ),
    )
    .unwrap();
    fs::write(skm.join("skills.yaml"), "name: User\nagents: [codex]\nskills:\n  - {name: team/skill, version: v1.0.0, source: default}\n").unwrap();
    symlink(&external, skm.join("cache")).unwrap();
    let before = snapshot(root.path());
    let output = command(&home, &project, &["install", "--global", "--json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Expected a real directory"));
    assert_eq!(snapshot(root.path()), before);
    fs::remove_file(skm.join("cache")).unwrap();
    fs::create_dir(skm.join("cache")).unwrap();
    symlink(external.join("registries"), skm.join("cache/registries")).unwrap();
    let before = snapshot(root.path());
    let output = command(&home, &project, &["install", "--global", "--json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Expected a real directory"));
    assert_eq!(snapshot(root.path()), before);
}

#[test]
fn global_outdated_refresh_resolves_relative_git_from_user_manifest() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let first = root.path().join("first");
    let second = root.path().join("second");
    for dir in [&home, &first, &second] {
        fs::create_dir(dir).unwrap();
    }
    success(command(&home, &first, &["config", "show", "--global"]));
    let source = home.join(".skm/origin");
    fs::create_dir(&source).unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&source)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "--initial-branch=main"]);
    git(&["config", "user.name", "SKM Test"]);
    git(&["config", "user.email", "skm-test@example.invalid"]);
    for version in ["v1.0.0", "v2.0.0"] {
        let path = source.join("skills/team/user").join(version);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("SKILL.md"),
            format!(
                "---\nname: user\nmetadata:\n  skm-version: \"{}\"\n---\n",
                version.trim_start_matches('v')
            ),
        )
        .unwrap();
    }
    git(&["add", "."]);
    git(&["commit", "-m", "first"]);
    let manifest = "name: User\nagents: [codex]\nregistries: {default: origin}\nskills:\n  - {name: team/user, version: v1.0.0, source: default}\n";
    fs::write(home.join(".skm/skills.yaml"), manifest).unwrap();
    let project_manifest = "name: Project\nagents: []\nregistries: {default: wrong}\nskills: []\n";
    fs::write(first.join("skills.yaml"), project_manifest).unwrap();
    fs::create_dir(first.join("origin")).unwrap();
    let first_before = snapshot(&first);
    let second_before = snapshot(&second);
    let args = ["skill", "outdated", "team/user", "--global", "--refresh"];
    let output = success(command(&home, &first, &args));
    assert!(String::from_utf8_lossy(&output.stdout).contains("team/user: v1.0.0 -> v2.0.0"));
    let cache = home.join(".skm/cache/registries/default");
    assert!(cache.join("skills/team/user/v2.0.0").is_dir());
    let path = source.join("skills/team/user/v3.0.0");
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("SKILL.md"),
        "---\nname: user\nmetadata:\n  skm-version: \"3.0.0\"\n---\n",
    )
    .unwrap();
    git(&["add", "."]);
    git(&["commit", "-m", "second"]);
    let output = success(command(&home, &second, &args));
    assert!(String::from_utf8_lossy(&output.stdout).contains("team/user: v1.0.0 -> v3.0.0"));
    assert_eq!(
        fs::read_to_string(home.join(".skm/skills.yaml")).unwrap(),
        manifest
    );
    assert_eq!(snapshot(&first), first_before);
    assert_eq!(snapshot(&second), second_before);

    // A conflicting selected URL cannot repoint or refresh an existing cache.
    fs::write(
        home.join(".skm/skills.yaml"),
        manifest.replace("default: origin", "default: other"),
    )
    .unwrap();
    let old_head = Command::new("git")
        .current_dir(&cache)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap()
        .stdout;
    let output = command(&home, &first, &args);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("different or missing origin"));
    let head = Command::new("git")
        .current_dir(&cache)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap()
        .stdout;
    assert_eq!(head, old_head);
    assert_eq!(snapshot(&first), first_before);
    assert_eq!(snapshot(&second), second_before);
}

#[test]
fn global_outdated_refresh_populates_relative_directory_from_inherited_registry() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let project = root.path().join("project");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&project).unwrap();
    success(command(&home, &project, &["config", "show", "--global"]));
    let source = home.join(".skm/local-registry");
    fs::create_dir_all(source.join("skills/team/user/v1.0.0")).unwrap();
    fs::create_dir_all(source.join("skills/team/user/v2.0.0")).unwrap();
    fs::write(
        home.join(".skm/config.yaml"),
        "default_registry: local\nregistries: {local: local-registry}\n",
    )
    .unwrap();
    let manifest =
        "name: User\nagents: []\nskills:\n  - {name: team/user, version: v1.0.0, source: local}\n";
    fs::write(home.join(".skm/skills.yaml"), manifest).unwrap();
    fs::write(
        project.join("skills.yaml"),
        "name: Project\nagents: []\nskills: []\n",
    )
    .unwrap();
    let before = snapshot(&project);
    let result = success(command(
        &home,
        &project,
        &["skill", "outdated", "--global", "--refresh"],
    ));
    assert!(String::from_utf8_lossy(&result.stdout).contains("team/user: v1.0.0 -> v2.0.0"));
    assert_eq!(
        fs::canonicalize(home.join(".skm/cache/registries/local")).unwrap(),
        fs::canonicalize(source).unwrap()
    );
    assert_eq!(
        fs::read_to_string(home.join(".skm/skills.yaml")).unwrap(),
        manifest
    );
    assert_eq!(snapshot(&project), before);
}

#[test]
fn global_outdated_refresh_accepts_relative_git_origin_beneath_symlinked_home_ancestor() {
    let root = tempfile::tempdir().unwrap();
    let physical = root.path().join("physical");
    let logical = root.path().join("logical");
    let project = root.path().join("project");
    fs::create_dir(&physical).unwrap();
    fs::create_dir(&project).unwrap();
    symlink(&physical, &logical).unwrap();
    let home = logical.join("home");
    fs::create_dir(&home).unwrap();
    success(command(&home, &project, &["config", "show", "--global"]));
    let skm = home.join(".skm");
    let source = skm.join("origin");
    fs::create_dir(&source).unwrap();
    fs::create_dir(skm.join("nested")).unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&source)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "--initial-branch=main"]);
    git(&["config", "user.name", "SKM Test"]);
    git(&["config", "user.email", "skm-test@example.invalid"]);
    for version in ["v1.0.0", "v2.0.0"] {
        fs::create_dir_all(source.join("skills/team/user").join(version)).unwrap();
        fs::write(
            source
                .join("skills/team/user")
                .join(version)
                .join("SKILL.md"),
            "---\nname: user\ndescription: test\n---\n",
        )
        .unwrap();
    }
    git(&["add", "."]);
    git(&["commit", "-m", "first"]);
    let manifest = "name: User\nagents: [codex]\nregistries: {default: nested/../origin}\nskills:\n  - {name: team/user, version: v1.0.0, source: default}\n";
    fs::write(skm.join("skills.yaml"), manifest).unwrap();
    fs::write(
        project.join("skills.yaml"),
        "name: Project\nagents: []\nskills: []\n",
    )
    .unwrap();
    let before = snapshot(&project);
    let args = ["skill", "outdated", "--global", "--refresh"];
    for _ in 0..2 {
        let output = success(command(&home, &project, &args));
        assert!(String::from_utf8_lossy(&output.stdout).contains("team/user: v1.0.0 -> v2.0.0"));
        assert_eq!(snapshot(&project), before);
        assert_eq!(
            fs::read_to_string(skm.join("skills.yaml")).unwrap(),
            manifest
        );
    }
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
    assert_eq!(snapshot(&project), before);
    assert_eq!(
        fs::read_to_string(skm.join("skills.yaml")).unwrap(),
        manifest
    );
    let cache = skm.join("cache/registries/default");
    let origin = Command::new("git")
        .current_dir(cache)
        .args(["remote", "get-url", "origin"])
        .output()
        .unwrap();
    assert!(origin.status.success());
    assert_eq!(
        String::from_utf8(origin.stdout).unwrap(),
        format!(
            "{}/nested/../origin\n",
            fs::canonicalize(&skm).unwrap().display()
        )
    );
}

#[test]
fn global_install_targets_every_selected_agent_and_checks_exact_sources() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let home = home.path();
    fs::create_dir_all(home.join(".skm/local-skill")).unwrap();
    fs::write(
        home.join(".skm/local-skill/SKILL.md"),
        "---\nname: sample\ndescription: test\n---\n",
    )
    .unwrap();
    let manifest = b"name: Global\nagents: [claude, codex, copilot, cursor, antigravity, pi, opencode, cline, kilo, gemini-cli, goose, crush, openhands, grok, qwen, hermes]\nskills:\n  - name: sample\n    path: local-skill\n";
    fs::write(home.join(".skm/skills.yaml"), manifest).unwrap();
    let project_manifest = b"name: Unrelated\nagents: []\nskills: []\n";
    fs::write(project.path().join("skills.yaml"), project_manifest).unwrap();
    let paths = [
        ".claude/skills",
        ".agents/skills",
        ".copilot/skills",
        ".cursor/skills",
        ".gemini/config/skills",
        ".pi/agent/skills",
        ".config/opencode/skills",
        ".cline/skills",
        ".kilo/skills",
        ".gemini/skills",
        ".config/crush/skills",
        ".openhands/skills",
        ".grok/skills",
        ".qwen/skills",
        ".hermes/skills",
    ];
    let preview = command(home, project.path(), &["install", "--global", "--json"]);
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(plan["actions"].as_array().unwrap().len(), paths.len());
    assert!(!home.join(".agents").exists());
    assert!(!home.join(".skm/config.yaml").exists());
    for args in [
        &["install", "--global", "--yes"][..],
        &["check", "--global"],
    ] {
        let output = command(home, project.path(), args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for path in paths {
        assert_eq!(
            fs::canonicalize(home.join(path).join("sample")).unwrap(),
            fs::canonicalize(home.join(".skm/local-skill")).unwrap()
        );
    }
    assert_eq!(fs::read(home.join(".skm/skills.yaml")).unwrap(), manifest);
    assert_eq!(
        fs::read(project.path().join("skills.yaml")).unwrap(),
        project_manifest
    );
    assert!(!project.path().join("skills.lock.yaml").exists());
    assert!(!project.path().join(".agents").exists());
    let repeat = command(home, project.path(), &["install", "--global", "--json"]);
    let plan: serde_json::Value = serde_json::from_slice(&repeat.stdout).unwrap();
    assert!(plan["actions"].as_array().unwrap().is_empty());
    let wrong = home.join("wrong");
    fs::create_dir(&wrong).unwrap();
    let target = home.join(".agents/skills/sample");
    fs::remove_file(&target).unwrap();
    symlink(&wrong, &target).unwrap();
    let check = command(home, project.path(), &["check", "--global"]);
    assert!(!check.status.success());
    assert!(String::from_utf8_lossy(&check.stderr).contains("wrong target"));
    assert!(
        command(home, project.path(), &["install", "--global", "--yes"])
            .status
            .success()
    );
    assert!(command(home, project.path(), &["check", "--global"])
        .status
        .success());
}

#[test]
fn global_install_rejects_bad_targets_and_preflights_all_destinations() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    fs::create_dir_all(home.join(".skm/source")).unwrap();
    fs::write(
        home.join(".skm/source/SKILL.md"),
        "---\nname: sample\ndescription: test\n---\n",
    )
    .unwrap();
    let path = home.join(".skm/skills.yaml");
    for agents in ["[]", "[unknown]"] {
        fs::write(
            &path,
            format!(
                "name: Global\nagents: {agents}\nskills:\n  - name: sample\n    path: source\n"
            ),
        )
        .unwrap();
        assert!(!command(home, home, &["install", "--global", "--yes"])
            .status
            .success());
    }
    fs::write(
        &path,
        "name: Global\nagents: [codex, cursor]\nskills:\n  - name: sample\n    path: source\n",
    )
    .unwrap();
    fs::create_dir_all(home.join(".cursor/skills")).unwrap();
    fs::write(home.join(".cursor/skills/sample"), "owned").unwrap();
    assert!(!command(home, home, &["install", "--global", "--yes"])
        .status
        .success());
    assert!(!home.join(".agents").exists());
    assert_eq!(
        fs::read(home.join(".cursor/skills/sample")).unwrap(),
        b"owned"
    );
    fs::remove_file(home.join(".cursor/skills/sample")).unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::remove_dir_all(home.join(".cursor")).unwrap();
    symlink(outside.path(), home.join(".cursor")).unwrap();
    assert!(!command(home, home, &["install", "--global", "--yes"])
        .status
        .success());
    assert!(!home.join(".agents").exists());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    fs::write(&path, "name: Global\nagents: [codex]\nskills:\n  - name: one/sample\n    path: source\n  - name: two/sample\n    path: source\n").unwrap();
    assert!(!command(home, home, &["install", "--global", "--yes"])
        .status
        .success());
    assert!(!home.join(".agents").exists());
}

#[test]
fn global_install_applies_registry_dependencies_to_every_selected_target() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let home = home.path();
    let registry = home.join(".skm/registry");
    for (name, dependencies) in [
        ("alpha", "  skm-dependencies: 'team/beta@1.0.0'\n"),
        ("beta", ""),
    ] {
        let source = registry.join(format!("skills/team/{name}/v1.0.0"));
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), format!("---\nname: {name}\ndescription: test\nmetadata:\n  skm-version: '1.0.0'\n{dependencies}---\n")).unwrap();
    }
    let manifest = home.join(".skm/skills.yaml");
    fs::write(&manifest, "name: Global\nagents: [codex, cursor]\nregistries: {default: registry}\nskills:\n  - name: team/alpha\n    version: '1.0.0'\n").unwrap();
    let before = fs::read(&manifest).unwrap();
    let output = command(home, project.path(), &["install", "--global", "--yes"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for agent in [".agents/skills", ".cursor/skills"] {
        for skill in ["alpha", "beta"] {
            assert_eq!(
                fs::canonicalize(home.join(agent).join(skill)).unwrap(),
                fs::canonicalize(registry.join(format!("skills/team/{skill}/v1.0.0"))).unwrap()
            );
        }
    }
    assert_eq!(fs::read(manifest).unwrap(), before);
    let dry = success(command(
        home,
        project.path(),
        &["clean", "symlinks", "--global", "--orphaned", "--dry-run"],
    ));
    assert!(!String::from_utf8_lossy(&dry.stdout).contains("beta"));
    success(command(
        home,
        project.path(),
        &["clean", "symlinks", "--global", "--orphaned", "--yes"],
    ));
    for agent in [".agents/skills", ".cursor/skills"] {
        assert!(home.join(agent).join("beta").is_symlink());
    }
    assert!(command(home, project.path(), &["check", "--global"])
        .status
        .success());
    fs::rename(&registry, home.join(".skm/registry-unavailable")).unwrap();
    let failed = command(
        home,
        project.path(),
        &["clean", "symlinks", "--global", "--orphaned", "--yes"],
    );
    assert!(!failed.status.success());
    for agent in [".agents/skills", ".cursor/skills"] {
        assert!(home.join(agent).join("beta").is_symlink());
    }
}

#[test]
fn global_remove_uninstalled_registry_declaration_without_cache() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let home = home.path();
    success(command(
        home,
        project.path(),
        &["config", "show", "--global"],
    ));
    let registry = home.join(".skm/registry");
    let skill = registry.join("skills/acme/parent/v1.0.0");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: parent\nmetadata:\n  skm-version: '1.0.0'\n---\n",
    )
    .unwrap();
    symlink("v1.0.0", skill.parent().unwrap().join("latest")).unwrap();
    fs::write(
        registry.join("skills/acme/manifest.yaml"),
        "schema_version: 2\nnamespace: acme\npackages: {parent: 1.0.0}\nbundles: {}\n",
    )
    .unwrap();
    let manifest = home.join(".skm/skills.yaml");
    fs::write(&manifest, "name: User\nagents: [codex]\nregistries: {default: registry}\nskills: []\nextension: retained\n").unwrap();
    let independent = project.path().join("independent");
    fs::create_dir(&independent).unwrap();
    fs::write(
        independent.join("SKILL.md"),
        "---\nname: independent\n---\n",
    )
    .unwrap();
    success(command(
        home,
        project.path(),
        &[
            "add",
            "independent",
            "--path",
            "independent",
            "--global",
            "--yes",
        ],
    ));
    success(command(
        home,
        project.path(),
        &["install", "--global", "--yes"],
    ));
    let independent_link = home.join(".agents/skills/independent");
    let independent_target = fs::read_link(&independent_link).unwrap();
    success(command(
        home,
        project.path(),
        &["add", "acme/parent", "--global", "--yes"],
    ));
    assert!(!home.join(".skm/cache/registries/default").exists());
    // With existing targets and no dependency metadata, ownership is ambiguous.
    let other = home.join(".agents/skills/other");
    fs::create_dir_all(other.parent().unwrap()).unwrap();
    symlink(project.path(), &other).unwrap();
    let ambiguous = snapshot(home);
    assert!(!command(
        home,
        project.path(),
        &["remove", "acme/parent", "--global", "--yes"]
    )
    .status
    .success());
    assert_eq!(snapshot(home), ambiguous);
    fs::remove_file(other).unwrap();
    let git_stub_dir = project.path().join("git-stub");
    fs::create_dir(&git_stub_dir).unwrap();
    let git_stub = git_stub_dir.join("git");
    fs::write(&git_stub, "#!/bin/sh\n: > git-invoked\nexit 97\n").unwrap();
    fs::set_permissions(&git_stub, fs::Permissions::from_mode(0o755)).unwrap();
    let before = snapshot(home);
    success(
        Command::new(env!("CARGO_BIN_EXE_skm"))
            .current_dir(project.path())
            .args(["remove", "acme/parent", "--global", "--dry-run", "--yes"])
            .env("HOME", home)
            .env("XDG_CONFIG_HOME", home.join("xdg-config"))
            .env("XDG_CACHE_HOME", home.join("xdg-cache"))
            .env("SKM_NO_UPDATE_CHECK", "1")
            .env("PATH", &git_stub_dir)
            .output()
            .unwrap(),
    );
    assert!(!project.path().join("git-invoked").exists());
    assert_eq!(snapshot(home), before);
    success(command(
        home,
        project.path(),
        &["remove", "acme/parent", "--global", "--yes"],
    ));
    let value: serde_yaml::Value = serde_yaml::from_slice(&fs::read(manifest).unwrap()).unwrap();
    assert_eq!(value["skills"].as_sequence().unwrap().len(), 1);
    assert_eq!(value["skills"][0]["name"].as_str(), Some("independent"));
    assert_eq!(value["extension"].as_str(), Some("retained"));
    assert_eq!(fs::read_link(independent_link).unwrap(), independent_target);
    assert!(!home.join(".skm/cache/registries/default").exists());
}

#[test]
fn global_remove_rejects_conflicting_cache_even_without_links() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let home = home.path();
    success(command(
        home,
        project.path(),
        &["config", "show", "--global"],
    ));
    let registry = home.join(".skm/registry");
    let skill = registry.join("skills/acme/parent/v1.0.0");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: parent\nmetadata:\n  skm-version: '1.0.0'\n---\n",
    )
    .unwrap();
    symlink("v1.0.0", skill.parent().unwrap().join("latest")).unwrap();
    fs::write(
        registry.join("skills/acme/manifest.yaml"),
        "schema_version: 2\nnamespace: acme\npackages: {parent: 1.0.0}\nbundles: {}\n",
    )
    .unwrap();
    let manifest = home.join(".skm/skills.yaml");
    fs::write(
        &manifest,
        "name: User\nagents: [codex]\nregistries: {default: registry}\nskills: []\n",
    )
    .unwrap();
    success(command(
        home,
        project.path(),
        &["add", "acme/parent", "--global", "--yes"],
    ));
    let cache = home.join(".skm/cache/registries/default");
    assert!(!cache.exists());
    let other_registry = home.join("other-registry");
    fs::create_dir(&other_registry).unwrap();
    symlink(&other_registry, &cache).unwrap();
    let before = snapshot(home);
    for args in [
        &["install", "--global", "--dry-run"][..],
        &["remove", "acme/parent", "--global", "--dry-run", "--yes"],
        &["remove", "acme/parent", "--global", "--yes"],
    ] {
        let output = command(home, project.path(), args);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("different or missing origin"));
        assert_eq!(snapshot(home), before);
    }
}

#[test]
fn global_remove_unconfigured_registry_requires_absent_caches() {
    for cache_root in [
        None,
        Some(".skm/cache/registries"),
        Some(".cache/skm/registries"),
        Some("xdg-cache/skm/registries"),
    ] {
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let home = home.path();
        let registry = home.join("registry");
        let skill = registry.join("skills/acme/parent/v1.0.0");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: parent\nmetadata:\n  skm-version: '1.0.0'\n---\n",
        )
        .unwrap();
        symlink("v1.0.0", skill.parent().unwrap().join("latest")).unwrap();
        fs::write(
            registry.join("skills/acme/manifest.yaml"),
            "schema_version: 2\nnamespace: acme\npackages: {parent: 1.0.0}\nbundles: {}\n",
        )
        .unwrap();
        success(command(
            home,
            project.path(),
            &[
                "registry",
                "add",
                "acme-local",
                registry.to_str().unwrap(),
                "--skip-validate",
            ],
        ));
        let manifest = home.join(".skm/skills.yaml");
        fs::write(&manifest, "name: User\nagents: [codex]\nskills: []\n").unwrap();
        success(command(
            home,
            project.path(),
            &[
                "add",
                "acme/parent",
                "--registry",
                "acme-local",
                "--global",
                "--yes",
            ],
        ));
        success(command(
            home,
            project.path(),
            &["registry", "remove", "acme-local", "--yes"],
        ));
        if let Some(root) = cache_root {
            let cache = home.join(root).join("acme-local");
            fs::create_dir_all(cache.parent().unwrap()).unwrap();
            // A dangling entry also has an origin that cannot be validated.
            symlink(home.join("missing-registry"), cache).unwrap();
        }
        let before = snapshot(home);
        for args in [
            &["remove", "acme/parent", "--global", "--dry-run", "--yes"][..],
            &["remove", "acme/parent", "--global", "--yes"],
        ] {
            let output = command(home, project.path(), args);
            if cache_root.is_some() {
                assert!(!output.status.success());
                assert!(String::from_utf8_lossy(&output.stderr)
                    .contains("cannot validate cached registry"));
                assert_eq!(snapshot(home), before);
            } else {
                success(output);
                if args.contains(&"--dry-run") {
                    assert_eq!(snapshot(home), before);
                } else {
                    let value: serde_yaml::Value =
                        serde_yaml::from_slice(&fs::read(&manifest).unwrap()).unwrap();
                    assert!(value["skills"].as_sequence().unwrap().is_empty());
                }
            }
        }
    }
}

#[test]
fn global_remove_unlinks_orphan_dependencies_but_retains_shared_ones() {
    for shared in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let home = home.path();
        let registry = home.join(".skm/registry");
        for (name, dependency) in [
            ("parent", "  skm-dependencies: 'acme/helper@1.0.0'\n"),
            ("other", "  skm-dependencies: 'acme/helper@1.0.0'\n"),
            ("helper", ""),
        ] {
            let source = registry.join(format!("skills/acme/{name}/v1.0.0"));
            fs::create_dir_all(&source).unwrap();
            fs::write(
                source.join("SKILL.md"),
                format!("---\nname: {name}\nmetadata:\n  skm-version: '1.0.0'\n{dependency}---\n"),
            )
            .unwrap();
        }
        let extra = if shared {
            "  - {name: acme/other, version: '1.0.0'}\n"
        } else {
            ""
        };
        let manifest = home.join(".skm/skills.yaml");
        fs::write(&manifest, format!("name: User\nagents: [codex, cursor]\nregistries: {{default: registry}}\nskills:\n  - {{name: acme/parent, version: '1.0.0'}}\n{extra}extension: retained\n")).unwrap();
        success(command(
            home,
            project.path(),
            &["install", "--global", "--yes"],
        ));
        let before = snapshot(home);
        success(command(
            home,
            project.path(),
            &["remove", "acme/parent", "--global", "--dry-run", "--yes"],
        ));
        assert_eq!(snapshot(home), before);
        if !shared {
            let target = home.join(".cursor/skills/helper");
            let expected = fs::read_link(&target).unwrap();
            fs::remove_file(&target).unwrap();
            symlink(project.path(), &target).unwrap();
            let unsafe_before = snapshot(home);
            assert!(!command(
                home,
                project.path(),
                &["remove", "acme/parent", "--global", "--yes"]
            )
            .status
            .success());
            assert_eq!(snapshot(home), unsafe_before);
            fs::remove_file(&target).unwrap();
            symlink(expected, &target).unwrap();
        }
        success(command(
            home,
            project.path(),
            &["remove", "acme/parent", "--global", "--yes"],
        ));
        for agent in [".agents/skills", ".cursor/skills"] {
            assert!(!home.join(agent).join("parent").exists());
            assert_eq!(home.join(agent).join("helper").is_symlink(), shared);
            if shared {
                assert!(home.join(agent).join("other").is_symlink());
            }
        }
        let value: serde_yaml::Value =
            serde_yaml::from_slice(&fs::read(manifest).unwrap()).unwrap();
        assert_eq!(
            value["skills"].as_sequence().unwrap().len(),
            usize::from(shared)
        );
        assert_eq!(value["extension"].as_str(), Some("retained"));
    }
}

#[test]
fn global_remove_rejects_manifest_symlink_swap_during_confirmation() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let home = home.path();
    let registry = home.join(".skm/registry");
    for (name, dependency) in [
        ("parent", "  skm-dependencies: 'acme/helper@1.0.0'\n"),
        ("helper", ""),
    ] {
        let source = registry.join(format!("skills/acme/{name}/v1.0.0"));
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            format!("---\nname: {name}\nmetadata:\n  skm-version: '1.0.0'\n{dependency}---\n"),
        )
        .unwrap();
    }
    let manifest = home.join(".skm/skills.yaml");
    fs::write(&manifest, "name: User\nagents: [codex, cursor]\nregistries: {default: registry}\nskills:\n  - {name: acme/parent, version: '1.0.0'}\nextension: retained\n").unwrap();
    success(command(
        home,
        project.path(),
        &["install", "--global", "--yes"],
    ));
    let original = fs::read(&manifest).unwrap();
    let sibling = home.join(".skm/sibling.yaml");
    fs::write(&sibling, &original).unwrap();
    let targets: Vec<_> = [".agents/skills", ".cursor/skills"]
        .iter()
        .flat_map(|agent| {
            ["parent", "helper"]
                .iter()
                .map(move |skill| home.join(agent).join(skill))
        })
        .map(|path| {
            let destination = fs::read_link(&path).unwrap();
            (path, destination)
        })
        .collect();
    let mut child = Command::new(env!("CARGO_BIN_EXE_skm"))
        .current_dir(project.path())
        .args(["remove", "acme/parent", "--global"])
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("xdg-config"))
        .env("XDG_CACHE_HOME", home.join("xdg-cache"))
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
    fs::remove_file(&manifest).unwrap();
    symlink(&sibling, &manifest).unwrap();
    child.stdin.take().unwrap().write_all(b"y\n").unwrap();
    assert!(!child.wait().unwrap().success());
    assert!(manifest.is_symlink());
    assert_eq!(fs::read(sibling).unwrap(), original);
    for (path, destination) in targets {
        assert_eq!(fs::read_link(path).unwrap(), destination);
    }
}

#[test]
fn global_remove_and_orphan_cleanup_follow_user_manifest_across_directories() {
    let home = tempfile::tempdir().unwrap();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let home = home.path();
    let source = first.path().join("local");
    fs::create_dir(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: helper\ndescription: test\n---\n",
    )
    .unwrap();
    success(command(
        home,
        first.path(),
        &["init", "--global", "--non-interactive"],
    ));
    let manifest = home.join(".skm/skills.yaml");
    fs::write(
        &manifest,
        "name: User\nagents: [codex, cursor]\nskills: []\nextension: retained\n",
    )
    .unwrap();
    fs::write(
        second.path().join("skills.yaml"),
        "name: Other\nagents: []\nskills: []\n",
    )
    .unwrap();
    success(command(
        home,
        first.path(),
        &["add", "helper", "--path", "local", "--global", "--yes"],
    ));
    success(command(
        home,
        second.path(),
        &["install", "--global", "--yes"],
    ));
    let link = home.join(".agents/skills/helper");
    let second_link = home.join(".cursor/skills/helper");
    success(command(
        home,
        first.path(),
        &[
            "dev",
            "link",
            source.to_str().unwrap(),
            "--name",
            "team/dev-helper",
            "--global",
            "--agent",
            "codex",
        ],
    ));
    let dev_link = home.join(".agents/skills/team/dev-helper");
    let before = fs::read(&manifest).unwrap();
    success(command(
        home,
        second.path(),
        &["clean", "symlinks", "--global", "--orphaned", "--yes"],
    ));
    assert!(link.is_symlink() && second_link.is_symlink() && dev_link.is_symlink());
    success(command(
        home,
        second.path(),
        &["remove", "helper", "--global", "--dry-run"],
    ));
    assert_eq!(fs::read(&manifest).unwrap(), before);
    fs::remove_file(&second_link).unwrap();
    fs::write(&second_link, "real file").unwrap();
    assert!(!command(
        home,
        second.path(),
        &["remove", "helper", "--global", "--yes"]
    )
    .status
    .success());
    assert_eq!(fs::read(&manifest).unwrap(), before);
    assert!(link.is_symlink());
    fs::remove_file(&second_link).unwrap();
    symlink(&source, &second_link).unwrap();
    success(command(
        home,
        second.path(),
        &["remove", "helper", "--global", "--yes"],
    ));
    assert!(!link.is_symlink() && !second_link.is_symlink());
    let value: serde_yaml::Value = serde_yaml::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(value["extension"].as_str(), Some("retained"));
    assert!(value["skills"].as_sequence().unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(second.path().join("skills.yaml")).unwrap(),
        "name: Other\nagents: []\nskills: []\n"
    );
}

#[test]
fn global_cleanup_preview_rejects_unsafe_manifest_without_changes() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(home.path().join(".skm")).unwrap();
    let outside = home.path().join("outside");
    fs::write(&outside, "name: external\nagents: []\nskills: []\n").unwrap();
    symlink(&outside, home.path().join(".skm/skills.yaml")).unwrap();
    let before = snapshot(home.path());
    let output = command(
        home.path(),
        project.path(),
        &["clean", "symlinks", "--global", "--orphaned", "--dry-run"],
    );
    assert!(!output.status.success());
    assert_eq!(snapshot(home.path()), before);
}

#[test]
fn global_cleanup_refuses_symlinked_agent_root_and_preserves_project_links() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let home = home.path();
    let project = project.path();
    let source = project.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: local\ndescription: test\n---\n",
    )
    .unwrap();
    fs::write(
        project.join("skills.yaml"),
        "name: Project\nagents: [codex]\nskills: [{name: local, path: source}]\n",
    )
    .unwrap();
    success(command(home, project, &["install", "--yes"]));
    let root = home.join(".agents/skills");
    fs::create_dir_all(root.parent().unwrap()).unwrap();
    symlink(project.join(".agents/skills"), &root).unwrap();
    let link = project.join(".agents/skills/local");
    let before = fs::read_link(&link).unwrap();
    for args in [
        &["clean", "symlinks", "--global", "--orphaned", "--dry-run"][..],
        &["clean", "symlinks", "--global", "--orphaned", "--yes"],
    ] {
        let result = command(home, project, args);
        assert!(!result.status.success());
        assert_eq!(fs::read_link(&link).unwrap(), before);
    }
    assert!(command(home, project, &["check"]).status.success());
}

#[test]
fn cache_prune_preserves_global_dependency_versions_from_git_cache() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let other_project = tempfile::tempdir().unwrap();
    let home = home.path();
    let project = project.path();
    let origin = home.join("origin");
    fs::create_dir(&origin).unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&origin)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "--initial-branch=main"]);
    git(&["config", "user.name", "SKM Test"]);
    git(&["config", "user.email", "skm-test@example.invalid"]);
    for name in ["alpha", "beta"] {
        for version in ["1.0.0", "2.0.0"] {
            let path = origin.join(format!("skills/team/{name}/v{version}"));
            fs::create_dir_all(&path).unwrap();
            let dependency = if name == "alpha" && version == "1.0.0" {
                "  skm-dependencies: 'team/beta@1.0.0'\n"
            } else {
                ""
            };
            fs::write(path.join("SKILL.md"), format!("---\nname: {name}\ndescription: test\nmetadata:\n  skm-version: '{version}'\n{dependency}---\n")).unwrap();
        }
    }
    git(&["add", "."]);
    git(&["commit", "-m", "fixture"]);
    success(command(home, project, &["config", "show", "--global"]));
    fs::write(
        home.join(".skm/config.yaml"),
        format!(
            "default_registry: default\nregistries:\n  default: '{}'\ncheck_for_updates: false\n",
            origin.display()
        ),
    )
    .unwrap();
    let manifest = home.join(".skm/skills.yaml");
    fs::write(&manifest, "name: Global\nagents: [codex, cursor]\nskills:\n  - {name: team/alpha, version: '1.0.0', source: default}\n").unwrap();
    success(command(home, project, &["install", "--global", "--yes"]));
    let cache = home.join(".skm/cache/registries/default/skills/team");
    assert!(cache.join("alpha/v1.0.0").exists() && cache.join("beta/v1.0.0").exists());
    let before = snapshot(home);
    let preview = success(command(
        home,
        project,
        &["cache", "prune", "default", "--keep", "1", "--dry-run"],
    ));
    let text = String::from_utf8_lossy(&preview.stdout);
    assert!(!text.contains("v1.0.0"));
    assert!(text.contains("v2.0.0"));
    assert_eq!(snapshot(home), before);
    let original = fs::read(&manifest).unwrap();
    fs::write(&manifest, "name: Global\nagents: [codex, cursor]\nregistries: {default: /wrong-origin}\nskills:\n  - {name: team/alpha, version: '1.0.0', source: default}\n").unwrap();
    let conflict_snapshot = snapshot(home);
    let conflict = command(
        home,
        other_project.path(),
        &["cache", "prune", "default", "--keep", "1", "--yes"],
    );
    assert!(!conflict.status.success());
    assert_eq!(snapshot(home), conflict_snapshot);
    fs::write(&manifest, &original).unwrap();
    success(command(
        home,
        other_project.path(),
        &["cache", "prune", "default", "--keep", "1", "--yes"],
    ));
    for name in ["alpha", "beta"] {
        assert!(cache.join(name).join("v1.0.0").exists());
        assert!(!cache.join(name).join("v2.0.0").exists());
    }
    success(command(home, project, &["check", "--global"]));
    assert_eq!(fs::read_to_string(manifest).unwrap(), "name: Global\nagents: [codex, cursor]\nskills:\n  - {name: team/alpha, version: '1.0.0', source: default}\n");
}
