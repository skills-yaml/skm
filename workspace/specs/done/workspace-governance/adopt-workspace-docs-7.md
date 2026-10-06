# Adopt Workspace Docs 7.0.0

## Status

State: `done`

Rationale: PR #80 merged the confirmed development implementation into main at 79658625c5bb30cd84b0ad5185237e28e20038eb on 2026-10-06; actual main CI run 37451646628 passed task check, all 202 tests, and task build. Acceptance, documentation, catalog, applied SKM 0.9.0, and memory are reconciled. Production publication remains separately subject to qualification and human release-prod approval.

## Problem and Scope

- Mode: version update from `workspace-docs@6.0.0` to `workspace-docs@7.0.0`.
- Authority: the user's named migration request prospectively authorizes the governed instruction changes necessary for this migration.
- Trusted source: the complete released 7.0.0 package bundled with the installed `adopt-workspace-structure@0.6.1` skill, selected because the repository's standard tree lacks the requested version. Preserve all existing version directories.
- Scope: Workspace standard, generated context, technical workflow guidance, repository-owned adoption/coordination instructions, native validators and tests, Taskfile modules, catalog, project pin, release membership, and durable memory.
- Boundaries: SKM runtime, toolkit/bundle compatibility allowlists, authentication, release publication, CI workflow behavior, and application layout are unchanged. A separate follow-up may add runtime 7.x compatibility.
- Shared test target: `development`. Completion target: `main`. Publication is tracked separately. Use the shared-test route; no direct route is introduced.

## Confirmed Project Choices and Evidence

Retain the previously user-confirmed Rust CLI layout (`src/`, `tests/`, `scripts/`) and Taskfile interface. Evidence: `Cargo.toml`, `workspace/instructions/tech/project_structure.md`, and the prior 6.0 adoption spec. Retain/adapt task, SDLC, CI, and project-structure guides; add no frontend, backend, database, or infrastructure guide. Preserve `DESIGN.md` and all manual `AGENTS.md` bytes. Preserve pre-existing staged changes to `.gitignore` and `skills.yaml`, working personal skill pins, agent links, and untracked `skills.lock.yaml`; update only the working Workspace standard pin.

## Acceptance Criteria

1. The complete trusted 7.0.0 package exists with contained relative `default` and `latest` symlinks selecting released 7.0.0. Prior version directories remain byte-for-byte unchanged.
2. One generated `AGENTS.md` block exactly matches 7.0.0; manual policy and Rust/application/CI behavior remain unchanged.
3. The blocked state has feature/catalog coverage and validated Previous State, Block Kind, Block Reason, and Resume Condition. Resumption retains the prior stage and renews stale evidence.
4. Current workflow guidance defines done by verified main merge and reconciled acceptance/records, separates publication, requires independent review for governed instructions, and preserves host protections. Historical done specs and explicit older contracts remain intact. Existing test specs remain in test without invented completion evidence.
5. Version validation includes blocked specs, requires applied versions for test/done, and treats publication separately. Reuse the existing applied `skm-next` 0.9.0 target without another bump; synchronize member paths.
6. Taskfile modules document their inputs, dependencies, consumers, isolation, pass conditions, and freshness; positive and negative validator tests cover changed contracts. Unknown scope uses complete aggregate gates; final evidence reuse stays disabled.
7. Independent exact-candidate review resolves all findings before `task check` and `task test`. Structure, catalog, memory, versions, coordination, relative links, privacy, preservation, and diff checks pass on the stable candidate.
8. Record the durable adoption decision and changelog, reconcile acceptance evidence, and keep this spec in development until confirmed shared-test integration. Report the separate runtime compatibility gap and publication status.

## Source / Destination / Merge Map

