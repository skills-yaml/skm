#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

struct Fixture {
    _root: tempfile::TempDir,
    project: PathBuf,
    home: PathBuf,
    registry: PathBuf,
}

impl Fixture {
    fn new(bundle: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        let home = root.path().join("home");
        let registry = root.path().join("registry");
        for dir in [&project, &home, &registry] {
            fs::create_dir_all(dir).unwrap();
        }
        let fixture = Self {
            _root: root,
            project,
            home,
            registry,
        };
        fixture.publish("1.0.0");
        git(&fixture.registry, &["init", "--quiet"]);
        fixture.commit();
        let entry = if bundle {
            "{name: acme/starter, kind: bundle, source: local}"
        } else {
            "{name: acme/helper, version: v1.0.0, source: local}"
        };
        fs::write(
            fixture.project.join("skills.yaml"),
            format!(
                "name: fixture\nregistries:\n  local: {}\nagents: [codex]\nskills:\n  - {entry}\n",
                fixture.registry.display()
            ),
        )
        .unwrap();
        fixture.skm(&["cache", "refresh", "local"]);
        fixture.skm(&["install", "--yes"]);
        fixture.publish("2.0.0");
        fixture.commit();
        fixture.skm(&["cache", "refresh", "local"]);
        fixture
    }

    fn publish(&self, version: &str) {
        let skill = self.registry.join(format!("skills/acme/helper/v{version}"));
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            format!("---\nname: helper\nmetadata:\n  skm-version: \"{version}\"\n---\n# helper\n"),
        )
        .unwrap();
        fs::write(self.registry.join("skills/acme/manifest.yaml"), format!("schema_version: 2\nnamespace: acme\npackages: {{helper: {version}}}\nbundles: {{starter: {{packages: [helper]}}}}\n")).unwrap();
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

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_skm"));
        command
            .args(args)
            .current_dir(&self.project)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("XDG_CACHE_HOME", self.home.join(".cache"))
            .env("SKM_NO_UPDATE_CHECK", "1");
        command
    }

    fn skm(&self, args: &[&str]) -> Output {
        let output = self.command(args).output().unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn interactive(&self, args: &[&str], prompt: &str, answer: &[u8]) -> Output {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        let prompt = prompt.as_bytes().to_vec();
        let reader = std::thread::spawn(move || {
            let mut captured = Vec::new();
            let mut byte = [0];
            while stdout.read(&mut byte).unwrap() != 0 {
                captured.push(byte[0]);
                if captured.ends_with(&prompt) {
                    tx.send(()).unwrap();
                    break;
                }
            }
            stdout.read_to_end(&mut captured).unwrap();
            captured
        });
        if rx.recv_timeout(Duration::from_secs(30)).is_err() {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            let text = reader.join().unwrap();
            panic!(
                "prompt not reached: {} {}",
                String::from_utf8_lossy(&text),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        {
            fs::write(
                self.project.join("skills.yaml"),
                "name: externally-edited\nagents: [codex]\nskills: []\n",
            )
            .unwrap();
        }
        child.stdin.take().unwrap().write_all(answer).unwrap();
        let mut output = child.wait_with_output().unwrap();
        output.stdout = reader.join().unwrap();
        output
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
fn cancelled_use_and_upgrade_preserve_lock_when_manifest_changes_externally() {
    for (bundle, args, prompt) in [
        (true, & ["skill", "upgrade", "acme/starter"][..], "Upgrade bundles? [y/N] "),
        (true, & ["skill", "use", "acme/starter@latest"][..], "Update bundle selection? [y/N] "),
        (false, & ["skill", "upgrade", "acme/helper"][..], "Are you sure you want to switch skill 'acme/helper' from v1.0.0 to version 'v2.0.0'? [y/N] "),
        (false, & ["skill", "use", "acme/helper@v2.0.0"][..], "Are you sure you want to switch skill 'acme/helper' from v1.0.0 to version 'v2.0.0'? [y/N] "),
    ] {
        let fixture = Fixture::new(bundle);
        let lock = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
        let link = fixture.project.join(".agents/skills/helper");
        let original_target = fs::read_link(&link).unwrap();
        let output = fixture.interactive(args, prompt, b"n\n");
        assert!(output.status.success(), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(fs::read(fixture.project.join("skills.yaml")).unwrap(), b"name: externally-edited\nagents: [codex]\nskills: []\n");
        assert_eq!(fs::read(fixture.project.join("skills.lock.yaml")).unwrap(), lock, "{args:?}");
        assert_eq!(fs::read_link(&link).unwrap(), original_target);
    }
}

#[test]
fn standalone_use_and_upgrade_sync_existing_lock_after_success() {
    for args in [
        &["skill", "upgrade", "acme/helper", "--yes"][..],
        &["skill", "use", "acme/helper@v2.0.0", "--yes"][..],
    ] {
        let fixture = Fixture::new(false);
        let lock = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
        fixture.skm(args);
        assert_ne!(
            fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
            lock,
            "{args:?}"
        );
        assert!(fs::read_link(fixture.project.join(".agents/skills/helper"))
            .unwrap()
            .ends_with("v2.0.0"));
        fixture.skm(&["check"]);
    }
}

#[test]
fn unchanged_standalone_pin_does_not_rewrite_lock() {
    let fixture = Fixture::new(false);
    let lock_path = fixture.project.join("skills.lock.yaml");
    let mut lock = fs::read(&lock_path).unwrap();
    lock.extend_from_slice(b"# preserved comment\n");
    fs::write(&lock_path, &lock).unwrap();
    fixture.skm(&["skill", "use", "acme/helper@v1.0.0", "--yes"]);
    assert_eq!(fs::read(lock_path).unwrap(), lock);
}
