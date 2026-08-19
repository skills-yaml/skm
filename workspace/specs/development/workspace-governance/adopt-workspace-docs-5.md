# Specification: Adopt workspace-docs@5.0.0

## Status

State: development

Version update from the committed `workspace-docs@1.0.0` 1.x layout to
`workspace-docs@5.0.0`. Local implementation is active on the current feature
branch.

## Problem

This repository adopted `workspace-docs@1.0.0` with `docs/tech/`,
`docs/standards/`, `docs/specs/`, and `agents/memory/`. The target 5.0.0
contract uses a single `workspace/` container, a four-state spec lifecycle
with a root catalog, and memory-impact plus spec-catalog completion gates.

## Scope

- Upgrade the current-project workspace through the required 2.1, 3.0, 4.0,
  and 5.0 contracts. Versions 1.1.0 and 1.2.0 have no `migration.md`; they
  are additive 1.x pins only.
- Preserve unrelated and pre-existing worktree changes, including the SKM
  0.2.1 toolkit compatibility work and root `DESIGN.md`.
- Do not restore the deleted `docs/specs/foundation/ui_brand_and_style.md`.
- Do not change SKM product, authentication, data-flow, infrastructure, or
  CI/CD behavior except Taskfile gates required by the target standard.
- Pin generated policy and migration records to `workspace-docs@5.0.0`.

## Migration Mode

- Mode: version update (mixed 1.x layout)
- Previous version: `workspace-docs@1.0.0` (committed pin and inventory)
- Detected AGENTS.md token before this migration: `workspace-docs@1.2.0`
  (uncommitted retag without a 1.2 package or migration notes)
- Target version: `workspace-docs@5.0.0`
- Trusted source: canonical `skills-yaml/workspace` package at commit
  `a934c9fbf8c0506eff018cb5c7bedeec214c1d58`, installed at
  `workspace/instructions/standards/workspace-docs`

## Acceptance Criteria

- Root `AGENTS.md`, `DESIGN.md`, and `README.md` remain at the repository root.
- Required 5.0.0 workspace directories and memory files exist.
- One balanced `AGENT-CONTEXT` block is pinned to `workspace-docs@5.0.0`.
- Manual `AGENTS.md` policy outside that block is preserved, with only
  path updates for moved files.
- Every non-legacy spec lives at
  `workspace/specs/<state>/<primary-feature>/<spec>.md`.
- Every non-legacy spec has one catalog row with state and a non-empty
  rationale.
- Done specs are limited to work confirmed released through `main`.
- Development specs remain in development until integration into
  `development` is confirmed.
- Memory impact is classified `updated` or `none` with a rationale.
- `task check` and `task test` pass, including deterministic workspace
  structure, catalog, memory-impact, and privacy gates.
- No secrets, machine-local paths, or sibling-project context remain in
  current-project documentation or memory.

## Source / Destination / Merge Map

| Source | Destination | Merge decision |
| --- | --- | --- |
| `AGENTS.md` generated block | `AGENTS.md` generated block | Replace only the generated block with the 5.0.0 template. |
| `AGENTS.md` manual rules | `AGENTS.md` manual rules | Preserve byte-for-byte except repository-relative path updates. |
| `DESIGN.md` | `DESIGN.md` | Keep the pre-existing root file. Do not overwrite. |
| `README.md` | `README.md` | Keep. Update only links that break after moves. |
| `agents/memory/*` | `workspace/agents/memory/` | Move. Remove the sibling-repo absolute path from `README.md`. |
| `docs/tech/*` | `workspace/instructions/tech/` | Move. Update internal path references. |
| `docs/standards/workspace-docs/README.md` | `workspace/instructions/standards/workspace-docs/` | Do not copy. The pin file records a sibling absolute path. Install the complete trusted 5.0.0 package instead. |
| `docs/specs/backlog/` | `workspace/specs/backlog/` | Empty besides `.gitkeep`. Recreate as reserved state. |
| `docs/specs/development/workspace-docs-5-compatibility.md` | `workspace/specs/development/workspace-toolkit/workspace-docs-5-compatibility.md` | Move. Keep `development`; no `development` or `main` integration evidence. |
| `docs/specs/done/workspace-toolkit-manager.md` | `workspace/specs/development/workspace-toolkit/workspace-toolkit-manager.md` | Move to development. Present on this feature branch only; not released through `main`. |
| `docs/specs/done/auto-update-notification.md` | `workspace/specs/done/updates/auto-update-notification.md` | Move. Present on `main`. |
| `docs/specs/done/cleanup-commands.md` | `workspace/specs/done/maintenance/cleanup-commands.md` | Move. Present on `main`. |
| `docs/specs/done/config-management.md` | `workspace/specs/done/configuration/config-management.md` | Move. Present on `main`. |
| `docs/specs/done/global-env-auto-config.md` | `workspace/specs/done/configuration/global-env-auto-config.md` | Move. Present on `main`. |
| `docs/specs/done/local-dev-mode.md` | `workspace/specs/done/local-dev/local-dev-mode.md` | Move. Present on `main`. |
| `docs/specs/done/registry-management.md` | `workspace/specs/done/registry/registry-management.md` | Move. Present on `main`. |
| `docs/specs/done/skill-removal.md` | `workspace/specs/done/skill-lifecycle/skill-removal.md` | Move. Present on `main`. |
| `docs/specs/done/skill-version-management.md` | `workspace/specs/done/skill-lifecycle/skill-version-management.md` | Move. Present on `main`. |
| `docs/specs/foundation/product.md` | `workspace/specs/legacy/product.md` | Move historical current-project spec. |
| `docs/specs/foundation/ui_brand_and_style.md` | none | Pre-existing deletion. Do not restore. |
| `docs/specs/README.md` | `workspace/specs/README.md` | Replace with the 5.0.0 catalog template plus this repository's features. |
| `docs/analysis/feature-audit.md` | `workspace/docs/work/feature-audit.md` | Move work notes. |
| `docs/projects/skm/inventory.md` | `workspace/docs/work/adoption-inventory.md` | Move current-project inventory. Do not keep `workspace/docs/projects/`. Update paths. |
| reserved company and instruction dirs | `workspace/company/**`, `workspace/instructions/agents/`, `workspace/instructions/skills/` | Create reserved READMEs. Populate `adopt-workspace-structure` from the trusted package. |
| none | `scripts/validate_workspace.py` | Add the 5.0 memory-impact, spec-catalog, structure, and privacy gates. |
| none | `skills.yaml` | Add a project pin for `workspace-docs@5.0.0` so SKM workspace commands can audit the local package. No skills are installed. |

