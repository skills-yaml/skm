# Specification: Skill Removal Conformance

## Status

State: done

The removal command's safety and partial-success corrections were integrated
through PR #2, then released through `main` by PR #4 on 2026-09-05 as merge
commit `e12ba7d`. The `prod-latest` release artifacts were published
successfully.

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
- Production evidence: PR #4 merged the validated `development` branch into
  `main` as `e12ba7d`; main CI and Release Artifacts run `33967911337` passed
  and published the `prod-latest` assets.
