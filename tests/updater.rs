use std::process::Command;

fn skm() -> Command {
    Command::new(env!("CARGO_BIN_EXE_skm"))
}

#[test]
fn update_command_documents_channels_and_compatibility_flags() {
    let output = skm()
        .args(["update", "--help"])
        .env("SKM_NO_UPDATE_CHECK", "1")
        .output()
        .expect("run skm update help");

    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).expect("UTF-8 help");
    assert!(help.contains("--channel <CHANNEL>"));
    assert!(help.contains("--check"));
    assert!(help.contains("--yes"));
    assert!(help.contains("prod"));
    assert!(help.contains("development"));
}

#[test]
fn local_build_update_fails_before_network_or_replacement() {
    let output = skm()
        .args(["update", "--yes"])
        .env("SKM_UPDATE_BASE_URL", "http://127.0.0.1:1/")
        .env("SKM_NO_UPDATE_CHECK", "1")
        .output()
        .expect("run skm update");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 update error");
    assert!(stderr.contains("not self-update managed"));
    assert!(!stderr.contains("release request failed"));
}

#[test]
fn version_command_reports_the_embedded_identity() {
    let output = skm()
        .args(["version"])
        .env("SKM_NO_UPDATE_CHECK", "1")
        .output()
        .expect("run skm version");

    assert!(output.status.success());
    let version = String::from_utf8(output.stdout).expect("UTF-8 version");
    assert!(version.starts_with("skm "));
    assert!(version.contains(env!("CARGO_PKG_VERSION")));
}
