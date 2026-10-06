# Qualify Legacy Updaters Before Production Publication

## Status

State: `done`

Rationale: PR #83 merged into main at 7a289ab9edcb8799c295041b2fab63a79ba233fa; actual main CI 37454486476 and development CI 37454491771 passed. Qualification 37455327437 passed current-pair updates, actual legacy 0.8.0 updates, and modern-bootstrap rejection on Linux, macOS Intel/ARM, and Windows while production remained held. Release Artifacts 37454486538 then published verified SKM 0.9.0 through the normal eligible-reviewer approval path on 2026-10-06; all nine assets, manifest/source identity, checksums, Linux binary identity, and repeat-update stability were verified.

## Scope and Implementation Plan

Extend the existing qualification workflow with an optional immutable older development bootstrap. Preserve the mandatory consecutive current release pair, exact main/development/tag checks, production hold, and all checksum/source checks. Verify the supplemental run is a successful earlier development Release Artifacts run for the exact ancestor commit. Download its artifacts on all four native platforms and run the existing Task qualification entrypoints in a mode that requires legacy command detection. Add native negative checks using the current candidate artifact so a modern bootstrap cannot pass as legacy.

Affected areas: `.github/workflows/release-update-qualification.yml`, Unix/Windows qualification scripts, human validation module contracts, this spec/catalog, and memory. Exclude product runtime, dependencies, governed instructions, protection settings, and personal data. Linked contracts: [legacy updater identity handshake](../../done/updates/legacy-updater-identity-handshake.md) and [command reorganization](../../done/skill-lifecycle/command-reorganization.md).

## Acceptance Criteria

1. Existing consecutive-pair and exact pending-production gates remain mandatory; legacy inputs are both supplied or both absent, and supplied input IDs/commits/provenance/ancestry are verified.
2. Supplemental native tests require an actual old `version`/`update` bootstrap on Linux, macOS Intel, macOS ARM, and Windows; a modern bootstrap is rejected before notification/update with a specific error.
3. Both ordinary and legacy paths verify terminal notification, checksum/identity-bound replacement, exact candidate identity, and byte-preserving repeat updates; Windows also verifies cleanup.
4. Independent exact-candidate review and frozen `task check`, `task test`, and `task build` pass. Actual development/main CI and exact-candidate four-platform qualification pass before production approval.
5. Record confirmed integration/main events, acceptance, catalog, version impact, and memory; preserve historical and user data. Publication uses the existing protected workflow under the explicit release request.

## Validation Gates

Reproduce modern-bootstrap acceptance without the guard using a native Task invocation; test the corrected early rejection against a verified current artifact. Run script checks and release contract tests, then independent review and all aggregate checks/tests/build. After development/main integration, dispatch qualification with recent consecutive release inputs plus legacy development run `36197445530` at `07b4bba9c1d5f3de3964f398ff8296afb617db6e`. All four positive legacy paths and modern-bootstrap negative cases must pass while exact production remains held. Do not weaken or bypass protection rules.

## Risks and Compatibility

Optional inputs retain ordinary qualification compatibility. Historical artifacts must remain available and immutable; fail if expired or wrong. The extra mode rejects current commands rather than accepting mislabeled bootstraps. This adds validation only and changes no released binary behavior. Use one mutating agent and independent read-only review.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | none | none | Qualification-only workflow/script coverage changes no product binary, CLI, or installation format; retain the already-applied shared 0.9.0 release. |

## Memory Impact

Status: `updated`

Rationale: Recorded the supplemental legacy-path requirement and preserved current-pair/protection gates in `workspace/agents/memory/decisions.md` and `workspace/agents/memory/changelog.md`; actual qualification/publication remain unconfirmed.

## Local Regression Evidence

Before the guard, native legacy-mode invocation of verified modern bootstrap
`6e572b2` entered self-version notification rather than rejecting it. After the
fix, the same Task invocation fails immediately with the specific legacy
bootstrap error after checksum verification. Workflow negative cases repeat this
assertion on all four platforms; positive old-bootstrap checks remain mandatory
for this production release. Supplemental artifact run 36197445530 is successful
and all four artifacts are unexpired. Script and release-contract gates pass.

## Confirmed Test Integration

PR #82 merged into development at 1b6a6e0c501a05861e697399c6752343cd66c3a0 on 2026-10-06 after CI 37453601758 passed. Independent review and full local check, 202 tests, and build passed; main integration and native four-platform qualification remain pending before publication.

The Linux legacy smoke upgraded actual development 0.8.0 `07b4bba` to
0.9.0 `cdfa842` and verified a byte-stable no-op. Final qualification must use
the subsequent exact main/development candidate and include all four platforms.

## Verified Production Completion

PR #83 merged into main at 7a289ab9edcb8799c295041b2fab63a79ba233fa; actual main CI 37454486476 and development CI 37454491771 passed. Qualification 37455327437 passed current-pair updates, actual legacy 0.8.0 updates, and modern-bootstrap rejection on Linux, macOS Intel/ARM, and Windows while production remained held. Release Artifacts 37454486538 then published verified SKM 0.9.0 through the normal eligible-reviewer approval path on 2026-10-06; all nine assets, manifest/source identity, checksums, Linux binary identity, and repeat-update stability were verified.

Both ordinary and supplemental legacy paths passed on all four native runners. Every modern candidate artifact was rejected by the specific legacy-mode guard before update. The qualification workflow revalidated exact branch/tag identity and pending production before normal approval.

Memory is updated in `workspace/agents/memory/facts.md` and
`workspace/agents/memory/changelog.md`. Applied version 0.9.0 is now released;
no duplicate bump is introduced. Earlier local/pending statements describe
historical snapshots. These completion records receive fresh independent review
and frozen aggregate verification before their record-only main closeout.
