# Specification: Skill Removal Conformance

## Status

State: test

The removal command's safety and partial-success corrections were merged into
the configured `development` integration branch through PR #2 on 2026-09-05 as
merge commit `d0400bb`; they have not been released through `main`.

## Problem

`skm remove` currently unlinks agent targets before confirmation. A declined
prompt can therefore mutate the project, and one unlink failure stops later
agents instead of producing the specified partial-success report.

## Scope

- Preflight every configured agent target before mutation.
- Confirm before changing either `skills.yaml` or agent targets.
- Keep dry-run and already-removed behavior non-mutating and idempotent.
- Refuse unexpected links and non-symlink paths unless `--force` is explicit.
- Continue across per-agent unlink failures and report each failed target.

## Acceptance Criteria

- Declining confirmation leaves configuration and every agent target unchanged.
- Dry-run leaves configuration and every agent target unchanged.
- Unexpected links and non-symlink targets fail before writes without `--force`.
- Confirmed removal updates configuration and removes all removable targets.
- Per-agent failures are reported after all targets have been attempted.
- Focused tests cover cancellation, dry-run, target safety, success, and
  idempotency.

## Affected Areas

- `src/remover.rs`
- `src/linker.rs`
- `workspace/specs/README.md`
- `workspace/agents/memory/`

## Validation Gates

- `task check`
- `task test`
- `git diff --check`

## Risks

- `--force` is destructive by design; its target set must remain limited to
  the configured skill paths.
- Configuration can be removed while an operating-system unlink fails. That
  outcome must be explicit and must not hide successfully processed targets.

## Memory Impact

Status: `updated`

Rationale: Recorded the removal preflight, confirmation-ordering, and
partial-success behavior in `workspace/agents/memory/facts.md` and
`workspace/agents/memory/changelog.md`.

## Validation Result

- `task check`: passed formatting, warnings-free Clippy, compilation, and
  workspace documentation gates.
- `task test`: passed 66 Rust tests and 6 workspace-validator tests.
- Focused coverage confirms cancellation and dry-run preserve configuration
  and links, unexpected links are rejected without `--force`, removal is
  idempotent, forced non-symlink removal remains scoped to the configured
  target, and later targets are attempted after a per-target failure.
- Review added rejection of symlinked namespace parents so nested skill names
  cannot redirect link or unlink mutations outside the selected skills root.
