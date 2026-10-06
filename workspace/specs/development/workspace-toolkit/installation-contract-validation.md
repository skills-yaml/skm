# Validate Installation Contracts Independently of Workspace Governance

## Status

State: `development`

Rationale: The user requested removal of the hard-coded Workspace Docs major-version allowlist on 2026-10-05. Local implementation has independent approval and passing aggregate checks/tests; reconciled result records receive renewed review and verification before handoff. Shared-test integration and main merge are unconfirmed.

## Problem and Scope

SKM currently rejects otherwise valid toolkit and published Workspace bundle manifests unless they declare Workspace Docs 4.x, 5.x, or 6.x. Governance compatibility belongs to Workspace skills; requiring a CLI release for each governance major prevents valid installations.

Consumers are local toolkit users and users installing published Workspace bundles. Follow up on [the historical 6.x compatibility contract](../../done/workspace-toolkit/workspace-docs-6-compatibility.md) and [the separate 7.0.0 repository adoption](../workspace-governance/adopt-workspace-docs-7.md); preserve those records.

SKM must validate the installation contract: supported manifest schemas, selected versions, minimum SKM version, adapter compatibility where declared, dependencies, package/bundle completeness, provenance, integrity, ownership, safe paths, and transactional writes. The recognized legacy `workspace_docs_compatibility` field becomes optional ignored YAML metadata. Future major strings, omitted values, nulls, and other YAML values must not impose a governance gate. Strict parsing of installation fields and rejection of unknown fields remain intact. Workspace standard pins remain opaque identifiers recorded in lockfiles; source integrity and revisions remain enforced.

Exclude changes to governed instructions, CLI commands, dependencies, lockfile schemas, adapter versions, source authorization rules, release publication, personal configuration, and historical done specs. This does not assert that every skill supports every governance standard: the skills make that determination.

## Affected Areas

- `src/toolkit.rs`: manifest parsing/validation and installation/check/idempotence regression tests.
- `src/bundle.rs`: published Workspace manifest parsing/validation and bundle preview/apply regression tests.
- `Taskfile.yml` and `workspace/validation/README.md`: focused test modules with serialized Cargo resources.
- `workspace/docs/workspace-toolkit.md`: installation/governance responsibility boundary.
- Spec catalog, release reservation membership, and durable decision/changelog.

## Acceptance Criteria

1. Both installation paths accept 7.x and future major declarations, absent governance metadata, and arbitrary valid YAML metadata without governance-version interpretation. Older manifests remain accepted.
2. Toolkit planning remains read-only; apply and check succeed, preserve an opaque Workspace standard pin, and repeat installation preserves lockfile bytes with no actions. Bundle preview remains read-only; apply installs every member and dependency, verifies expected symlink targets, and repeats without manifest changes.
3. Unsupported installation schemas, unknown fields, insufficient SKM versions, unsupported adapters, invalid provenance, incomplete bundles, dependency errors, unsafe paths, ownership collisions, and integrity drift retain their existing rejection behavior. Representative negative tests demonstrate that removing the governance gate does not bypass installation constraints.
4. Documentation assigns governance compatibility to Workspace skills and describes the retained installation checks. Existing migration edits, staged user changes, personal configuration/links/lockfile, historical packages, and done specs are preserved.
5. Independent exact-candidate review resolves every finding before full `task check` and `task test`; every criterion has local evidence. Stay in development until confirmed integration into `development`, then require verified `main` merge for done. Publication is separate.

## Implementation Plan

1. Register this contract, catalog row, and membership in the already-applied shared release before code changes.
2. Add focused toolkit/bundle test entrypoints and regressions; reproduce the current rejection.
3. Remove the two allowlists and ignore optional governance metadata while preserving installation validation.
4. Update human documentation and durable memory, run affected modules, and self-review preservation and acceptance.
5. Obtain independent read-only review, resolve findings, freeze the candidate, and run complete checks/tests. Later tracked result edits renew review and verification.

## Validation Gates

- `task test:toolkit` and `task test:bundle`, sequentially because Cargo shares target storage; temporary fixtures never touch personal installations.
- `task workspace:check`, `task versions:check`, and `task coordination:check` for reconciled records.
- Independent exact-candidate review, then `task check`, `task test`, and working/staged `git diff --check`.
- Preserve canonical staged content and personal manifest/lockfile bytes; no final evidence reuse or inferred lifecycle transitions.

## Risks and Rollback

