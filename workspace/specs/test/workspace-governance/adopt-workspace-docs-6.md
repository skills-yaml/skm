# Adopt Workspace Docs 6.0.0

## Status

State: `test`

Rationale: PR #72 merged the Workspace Docs 6.0.0 migration into the configured `development` test channel at `c49f1cc` on 2026-09-30 after Validate run `36682939489` passed. Production release through `main` is pending.

## Problem and Scope

This Rust CLI repository is pinned to `workspace-docs@5.0.0`. The selected 6.0.0 candidate adds concurrent-agent worktree records and a portable peer runtime, per-spec version impact, a shared release reservation ledger, and native validation gates. The project must adopt those contracts without changing SKM skill management behavior or discarding personal workspace files.

- Migration mode: version update.
- Previous version: `workspace-docs@5.0.0`.
- Target version: `workspace-docs@6.0.0` (unreleased candidate, explicitly selected by the user).
- Trusted source: the 6.0.0 package bundled with the user-invoked `adopt-workspace-structure` skill. The repository's existing standard tree contains released versions through 5.0.0 and is retained.
- Configured test target: `development`; production target: `main`.
- Product runtime, authentication, data flow, release publication, and CI workflow behavior are out of scope. Taskfile validation entrypoints and the SKM version metadata required by the new reservation contract are in scope.

## Confirmed Project Choices

The user confirmed retaining `src/`, `tests/`, and `scripts/` as the Rust CLI layout; retaining and adapting the existing Taskfile, CI, task, SDLC, and project-structure guides; and adding no backend, database, frontend, or infrastructure guides. Evidence is `Cargo.toml`, the Rust source tree, `Taskfile.yml`, `.github/workflows/`, and the current guides under `workspace/instructions/tech/`. The user authorized adding the 6.0.0 Workspace pin to the working `skills.yaml` while preserving its personal skill entries. The pre-existing personal agent links and `skills.lock.yaml` remain outside migration ownership.

## Acceptance Criteria

1. The complete 6.0.0 candidate package is present alongside immutable prior versions. `default` and `latest` remain on the released 5.0.0 version until 6.0.0 is published; the project pin and generated context identify 6.0.0.
2. Manual `AGENTS.md` policy remains byte-for-byte intact outside the one generated context block. Existing instructions, specs, docs, and memory retain one active canonical location.
3. The repository has a `workspace/docs/work/multi-agent/` home for records, a portable coordination skill/runtime, and a deterministic `task coordination:check` gate. Existing worktrees are not attached, moved, or deleted.
4. Every non-legacy spec has the 6.0.0 Version Impact table. Historical done specs are frozen without invented release numbers. Active specs share an evidence-backed SKM candidate reservation where they affect one release.
5. `workspace/releases.json` has collision-free ownership, current member paths, a real released baseline and candidate target, applied timing evidence, and a deterministic `task versions:check` gate in `task check`.
6. Project guides and repository references describe the 6.0.0 contract accurately without changing Rust application behavior. Personal skill entries and local links survive.
7. `task check`, `task test`, focused validator failure tests, and `git diff --check` pass. Relative links, structure, privacy, and the final diff are reviewed.

## Source / Destination / Merge Map

| Source | Destination | Merge decision |
| --- | --- | --- |
| Existing `workspace/instructions/standards/workspace-docs/` | Same tree plus `v6.0.0/` | Preserve released versions and aliases; add 6.0.0 files and update unversioned package index/versioning notes only where needed. |
| Existing generated `AGENTS.md` block | Generated block in `AGENTS.md` | Replace with the exact 6.0.0 template; preserve all manual policy outside the markers. |
| `workspace/instructions/skills/adopt-workspace-structure/` | Same path | Adapt the repository-owned skill to the selected 6.0.0 migration procedure; retain project-local authority boundaries. |
| Bundled coordination skill and runtime | `workspace/instructions/skills/coordinate-multi-agent-development/` | Add portable skill instructions and runtime without copying local operational board state. |
| `workspace/docs/work/` | `workspace/docs/work/multi-agent/` | Add a record home and validation rules; retain existing work notes. No historical assignments are invented. |
| Current non-legacy specs and `workspace/specs/README.md` | Same lifecycle paths and catalog | Add Version Impact sections; leave states and evidence unchanged. Add this migration spec and catalog row. |
| Current SKM 0.8.0 released baseline and integrated candidate work | `workspace/releases.json`, `Cargo.toml`, `Cargo.lock` | Reserve and apply one shared 0.9.0 candidate for affected active specs, subject to occupancy checks. Preserve released history. |
| Existing `scripts/validate_workspace.py` and tests | Same scripts plus focused 6.0 validators/tests | Retain structure, catalog, memory, and privacy checks; add version and coordination checks. |
| Existing `Taskfile.yml` | Same file | Add `versions:check` and `coordination:check` to the authoritative `check` aggregate; preserve build/test semantics. |
| None | `workspace/validation/peer-commands.json` | Register aggregate Taskfile commands as the conservative peer integration gate; focused evidence reuse remains disabled. |
| Existing project guides, memory, and `skills.yaml` | Same paths | Update 6.0 pin/policy references, record durable adoption decision and evidence, preserve unrelated project content and personal skill entries. |

