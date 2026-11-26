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
    fn new() -> Self {
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
        fs::write(fixture.project.join("skills.yaml"), format!("name: fixture\nregistries:\n  local: {}\nagents: [codex]\nskills:\n  - {{name: acme/starter, kind: bundle, source: local}}\n", fixture.registry.display())).unwrap();
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

    fn interactive_upgrade(&self, args: &[&str], prompt: &str, edit: bool) -> Output {
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
        if edit {
            fs::write(
                self.project.join("skills.yaml"),
                "name: externally-edited\nagents: [codex]\nskills: []\n",
            )
            .unwrap();
        }
        child.stdin.take().unwrap().write_all(b"y\n").unwrap();
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
fn bundle_upgrades_validate_the_parsed_manifest_before_writes() {
    for (args, prompt) in [
        (
            &["skill", "upgrade", "acme/starter"][..],
            "Upgrade bundles? [y/N] ",
        ),
        (
            &["skill", "upgrade", "--all"][..],
            "Upgrade 0 skill(s)? [y/N] ",
        ),
    ] {
        let fixture = Fixture::new();
        let manifest = fs::read(fixture.project.join("skills.yaml")).unwrap();
        let lock = fs::read(fixture.project.join("skills.lock.yaml")).unwrap();
        let link = fixture.project.join(".agents/skills/helper");
        let original_target = fs::read_link(&link).unwrap();
        let mut dry_run = args.to_vec();
        dry_run.push("--dry-run");
        fixture.skm(&dry_run);
        assert_eq!(
            fs::read(fixture.project.join("skills.yaml")).unwrap(),
            manifest
        );
        assert_eq!(
            fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
            lock
        );
        assert_eq!(fs::read_link(&link).unwrap(), original_target);

        let output = fixture.interactive_upgrade(args, prompt, true);
        assert!(!output.status.success(), "{args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("skills.yaml changed after planning")
        );
        assert_eq!(
            fs::read(fixture.project.join("skills.yaml")).unwrap(),
            b"name: externally-edited\nagents: [codex]\nskills: []\n"
        );
        assert_eq!(
            fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
            lock
        );
        assert_eq!(fs::read_link(&link).unwrap(), original_target);

        fs::write(fixture.project.join("skills.yaml"), &manifest).unwrap();
        let output = fixture.interactive_upgrade(args, prompt, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(fixture.project.join("skills.yaml")).unwrap(),
            manifest
        );
        assert_ne!(
            fs::read(fixture.project.join("skills.lock.yaml")).unwrap(),
            lock
        );
        assert!(fs::read_link(&link).unwrap().ends_with("v2.0.0"));
        fixture.skm(&["check"]);
    }
}
