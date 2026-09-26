use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_home() -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "skm-command-layout-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn skm(home: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_skm"));
    command
        .current_dir(home)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("SKM_NO_UPDATE_CHECK", "1");
    command
}

fn git(directory: &std::path::Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(directory)
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
fn command_groups_replace_old_names() {
    let home = temp_home();
    for args in [
        &["cache", "refresh", "--help"][..],
        &["cache", "status", "--help"],
        &["cache", "prune", "--help"],
        &["cache", "clear", "--help"],
        &["skill", "versions", "--help"],
        &["skill", "use", "--help"],
        &["skill", "outdated", "--help"],
        &["skill", "upgrade", "--help"],
        &["self", "version", "--help"],
        &["self", "check", "--help"],
        &["self", "upgrade", "--help"],
    ] {
        let output = skm(&home).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for args in [
        &["version"][..],
        &["versions", "sample"],
        &["use", "sample@v1.0.0"],
        &["update-skill", "sample"],
        &["update"],
        &["cache-update"],
        &["registry", "update", "default"],
        &["clean", "cache", "--stats"],
    ] {
        let output = skm(&home).args(args).output().unwrap();
        assert!(
            !output.status.success(),
            "old command still works: {args:?}"
        );
    }
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn outdated_uses_cache_and_preserves_manifest() {
    let home = temp_home();
    let manifest = "name: test\nagents: []\nskills:\n  - name: sample\n    source: default\n    version: v1.0.0\n";
    fs::write(home.join("skills.yaml"), manifest).unwrap();
    let cache = home.join(".cache/skm/registries/default/skills/sample");
    for version in ["v1.0.0", "v1.1.0", "v2.0.0-beta.1"] {
        fs::create_dir_all(cache.join(version)).unwrap();
    }

    let stable = skm(&home)
        .args(["skill", "outdated", "sample"])
        .output()
        .unwrap();
    assert!(
        stable.status.success(),
        "{}",
        String::from_utf8_lossy(&stable.stderr)
    );
    let stable_text = String::from_utf8(stable.stdout).unwrap();
    assert!(stable_text.contains("sample: v1.0.0 -> v1.1.0 (default)"));
    assert!(!stable_text.contains("v2.0.0-beta.1"));

    let pre = skm(&home)
        .args(["skill", "outdated", "sample", "--pre"])
        .output()
        .unwrap();
    assert!(pre.status.success());
    assert!(String::from_utf8(pre.stdout)
        .unwrap()
        .contains("v2.0.0-beta.1"));

    let missing = skm(&home)
        .args(["skill", "outdated", "unknown"])
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8(missing.stderr)
        .unwrap()
        .contains("not found in configuration"));
    assert_eq!(
        fs::read_to_string(home.join("skills.yaml")).unwrap(),
        manifest
    );
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn cache_refresh_fetches_a_new_remote_commit() {
    let home = temp_home();
    let origin = home.join("origin.git");
    let seed = home.join("seed");
    fs::create_dir_all(&seed).unwrap();
    git(
        &home,
        &[
            "init",
            "--bare",
            "--initial-branch=main",
            origin.to_str().unwrap(),
        ],
    );
    git(&seed, &["init", "--initial-branch=main"]);
    git(&seed, &["config", "user.email", "skm-test@example.invalid"]);
    git(&seed, &["config", "user.name", "SKM Test"]);
    git(
        &seed,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    );
    fs::write(seed.join("first.txt"), "first").unwrap();
    git(&seed, &["add", "."]);
    git(&seed, &["commit", "-m", "first"]);
    git(&seed, &["push", "-u", "origin", "main"]);

    let config_dir = home.join(".config/skm");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.yaml"),
        format!(
            "default_registry: demo\nregistries:\n  demo: {}\n",
            origin.display()
        ),
    )
    .unwrap();
    let first = skm(&home)
        .args(["cache", "refresh", "demo"])
        .output()
        .unwrap();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let cached = home.join(".cache/skm/registries/demo");
    assert!(cached.join("first.txt").exists());

    fs::write(seed.join("second.txt"), "second").unwrap();
    git(&seed, &["add", "."]);
    git(&seed, &["commit", "-m", "second"]);
    git(&seed, &["push", "origin", "main"]);
    let second = skm(&home)
        .args(["cache", "refresh", "demo"])
        .output()
        .unwrap();
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(cached.join("second.txt").exists());

    let status = skm(&home)
        .args(["cache", "status", "demo"])
        .output()
        .unwrap();
    assert!(status.status.success());
    let clear = skm(&home)
        .args(["cache", "clear", "demo", "--dry-run"])
        .output()
        .unwrap();
    assert!(clear.status.success());
    assert!(cached.exists());

    fs::write(
        config_dir.join("config.yaml"),
        "default_registry: demo\nregistries:\n  demo: /unexpected/origin\n",
    )
    .unwrap();
    let mismatch = skm(&home)
        .args(["cache", "refresh", "demo"])
        .output()
        .unwrap();
    assert!(!mismatch.status.success());
    assert!(String::from_utf8(mismatch.stderr)
        .unwrap()
        .contains("different or missing origin"));
    assert!(cached.join("second.txt").exists());
    fs::remove_dir_all(home).unwrap();
}