## Risks and Rollback

- 6.0.0 is a draft candidate. Do not move `default` or `latest` from 5.0.0 or claim a production release.
- The local `skills.yaml` and agent links contain pre-existing personal changes. Merge only the authorized Workspace pin into that manifest; do not stage or publish personal entries or generated lock data.
- A shared SKM candidate version must be checked against current release state before it is reserved. If occupied by unrelated work, reconcile the release rather than picking an arbitrary number.
- Existing linked worktrees may contain unique work. Preserve them, and do not initialize a peer board from an unreviewed or dirty base.
- Rollback reverts only migration-owned files and version metadata. Preserve personal files, worktrees, and any peer recovery state.

## Validation Gates

- Compare the final tree and generated context against the 6.0.0 manifest and checklist.
- Run `task versions:check`, `task coordination:check`, `task check`, `task test`, and `git diff --check`.
- Exercise failure cases for missing version sections, reservation collisions/source drift, malformed coordination records, and privacy violations.
- Validate current-document relative links and search for stale active 5.0.0 pin references.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | minor | skm-next | The repository's contributor and validation contract changes incompatibly under the selected 6.0.0 candidate; SKM is pre-1.0 and shares the next candidate with active CLI changes. |

## Memory Impact

Status: `updated`

Rationale: The adoption decision is recorded in `workspace/agents/memory/decisions.md`, the SKM 0.9.0 candidate in `workspace/agents/memory/facts.md`, and both in `workspace/agents/memory/changelog.md`.

## Validation Result

- `task workspace:check`, `task versions:check`, and `task coordination:check` passed on the local migration candidate.
- `task check` passed Rust formatting, Clippy with warnings denied, compilation, and all Workspace Docs gates.
- `task test` passed 148 Rust unit tests, 13 Rust integration tests, and 21 Python validator tests.
- After `development` advanced to `b90a107`, the newly integrated Registry notification spec received a justified `none` impact row. The combined tree passed `task check` and `task test`, and the release reservation still has five active member specs.
- `task peers -- --help` confirmed the portable runtime wrapper is callable without initializing an operational board.
- The 2026-09-29 `prod-latest` and `development-latest` release manifests both reported SKM 0.8.0; the remote had no `v0.9.0` tag. The candidate uses one 0.9.0 reservation from that released baseline.
- The generated `AGENTS.md` block exactly matches the 6.0.0 template, and its manual prefix and suffix match the previous committed policy. The copied 6.0.0 package and peer runtime match their trusted sources; current-document relative links passed inspection.
- `git diff --cached --check` passed. The staged `skills.yaml` change contains only the 5.0.0-to-6.0.0 pin, while the working file retains personal skill entries outside the migration diff.
- The standard aliases remain on released 5.0.0 because 6.0.0 is still draft. No peer board or historical assignment was created, and existing linked worktrees were not changed.
- PR #72 passed Validate run `36682939489` and merged into `development` at `c49f1cc47a477d92c2923677d49108ff222e9b01` on 2026-09-30; this confirms the `test` transition.

The peer runtime's bundled file-locking implementation uses Unix `fcntl`; Windows peer coordination has not been qualified. SKM's Windows CLI behavior is outside this migration and remains covered by its existing tests.

The spec is in `test` after confirmed integration into the configured `development` test target. A production release and `done` transition remain pending.