| Source | Destination | Merge decision |
| --- | --- | --- |
| Bundled `references/workspace-docs/v7.0.0/` | `workspace/instructions/standards/workspace-docs/v7.0.0/` | Add every regular package file; stop on an occupied destination. |
| Bundled standard README, migration guide, and VERSIONING | Same unversioned files under the repository standard root | Replace package-level indexes with trusted 7.0 guidance; preserve every existing version directory. |
| Repository standard `default` and `latest` | Same aliases | Replace only verified relative links with `v7.0.0`; never copy bundled text pointers as alias files. |
| Generated `AGENTS.md` block | Same block | Use exact target template; preserve manual prefix/suffix bytes. |
| Task, SDLC, CI, project-structure, and standard/Workspace indexes | Same current paths | Adapt only migration-governed contracts; preserve product and release workflow facts. |
| Repository-owned adoption and coordination skill instructions/profiles | Same paths | Reconcile local policy to v7; retain the existing detached runtime implementation and conservative full integration gates. Do not modify installed skill caches. |
| No blocked directory | `workspace/specs/blocked/README.md` | Add canonical state guidance; do not move unrelated specs. |
| Current spec catalog/state READMEs | Same paths | Describe v7, register this spec, preserve historical rows and test states. Explicit older migration specs keep their original completion contract. |
| Native Workspace, version, coordination validators and tests | Same scripts | Add blocked metadata and separate main completion/publication semantics with regression tests. |
| `Taskfile.yml` and validation guidance | Same Taskfile plus `workspace/validation/README.md` | Add focused script-test modules and declare conservative module contracts; preserve all aggregate coverage. |
| `skills.yaml` Workspace pin | Same working file | Change only `workspace-docs@6.0.0` to `workspace-docs@7.0.0`; preserve personal entries and staged baseline. |
| Applied `skm-next` reservation | `workspace/releases.json` | Add this migration as a minor-impact member; reuse applied 0.9.0 and preserve owner, baseline, and prior evidence. |
| Durable memory | `workspace/agents/memory/decisions.md` and `changelog.md` | Append the v7 decision superseding the v6 adoption decision; preserve historical entries. |

## Implementation Plan

1. Register this complete contract, catalog row, and release membership before other migration edits.
2. Add the trusted package, generated context, pin, blocked directory, and reconciled guides/skills.
3. Update native validators and focused failure tests; exercise affected Taskfile modules.
4. Reconcile knowable acceptance and memory; self-review preservation and links.
5. Obtain independent review of the exact candidate, resolve findings, freeze it, and run full Taskfile verification.
6. Handoff local implementation in development; delivery transitions require actual combined-revision evidence.

## Risks and Rollback

- Never overwrite an occupied package or rewrite historical standards/specs.
- Preserve personal manifest/index changes and generated local lock/link state.
- Existing worktrees and peer operational state remain untouched. Only one agent mutates; independent review is read-only.
- Reuse of the documented applied draft release is local and offline; remote occupancy/publication requires fresh verification before publishing.
- Restore only migration-owned changes and the standard pin if rollback is needed; preserve unrelated changes and unique work.
- SKM's toolkit/bundle runtime currently rejects 7.x; this governance migration does not imply runtime support.

## Validation Gates

- Target manifest/template/package equality, previous-version/manual-policy preservation, current relative links, privacy, and worktree ownership inspection.
- Focused Taskfile modules for Workspace, version, and coordination validation and their positive/negative regression suites.
- Independent review before final `task check`, `task test`, and `git diff --check`/staged diff checks.
- Final gate evidence binds to the unchanged candidate; tracked result edits renew affected review and verification. No evidence reuse is assumed.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | minor | skm-next | The contributor governance and native validation contract changes under v7; pre-1.0 SKM reuses its existing applied 0.9.0 shared candidate without another bump. |

## Memory Impact

Status: `updated`

Rationale: The adopted governance, preserved Rust layout, separate completion/publication contract, blocked metadata, review/gate obligations, and shared candidate reuse are recorded in `workspace/agents/memory/decisions.md` and `workspace/agents/memory/changelog.md`.

## Acceptance and Delivery Evidence

| Criterion | Knowable local evidence |
| --- | --- |
| 1 | Inspection compares every 7.0.0 package file byte-for-byte with the bundled trusted source, both aliases target contained `v7.0.0`, and all prior tracked version files match HEAD. |
| 2 | Generated template is validated natively; inspection proves manual prefix/suffix bytes match HEAD. No Rust, Cargo manifests/lock, CI workflow, or runtime file is changed. |
| 3 | Blocked directory and catalog guidance exist; positive/negative fixture cases cover resume metadata, catalog mismatch, and active memory obligations. No existing spec moves state. |
| 4 | Current technical guides and repository-owned skills follow v7; historical done files match HEAD and the older 6.0 test spec remains unchanged. |
| 5 | The existing applied `skm-next` 0.9.0 reservation adds this member without a bump; native versions check passes. Regression fixtures cover applied done, separate publication, and blocked timing. |
| 6 | `workspace/validation/README.md` declares Taskfile modules, inputs, consumers, isolation, and freshness. Full peer gate coverage remains configured; no inferred selector or receipt reuse is introduced. |
| 7 | Native structure, version, and coordination checks plus diff checks pass during iteration. Independent exact-candidate review approved the implementation with no findings; aggregate check/test evidence is recorded below and result-record changes renew affected review and verification. Current-document relative link inspection found no missing targets. |
| 8 | Memory is resolved to updated with category/changelog records. The index digest, normalized personal manifest bytes, local lockfile bytes, manual policy, old packages, and done history were preserved. |

