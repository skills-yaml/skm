use std::process::Command;

fn skm() -> Command {
    Command::new(env!("CARGO_BIN_EXE_skm"))
}

#[test]
fn update_command_documents_channels_and_compatibility_flags() {
    let output = skm()
        .args(["self", "upgrade", "--help"])
        .env("SKM_NO_UPDATE_CHECK", "1")
        .output()
        .expect("run skm self upgrade help");

    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).expect("UTF-8 help");
    assert!(help.contains("--channel <CHANNEL>"));
    assert!(help.contains("--yes"));
    assert!(help.contains("prod"));
    assert!(help.contains("development"));
}

#[test]
fn local_build_update_fails_before_network_or_replacement() {
    let output = skm()
        .args(["self", "upgrade", "--yes"])
        .env("SKM_UPDATE_BASE_URL", "http://127.0.0.1:1/")
        .env("SKM_NO_UPDATE_CHECK", "1")
        .output()
        .expect("run skm self upgrade");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 update error");
    assert!(stderr.contains("not self-update managed"));
    assert!(!stderr.contains("release request failed"));
}

#[test]
fn version_command_reports_the_embedded_identity() {
    let output = skm()
        .args(["self", "version"])
        .env("SKM_NO_UPDATE_CHECK", "1")
        .output()
        .expect("run skm self version");

    assert!(output.status.success());
    let version = String::from_utf8(output.stdout).expect("UTF-8 version");
    assert!(version.starts_with("skm "));
    assert!(version.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn legacy_updater_identity_probe_requires_exact_arguments_and_environment() {
    let current = skm()
        .args(["self", "version"])
        .env("SKM_NO_UPDATE_CHECK", "1")
        .output()
        .expect("run current version command");
    assert!(current.status.success());

    let probe = skm()
        .arg("version")
        .env("SKM_NO_UPDATE_CHECK", "1")
        .output()
        .expect("run legacy updater identity probe");
    assert!(probe.status.success());
    assert_eq!(probe.stdout, current.stdout);
    assert!(probe.stderr.is_empty());

    for value in [None, Some("0"), Some("true")] {
        let mut command = skm();
        command.arg("version");
        match value {
            Some(value) => {
                command.env("SKM_NO_UPDATE_CHECK", value);
            }
            None => {
                command.env_remove("SKM_NO_UPDATE_CHECK");
            }
        }
        let output = command.output().expect("run ordinary old version command");
        assert!(!output.status.success(), "version succeeded with {value:?}");
    }

    let extra = skm()
        .args(["version", "--help"])
        .env("SKM_NO_UPDATE_CHECK", "1")
        .output()
        .expect("run probe with extra argument");
    assert!(!extra.status.success());
}