Ignoring the legacy field deliberately accepts metadata older binaries rejected. It grants no source trust, filesystem permission, or governance capability. Preserve strict installation parsing and source hashing. Retain the field name as recognized input so `deny_unknown_fields` does not break legacy manifests. Restore only this task's code/tests/docs/records if rollback is needed; preserve pre-existing migration and user work. One agent mutates; independent review is read-only.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | minor | skm-next | Expands accepted installation inputs before 1.0; reuse the existing applied shared 0.9.0 candidate without another bump. |

## Memory Impact

Status: `updated`

Rationale: The user-directed responsibility boundary and retained installation checks are recorded in `workspace/agents/memory/decisions.md` and `workspace/agents/memory/changelog.md`.

## Local Acceptance Evidence

| Criterion | Evidence |
| --- | --- |
| 1 | Both manifest types recognize and ignore optional legacy metadata. Tests install declarations 7.x/99.x and absent/null/numeric/mapping/sequence/empty values; existing 4.x/5.x/6.x cases pass. No Workspace Docs major allowlist remains in runtime code. |
| 2 | Toolkit regression tests perform read-only planning, installation, check, byte-idempotent repetition, and preservation of `workspace-docs@99.0.0` in the lock. Published Workspace bundle tests preview without writes, resolve member dependencies, verify exact pins/source links, apply, and repeat without config changes. |
| 3 | New negative fixtures retain strict schema/unknown-field/version/minimum checks and Workspace bundle adapter/provenance/completeness constraints before any project writes. Existing dependency-cycle, unmanaged collision, path/symlink, lock ownership, integrity drift, and rollback tests pass. |
| 4 | Human toolkit documentation describes skill-owned governance and CLI-owned installation contracts. Inspection verifies personal manifest/lock bytes, canonical staged contents, old standard packages, and historical done specs are preserved. No governed instruction is changed by this follow-up. |
| 5 | Focused modules pass: 28 toolkit tests and 12 bundle tests. Independent review approved the implementation without findings and aggregate checks/tests pass; local implementation remains in development without inferred integration. |

## Regression and Delivery Evidence

Before the runtime change, the new toolkit installation cases failed on the
4.x/5.x/6.x gate, and the published bundle cases failed on unsupported or missing
governance metadata. After removing only those gates and ignoring the optional
field, `task test:toolkit` and `task test:bundle` pass. `task fix` applies standard
formatting and passes Clippy. No dependency, Cargo version, adapter version,
configuration, or lock schema changed. The shared applied 0.9.0 reservation adds
this member without a second bump.

Independent review must finish before frozen-candidate `task check` and `task
test`. Tracked review/result records renew affected review and verification;
final aggregate results and exact candidate identity are reported at handoff.
Shared-test integration, main merge, and publication are unconfirmed.

## Review and Aggregate Results

Independent read-only review approved candidate SHA256
`7204c50ca144934e0792d09a14e4ca78f03eaae123bb7d327e2488d3823db386`
with no actionable findings. The reviewer confirmed retained installation checks,
real installation/idempotence tests, coherent records, and preservation of user
and historical content. Native Workspace, version, coordination, and diff checks
also passed independently.

`task check` passed installer contracts, formatting, warnings-free all-target/
all-feature Clippy, compilation, and native Workspace/version/coordination checks.
`task test` passed 152 Rust unit tests, 13 Rust integration tests, and 35 Python
validator tests (200 total). The focused suites passed 28 toolkit and 12 bundle
tests. No source fix was required by review or aggregate verification.

These result/catalog edits renew affected review and verification. Freeze the
renewed candidate and rerun both complete aggregates before handoff; report its
exact SHA256 and repeated outcomes there without any later tracked edit. Hash
sorted unique repository-relative paths from `git ls-files -z --cached --others
--exclude-standard`, then each path, NUL, `L` plus symlink-target bytes or `F` plus
file bytes, and NUL into SHA256, including alias symlinks and preserved user files.
No evidence reuse is assumed. This local result does not establish shared-test
integration, main merge, or publication.

## Clarified Publisher-Independent Follow-up

On 2026-10-06 the user clarified that SKM owns the installation format and must
have no Workspace-specific runtime knowledge. See
[registry-format-ownership.md](../registry/registry-format-ownership.md).
That follow-up supersedes this candidate's retention of typed standard pins,
source hashing, strict root publisher fields, and publisher-specific bundle
constraints. The review/test evidence above describes this earlier candidate;
historical results and unconfirmed integration remain unchanged.

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
