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
