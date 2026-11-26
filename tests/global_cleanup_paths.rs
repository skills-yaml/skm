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
fn cleanup_uses_physical_global_manifest_root_through_symlinked_home_ancestor() {
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
    let origin = skm.join("origin");
    fs::create_dir(&origin).unwrap();
    fs::create_dir(skm.join("nested")).unwrap();
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
    let physical_skm = fs::canonicalize(&skm).unwrap();
    fs::write(skm.join("config.yaml"), format!("default_registry: default\nregistries:\n  default: '{}/nested/../origin'\ncheck_for_updates: false\n", physical_skm.display())).unwrap();
    let manifest = "name: Global\nagents: [codex]\nregistries: {default: nested/../origin}\nskills:\n  - {name: team/alpha, version: '1.0.0', source: default}\n";
    fs::write(skm.join("skills.yaml"), manifest).unwrap();
    success(command(
        &home,
        &project,
        &["skill", "outdated", "--global", "--refresh"],
    ));
    let cache = skm.join("cache/registries/default");
    let cached_origin = success(
        Command::new("git")
            .current_dir(&cache)
            .args(["remote", "get-url", "origin"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        String::from_utf8(cached_origin.stdout).unwrap(),
        format!("{}/nested/../origin\n", physical_skm.display())
    );
    success(command(&home, &project, &["install", "--global", "--yes"]));
    success(command(&home, &project, &["check", "--global"]));
    let before = snapshot(&physical);
    let project_before = snapshot(&project);
    for selected_home in [&fs::canonicalize(&home).unwrap(), &home] {
        let clean = success(command(
            selected_home,
            &project,
            &["clean", "symlinks", "--global", "--orphaned", "--dry-run"],
        ));
        assert!(!String::from_utf8_lossy(&clean.stdout).contains("team/alpha"));
        let prune = success(command(
            selected_home,
            &project,
            &["cache", "prune", "default", "--keep", "1", "--dry-run"],
        ));
        let output = String::from_utf8_lossy(&prune.stdout);
        assert!(!output.contains("v1.0.0"));
        assert!(output.contains("v2.0.0"));
        assert_eq!(snapshot(&physical), before);
        assert_eq!(snapshot(&project), project_before);
    }
    fs::write(
        skm.join("skills.yaml"),
        manifest.replace("nested/../origin", "/wrong-origin"),
    )
    .unwrap();
    let conflict_before = snapshot(&physical);
    for args in [
        &["clean", "symlinks", "--global", "--orphaned", "--dry-run"][..],
        &["cache", "prune", "default", "--keep", "1", "--dry-run"],
    ] {
        let result = command(&home, &project, args);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("different or missing origin"));
        assert_eq!(snapshot(&physical), conflict_before);
        assert_eq!(snapshot(&project), project_before);
    }
}
