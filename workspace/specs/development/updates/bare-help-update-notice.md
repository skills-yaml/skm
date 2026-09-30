# Bare invocation update notice

State: development
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
