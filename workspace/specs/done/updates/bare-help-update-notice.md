# Bare invocation update notice

State: `done`

Rationale: Verified main 7a289ab, branch CI 37454486476/37454491771, four-platform current/legacy qualification 37455327437, and protected production 0.9.0 publication 37454486538. Acceptance and records are reconciled.
Primary feature: `updates`

## Problem

Bare `skm` displays help but skips the startup update notice shown by `skm help`.
Clap returns `DisplayHelpOnMissingArgumentOrSubcommand` for the bare invocation,
while the notice hook only recognizes `DisplayHelp`.

## Scope

Run the existing startup notice hook for bare root help. Preserve Clap output,
exit status, terminal gating, opt-out behavior, and explicit help handling.
Do not add notices to invalid arguments or incomplete subcommands.

## Acceptance criteria

- Bare `skm` invokes the same notice hook as explicit help before rendering help.
- Clap's help text, stderr destination, and exit status remain unchanged.
- Invalid options, incomplete subcommands, and version requests skip the hook.
- The existing updater controls managed-build eligibility and opt-out behavior.

## Affected areas

- `src/main.rs`: parsing hook and deterministic regression tests.
- `README.md`: document bare invocation notice behavior.
- Spec catalog and shared release membership.

## Validation gates

- Demonstrate the new bare invocation regression fails before the correction.
- `task check`, `task test`, and `task versions:check`.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | patch | skm-next | Compatible notice fix sharing the already-applied 0.9.0 candidate; no second bump. |

## Memory Impact

Status: updated
Rationale: Bare root help shares explicit help's startup notice contract.
Memory: `workspace/agents/memory/decisions.md` and
`workspace/agents/memory/changelog.md`.

## Regression evidence

Before the correction, `task test` failed only the new bare invocation notice
test (148 other unit tests passed). The parsing hook now recognizes the bare
root help case while preserving Clap's error kind, text, destination, and code.

After the correction, `task check` and `task test` passed, including all 149
unit tests, 13 integration tests, and 21 documentation-validator tests.

## Delivery

Local implementation only; no confirmed test-channel integration or production
release. Reuses the documented shared candidate without claiming a new target.

## Confirmed Test Integration

PR #74 merged the implementation into `development` at
`e652af1122f54570c7dd189697730d6ac901cdaf` on 2026-09-30. Current development
contains that integration. This record corrects the stale local-only lifecycle;
main integration and production publication are not inferred.

## Verified Production Completion

PR #83 merged into main at 7a289ab9edcb8799c295041b2fab63a79ba233fa; actual main CI 37454486476 and development CI 37454491771 passed. Qualification 37455327437 passed current-pair updates, actual legacy 0.8.0 updates, and modern-bootstrap rejection on Linux, macOS Intel/ARM, and Windows while production remained held. Release Artifacts 37454486538 then published verified SKM 0.9.0 through the normal eligible-reviewer approval path on 2026-10-06; all nine assets, manifest/source identity, checksums, Linux binary identity, and repeat-update stability were verified.

The bare_invocation_notifies_without_changing_clap_help_behavior, help-request, and invalid-argument parsing regressions pass in the 202-test aggregate. Released help and managed updater behavior match the documented contract.

Memory is updated in `workspace/agents/memory/facts.md` and
`workspace/agents/memory/changelog.md`. Applied version 0.9.0 is now released;
no duplicate bump is introduced. Earlier local/pending statements describe
historical snapshots. These completion records receive fresh independent review
and frozen aggregate verification before their record-only main closeout.
