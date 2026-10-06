# Accept Legacy Updater Identity Probes

## Status

State: `done`

Rationale: PR #83 merged into main at 7a289ab9edcb8799c295041b2fab63a79ba233fa; actual main CI 37454486476 and development CI 37454491771 passed. Qualification 37455327437 passed current-pair updates, actual legacy 0.8.0 updates, and modern-bootstrap rejection on Linux, macOS Intel/ARM, and Windows while production remained held. Release Artifacts 37454486538 then published verified SKM 0.9.0 through the normal eligible-reviewer approval path on 2026-10-06; all nine assets, manifest/source identity, checksums, Linux binary identity, and repeat-update stability were verified.

## Problem and Users

An older managed `skm update` verifies a staged candidate by invoking `version` with `SKM_NO_UPDATE_CHECK=1`. The reorganized CLI rejects top-level `version`, so verified updates to the new development release fail before replacing the binary. On Windows, the old updater invokes the same probe again after replacement.

## Goals

- Allow the exact legacy updater identity probe to return the embedded version, channel, and commit on staged and installed binaries.
- Preserve rejection of ordinary old command names and other `version` invocations.
- Qualify the upgrade path from an older managed binary to a new candidate on every supported release platform.

## Non-goals

- Restoring a documented or generally available top-level `version` command.
- Restoring other removed command names or changing manifest, cache, or skill behavior.
- Weakening archive, checksum, manifest, binary identity, or executable replacement checks.

## Design

Before Clap parsing, after Windows worker dispatch, accept only exactly one positional argument `version` when `SKM_NO_UPDATE_CHECK` is exactly `1`. Print the existing `version::show_version()` output and exit successfully. Every other top-level `version` call still fails parsing. The probe performs no configuration, network, or filesystem writes.

Release qualification detects whether the bootstrap binary uses `self version` or the older `version`, invokes its matching update command, then requires the updated candidate to answer `self version`. Repeat-update qualification uses the new command. This exercises the old-to-new transition rather than rejecting a legacy bootstrap.

## Affected Areas

- `src/main.rs`: internal probe dispatch and unit tests.
- `tests/updater.rs` and `tests/command_layout.rs`: probe success and ordinary old-name rejection.
- `scripts/qualify-release-update.sh` and `.ps1`: old-to-new qualification routing.
- `README.md`: replace obsolete bridge-release warning with the supported migration path.
- `workspace/specs/test/skill-lifecycle/command-reorganization.md` and `workspace/specs/README.md`: reconcile the earlier release constraint and catalog rationale.
- `workspace/agents/memory/`: supersede the bridge prerequisite with the approved handshake decision.

## Compatibility, Safety, and Rollback

The compatibility response is gated by the exact environment value and argument vector used by old updaters. It prints public build identity only. The normal CLI remains breaking and its help stays unchanged. The old updater still performs its existing manifest and checksum verification before the probe. A source revert would restore the old-to-new failure; release qualification must catch that before promotion.

## Acceptance Criteria

1. A binary invoked as `skm version` with `SKM_NO_UPDATE_CHECK=1` prints the exact embedded identity and exits zero.
2. `skm version` without that exact environment value, and `skm version` with extra arguments, fail; removed command names remain unavailable.
3. Existing `skm self version`, `self check`, and `self upgrade` behavior remains unchanged.
4. Release qualification scripts exercise old bootstrap commands when necessary, then verify the new candidate through `self version` and a repeat `self upgrade`.
5. `task check`, `task test`, and `git diff --check` pass, and user-local configuration changes remain untouched.

## Validation Gates

- Focused CLI regression tests for exact probe and rejection paths.
- `task check`, `task test`, and `git diff --check`.
- Development release update qualification across Linux, macOS, and Windows before production promotion.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | patch | skm-next | Restored compatibility for an existing updater probe. |

## Memory Impact

Status: `updated`

Rationale: The user-approved compatibility contract is recorded in `workspace/agents/memory/decisions.md`; the reproduced failure, superseded bridge prerequisite, and confirmed development integration are recorded in `workspace/agents/memory/facts.md`. They have corresponding entries in `workspace/agents/memory/changelog.md`.

## Verified Production Completion

PR #83 merged into main at 7a289ab9edcb8799c295041b2fab63a79ba233fa; actual main CI 37454486476 and development CI 37454491771 passed. Qualification 37455327437 passed current-pair updates, actual legacy 0.8.0 updates, and modern-bootstrap rejection on Linux, macOS Intel/ARM, and Windows while production remained held. Release Artifacts 37454486538 then published verified SKM 0.9.0 through the normal eligible-reviewer approval path on 2026-10-06; all nine assets, manifest/source identity, checksums, Linux binary identity, and repeat-update stability were verified.

Exact legacy probe/rejection regression tests pass. All four native supplemental paths started from immutable development 0.8.0 at 07b4bba, exercised its version/update commands, verified replacement to this exact candidate, and repeated with the new self commands; Windows cleanup also passed.

Memory is updated in `workspace/agents/memory/facts.md` and
`workspace/agents/memory/changelog.md`. Applied version 0.9.0 is now released;
no duplicate bump is introduced. Earlier local/pending statements describe
historical snapshots. These completion records receive fresh independent review
and frozen aggregate verification before their record-only main closeout.
