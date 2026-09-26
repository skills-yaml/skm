use std::fs;
use std::path::PathBuf;

fn repository_text(path: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path))
        .unwrap_or_else(|error| panic!("read {path}: {error}"))
        .replace("\r\n", "\n")
}

#[test]
fn publisher_requires_a_complete_skm_manifest_asset_set() {
    let publisher = repository_text("scripts/publish-release.sh");
    for required in [
        "skm-linux-x86_64.tar.gz",
        "skm-macos-aarch64.tar.gz",
        "skm-macos-x86_64.tar.gz",
        "skm-windows-x86_64.zip",
        "skm-release.json",
        "Release transaction requires exactly four archives",
        "skm-release-transaction-v1",
    ] {
        assert!(publisher.contains(required), "publisher lacks {required}");
    }
}

#[test]
fn release_workflows_gate_and_qualify_publication() {
    let release = repository_text(".github/workflows/release.yml");
    assert!(release.contains("release-${{ needs.prepare.outputs.channel }}"));
    assert!(release.contains("scripts/publish-release.sh"));
    assert!(release.contains("SKM_BUILD_COMMIT"));
    assert!(release.contains("SKM_BUILD_CHANNEL"));

    let qualification = repository_text(".github/workflows/release-update-qualification.yml");
    assert!(qualification.contains("release-prod"));
    assert!(qualification.contains("skm-release.json"));
    assert!(qualification.contains("qualify:release-update:unix"));
    assert!(qualification.contains("qualify:release-update:windows"));
}

#[test]
fn windows_update_worker_dispatch_precedes_cli_parsing() {
    let main = repository_text("src/main.rs");
    let worker_dispatch = main
        .find("updater::run_windows_update_worker_if_requested()")
        .expect("main must dispatch the private Windows updater worker");
    let legacy_probe = main
        .find("is_legacy_updater_version_probe(&args")
        .expect("main must handle the legacy updater identity probe");
    let cli_parsing = main
        .find("parse_cli_with_help_notice(args")
        .expect("main must parse the public CLI");

    assert!(
        worker_dispatch < legacy_probe && legacy_probe < cli_parsing,
        "the Windows updater worker and legacy probe must run before Clap parsing"
    );
}