## Affected Areas

- `AGENTS.md`
- `workspace/`
- `docs/` (legacy 1.x locations, removed after verified moves)
- `agents/memory/` (moved)
- `Taskfile.yml`
- `scripts/validate_workspace.py`
- `scripts/test_validate_workspace.py`
- `skills.yaml`
- `.gitignore`
- `workspace/agents/memory/`

Product runtime, linker, toolkit install semantics, and GitHub workflow
behavior are out of scope except Taskfile aggregation of the new gates.

## Validation Gates

- `task check`
- `task test`
- `task build`
- `git diff --check`
- `skm workspace audit --target workspace-docs@5.0.0` after the local package
  is installed
- 5.0.0 manifest and audit checklist comparison

## Risks

- Collision: destinations under `workspace/` are new. Occupied destinations
  stop the move.
- Privacy: the 1.0 pin and memory README contain a sibling absolute path.
  Those strings must not be copied forward.
- Compatibility: SKM still recognizes `docs/standards/workspace-docs` as a
  consumer fallback. That product path stays in Rust source; this repository
  must not keep a second live standard there.
- Rollback: restore only paths changed by this migration with git. Do not
  discard unrelated worktree changes.
- SKM 0.2.1 requires `v<version>/migration.md` for every version after the
  detected pin. 1.1.0 and 1.2.0 have no such notes, so a 1.0.0-detected chain
  cannot be verified by SKM. Audit uses the complete 5.0.0 package and the
  2.0.0-through-5.0.0 notes required by `AGENT_MIGRATION.md`.

## Memory Impact

Status: `updated`

Rationale: Recorded the 5.0.0 adoption, lifecycle targets, and canonical paths
in `workspace/agents/memory/decisions.md`, `workspace/agents/memory/facts.md`,
and `workspace/agents/memory/changelog.md`.

## Validation Result

- `python3 -B scripts/validate_workspace.py`: passed structure, specs, memory, and privacy gates.
- `python3 -B scripts/test_validate_workspace.py`: passed 6 tests, including missing `test` state, missing feature grouping, pending done-spec memory impact, missing catalog row, and machine-local path failures.
- `task check`: passed formatting, Clippy, compilation, and workspace gates.
- `task test`: passed 52 cargo tests and 6 workspace validator tests.
- `task build`: produced the locked release build.
- `git diff --check`: passed.
- `skm workspace audit --target workspace-docs@5.0.0`: verified the local package at `workspace/instructions/standards/workspace-docs` with integrity `sha256:502be11e039435eb6691fab6cb0bb3021f10eb7b397baffd45e9df0d64022f41`. After the 5.0.0 pin was written, SKM reported `5.0.0 -> 5.0.0` with an empty remaining chain.
- `skm workspace repair --apply --yes --target workspace-docs@5.0.0`: wrote `.skm/workspace-plan.yaml`.
- 5.0.0 manifest required files and directories are present.
- Manual `AGENTS.md` policy outside the generated block was preserved except repository-relative path updates.

Justified deviations:

- SKM cannot verify a 1.0.0-to-5.0.0 chain because `v1.1.0` and `v1.2.0` have no `migration.md`. Intermediate notes applied were 2.0.0, 2.1.0, 3.0.0, 4.0.0, and 5.0.0 as required by `AGENT_MIGRATION.md`.
- This repository has no `develop` branch. The documented test target is `development`.
- Example command output in released specs used `$HOME` placeholders so the privacy gate can reject real machine-local paths.