Shared-test integration, main merge, and publication remain unconfirmed. The spec stays in development; historical 6.0 test integration retains its recorded older completion contract.

## Review and Final Verification

- Independent read-only agent review approved implementation snapshot SHA256 `dc47481255e9593cf9d0d060bd4ebf5981a60252c2be207ff7aa1e59dcc5039d` with no actionable findings. Package equality, historical/manual/personal preservation, governance coherence, native gates, and diffs were independently inspected.
- `task check` passed installer syntax/contract checks, Rust formatting, warnings-free all-target/all-feature Clippy, compilation, and all Workspace/version/coordination gates.
- `task test` passed 149 Rust unit tests, 13 Rust integration tests, and 35 Python validator tests (14 Workspace, 14 version, 7 coordination).
- Current-document relative links, working/staged diff checks, complete trusted package equality, manual policy, previous packages/done history, canonical staged content, personal manifest entries, and local lock bytes passed inspection.
- These knowable result/catalog records change the snapshot. Renew independent review for them, freeze the resulting candidate, and rerun `task check` and `task test` before handoff. The final reviewed snapshot and final repeated gate outcome are reported in the handoff; no tracked edit follows that run. Final evidence reuse is disabled.
- Snapshot recipe: sorted unique file paths from `git ls-files -z --cached --others --exclude-standard`; hash each repository-relative UTF-8 path, NUL, `L` plus symlink-target bytes or `F` plus file bytes, and NUL, into SHA256. This includes alias symlink targets and existing user files without staging/publishing them.
- Review found no deviations requiring fixes or rejection. Historical 6.0 migration contracts and SKM runtime 7.x compatibility remain outside the rewritten lifecycle/product scope. The existing detached peer runtime is preserved; Windows peer locking remains unqualified.

No contributor-only result proves shared-test integration or main completion. Publication remains unconfirmed; the applied SKM 0.9.0 reservation is reused without another bump.

## Linked Runtime Follow-up

The user's subsequent request is tracked in
[installation-contract-validation.md](../workspace-toolkit/installation-contract-validation.md).
It removes governance major-version interpretation while retaining installation
contract checks. The migration evidence above describes its earlier candidate;
this follow-up does not rewrite that history or infer integration.

## Delivery Baseline Reconciliation

The 2026-10-06 merge request authorizes delivery through the configured test and
main targets. The isolated delivery candidate merges current development
`2e18f4e97d42ea3b6cd2387d50db07495841a727`, preserving the independently shipped
0.8.1 compatibility history and updated shared-release baseline. Runtime fixes
that only enlarged the old allowlist are superseded by publisher-independent
validation; their done history and memory entries remain intact. The applied
0.9.0 target is reused without a duplicate bump. Personal YAML, lockfile, agent
links, and staged changes are excluded from delivery. This combined candidate
receives fresh independent review and full checks/tests before push or merge.
Actual test integration and main merge remain pending until verified events.

## Confirmed Test Integration

PR #79 merged into development at 6e572b29ca35b4ecafd841d870d5cbc8e08b4a32 on 2026-10-06 after CI run 37450733442 passed. The reviewed combined candidate passed task check, all 202 tests, and task build. Main promotion and publication remain pending.

Earlier local evidence records describe their historical snapshots. The current
combined delivery supersedes their unconfirmed-integration statements. The main
promotion preserves the production 0.8.1 history and reuses applied SKM 0.9.0;
production publication retains its required human approval.

## Verified Main Completion

PR #80 merged the confirmed development implementation into main at 79658625c5bb30cd84b0ad5185237e28e20038eb on 2026-10-06; actual main CI run 37451646628 passed task check, all 202 tests, and task build. Acceptance, documentation, catalog, applied SKM 0.9.0, and memory are reconciled. Production publication remains separately subject to qualification and human release-prod approval.

PR #79 previously confirmed test integration at
`6e572b29ca35b4ecafd841d870d5cbc8e08b4a32`. Independent review approved the
combined promotion at `a50a37ae214aefaf5317cf45701e6797cb4028d1`; its full
local check, 202-test aggregate, and build passed before PR #80 CI and merge.
The actual main tree matches that reviewed promotion. These completion records
receive independent review and full frozen-candidate checks/tests before
integration; earlier pending/local statements describe historical snapshots.
No production artifact or external registry notification is claimed. Older
release-bound specifications retain their individual completion obligations.
