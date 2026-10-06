# Reconcile Workspace toolkit 0.7.2 governance

## Status

State: `test`

Rationale: PR #85 merged the reviewed Workspace toolkit 0.7.2 guidance into development at c145429164e0e2dc2c798eaf5a5a40d98d1fa157 on 2026-10-06 after PR CI run 37515809249 passed. Fresh task check and task test passed on the actual combined revision (202 tests). Main completion is not claimed for this follow-up.

## Problem and Scope

Mode: repair of an existing Workspace Docs 7.0.0 adoption after the installed
Workspace toolkit changed from 0.7.1 to 0.7.2. Follow up the
[Workspace Docs 7 adoption](../../done/workspace-governance/adopt-workspace-docs-7.md); preserve its history.
The named upgrade request authorizes only the necessary governed guidance edits.

Canonical source: `skills-yaml/workspace` revision
`f9959adb462c524fd91b31288d5c35f54b2a801e`. Its toolkit manifest selects
0.7.2 and its standard aliases select 7.0.0. All standard files already match
that revision; all twenty working skill pins and installed packages already
select the toolkit's versions. Preserve released standard packages, generated
and manual AGENTS policy, runtime, CI, Cargo versions, user staging, personal
configuration, installation lockfile, and installed caches/links.

## Confirmed Project Choices

Retain the previously confirmed Rust CLI layout (`src/`, `tests/`, `scripts/`),
Taskfile entrypoints, and current task/CI/project-structure guides. Adapt only
existing SDLC and validation guidance; add no application directories or
frontend, backend, database, or infrastructure guides. Evidence: `Cargo.toml`,
`Taskfile.yml`, current technical guides, and the linked adoption spec.

## Acceptance Criteria

1. Workspace Docs remains 7.0.0 with matching canonical package files and
   contained aliases; toolkit and all twenty installed pins match 0.7.2.
2. Repository-owned adoption and coordination instructions, SDLC guidance,
   and validation module guidance define changed-component checks followed by
   direct/transitive consumer checks in dependency order. Failures block
   downstream stages; missing coverage is a gap. Deduplicate overlaps, group
   cycles, assess old/new relationships for renames and deletions, and justify
   safe fallback for unknown impact.
3. Full required checks after independent review, existing native protections,
   final evidence freshness rules, and contract/integration/end-to-end coverage
   remain mandatory. No runtime-specific Workspace knowledge is introduced.
4. Pre-existing staged content, personal manifest/lockfile, manual AGENTS policy,
   historical standard packages, and unrelated changes remain unchanged.
5. The catalog and durable memory record this bounded follow-up; lifecycle
   transitions require actual shared-test/main evidence.

## Source / Destination / Merge Map

| Source | Destination | Merge decision |
| --- | --- | --- |
| Installed 0.7.2 focused skills' Pinned SDLC Contract | Repository-owned adoption/coordination SKILL.md files | Add only ordered validation guidance; preserve local routing and runtime instructions. |
| Same ordered validation contract | `workspace/instructions/tech/sdlc.md` | Adapt the existing review/verification section; retain confirmed project facts. |
| Same contract and current Taskfile module dependencies | `workspace/validation/README.md` | Document behavioral impact mapping and staged execution; retain module commands and final aggregate requirements. |
| New follow-up spec | This development spec and `workspace/specs/README.md` | Register one workspace-governance row; preserve every unrelated row and state. |
| Durable validation decision | `workspace/agents/memory/decisions.md` and `changelog.md` | Append concise records without rewriting history. |
| Confirmed shared-test integration | `workspace/agents/memory/facts.md`, changelog, this spec, and catalog | Append actual merge/validation evidence and move this follow-up to test; preserve main-completion history. |
| Canonical standard and toolkit manifest | Existing standard packages, aliases, working skill pins, installed links | Verify in place; no replacement or downgrade. |

## Implementation Plan

1. Record the complete contract and catalog row before governed edits.
2. Merge the ordered validation guidance into the four current instruction/docs files.
3. Validate changed guidance by inspection; run Workspace gates first, then
   version/coordination consumers after their prerequisites pass.
4. Reconcile acceptance and memory; obtain independent review of the exact
   bounded candidate and resolve findings.
5. Freeze the candidate and run `task check` and `task test`. Preserve current
   staging and installation fingerprints. Record actual delivery separately.

## Validation Gates and Risks

Changed component: governance documentation; first inspect consistency with
the installed 0.7.2 contract and run `task workspace:check` / `workspace:test`.
Direct consumers: categorized specs/memory and the version/coordination gates;
run their checks/tests after Workspace passes. Final transitive coverage:
`task check` and `task test`, including Rust installation contract tests.
No new mirrored tests are needed for prose-only changes; existing validators
exercise the required preserved contracts. Independent exact-candidate review
is mandatory for governed instructions. Relevant edits invalidate evidence.

Risks: copying publisher project facts, changing historical version packages,
or overwriting personal staging. Mitigate with bounded merges, canonical blob
comparison, and before/after fingerprints. Keep rollback snapshots local;
never record machine paths or raw logs. Do not clear installed registry caches
to resolve an unrelated source-origin mismatch.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | none | none | Prose-only repository governance reconciliation changes no SKM runtime or installation format. Workspace's already-published 0.7.2 toolkit owns its external skill versions. |

## Memory Impact

Status: `updated`

Rationale: The ordered validation contract is established in current guidance and recorded in `workspace/agents/memory/decisions.md` plus `workspace/agents/memory/changelog.md`. SKM runtime ownership and historical version packages are preserved.

## Acceptance and Delivery Evidence

- AC-1: Canonical revision comparison matches all ten 7.0.0 package blobs,
  unversioned indexes/migration guide, and both aliases. The toolkit manifest
  selects 0.7.2; all twenty working pins and installed skill metadata agree.
  `skm check` passes and `skm install --dry-run` reports no changes needed.
- AC-2/3: Scoped inspection confirms changed-component-first execution,
  failure-blocked consumer stages, transitive dependency ordering, cycle and
  rename/deletion handling, justified fallback, and retained full/native gates.
  `task workspace:check` and `task workspace:test` passed first; then version
  and coordination checks/tests passed as independent consumer modules.
- AC-4: The primary checkout was switched to `development` after its full
  original state was captured in a local recovery stash/archive. Both original
  staged file contents, both corresponding working files, and the personal
  installation lockfile were restored and byte-compared successfully. Prior
  copied implementation and stale lifecycle records are preserved in recovery;
  the checkout now uses verified shared integration records.
- AC-5: The spec/catalog are in test with confirmed PR #85 integration evidence;
  the durable validation decision and integration fact have changelog records.
  Independent review approved exact implementation candidate
  `6fc37901d26ceb20cb739b9f7f708d99f9ddd9fa` with no findings. Frozen checks/tests
  passed on that candidate and again on actual integration `c145429`.
  Record-only delivery receives renewed review and complete final verification;
  its results are reported at handoff. Relevant edits renew their evidence.

Registry refresh refused the existing cached-origin mismatch. No cache or
configuration was rewritten; live canonical source and installed metadata
establish the requested versions without treating failed refresh as a pass.

Shared-test integration is established by PR #85 and fresh combined-revision
verification. Main merge and production publication are not claimed for this
follow-up. SKM has no version bump because this change affects only governance
guidance and records. Workspace toolkit publication remains external.
