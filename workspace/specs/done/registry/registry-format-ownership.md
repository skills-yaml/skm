# SKM-Owned Registry Formats Without Publisher-Specific Runtime Policy

## Status

State: `done`

Rationale: PR #80 merged the confirmed development implementation into main at 79658625c5bb30cd84b0ad5185237e28e20038eb on 2026-10-06; actual main CI run 37451646628 passed task check, all 202 tests, and task build. Acceptance, documentation, catalog, applied SKM 0.9.0, and memory are reconciled. Production publication remains separately subject to qualification and human release-prod approval.

## Problem and Contract

The [allowlist-removal follow-up](../workspace-toolkit/installation-contract-validation.md) removed governance version interpretation but retained Workspace-specific repository provenance, bundle completeness, standard pins, init flags, and prompts. Those rules exceed the CLI's responsibility. Workspace is an ordinary publisher/namespace whose skills own domain behavior. The repository's own development governance remains separate from product runtime.

SKM owns the registry layout, skill/dependency metadata, namespace manifest schemas, bundle membership, adapters, lock ownership, safe paths, and transactional installation. Namespace and bundle identifiers must never change validation rules. Every schema-2 publisher may publish arbitrary nonempty subsets of its packages under any valid bundle ID. Optional minimum-SKM, adapter, release-version, and provenance fields receive the same format checks across namespaces, with no repository identity allowlist or required publisher-specific metadata. Unsupported schemas and malformed recognized installation fields remain errors. Unknown root-level publisher metadata is opaque and ignored; nested installation structures remain strict.

Remove Workspace-only init flags, prompts, typed config/lock fields, and standard-source hashing. Existing unknown configuration and installation-lock values must remain readable and preserved as opaque metadata, including legacy Workspace data, without copying them into newly created locks or treating them as source authorization. Keep generic local toolkit/profile installation, registries, skills, dependencies, integrity checks for actual installed packages, and all filesystem/transaction safeguards. The user explicitly selected removal of Workspace-specific runtime support with preservation of existing YAML as opaque metadata.

## Affected Areas

- Bundle/search manifest validation and toolkit manifest parsing.
- Config serialization, generic installation locks, init flags/help, wizard prompts/validation, and removal of the unused Workspace source module.
- Regression tests for generic publishers, renamed namespaces/bundles, optional installation metadata, legacy metadata preservation, init/help, and retained failure paths.
- Human registry-format/toolkit documentation, README/index links, module contracts, spec catalog, release membership, and memory.

Exclude governed instructions, repository governance/pins, installed skill caches, personal configuration/links, historical done specs, runtime dependencies, CLI registry commands, adapter versions, publication, and other active work.

## Acceptance Criteria

1. Runtime validation contains no Workspace namespace, repository, governance field, standard-source, bundle-name, or option special cases. Workspace remains allowed as ordinary package data. Repository governance and example/test data may mention Workspace.
2. Equivalent manifests in arbitrary namespaces obey the same schema/member/dependency rules; bundles need not contain every package. Search discovers the same subsets that installation accepts. Optional minimum SKM, adapter, release, and provenance fields are validated uniformly; no publisher is implicitly trusted.
3. Domain metadata is ignored at manifest roots. Unknown config and existing installation-lock values survive read/write/reinstallation without interpretation; new locks contain only installation data. Legacy Workspace CLI options are rejected and help/prompts use generic toolkit terminology.
4. Tests retain supported-schema, exact-version, minimum-SKM, adapter, provenance-format, member-reference/duplicate, dependency, collision, integrity, ownership, rollback, and path safety coverage. Preview remains read-only, install/check work, and repeat installation is byte-idempotent.
5. SKM format documentation defines ownership, schema fields, extension behavior, and publisher-neutral validation. User/migration data and historical specs/packages are preserved. Reuse the applied shared 0.9.0 target without another bump.
6. Independent exact-candidate review resolves findings before frozen-candidate `task check` and `task test`. Results, catalog, and memory are reconciled; remain development until confirmed `development` integration and verified `main` merge. Publication remains separate.

## Implementation Plan

1. Register contract, catalog, and existing release membership before implementation.
2. Add generic publisher/subset/metadata regressions and remove namespace-specific bundle/search rules, applying optional format constraints consistently.
3. Retire Workspace-specific CLI/config/lock/source/prompt behavior while preserving generic opaque data and tested local toolkit functionality.
4. Publish human format reference and update examples/links, module contracts, and durable memory; self-review and run affected modules.
5. Obtain independent review, freeze the candidate, and run full aggregates. Result-record changes renew review/verification; no inferred lifecycle transitions.

## Risks and Compatibility

Removing Workspace CLI options and automatic standard-source pin hashing changes a prerelease public interface. Existing YAML data survives; SKM's config set/get and YAML edits remain available for publisher data. Skills own standard validation. Allowing opaque root metadata permits publishers to evolve domain data; recognized installation fields still deserialize strictly. A provenance claim is descriptive and never grants source trust. Do not relax real package hashing, lock ownership, safe paths, or dependency validation. Restore only this follow-up's changes for rollback, preserving earlier reviewed work and personal data. One agent mutates; independent review is read-only.

## Validation Gates

- Focused `task test:bundle` and `task test:toolkit`; extend Taskfile modules for search/config/init/wizard consumers. Serialize Cargo tasks; fixtures stay temporary.
- `task workspace:check`, `task versions:check`, `task coordination:check`, and working/staged diff checks.
- Independent review, then `task check` and `task test` with no final evidence reuse.
- Preserve canonical staged contents and personal manifest/lock bytes, prior standards, and historical done specs.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | minor | skm-next | Removes publisher-specific prerelease interfaces and generalizes registry-format validation before 1.0; reuse the applied shared 0.9.0 candidate without another bump. |

## Memory Impact

Status: `updated`

Rationale: The clarified ownership boundary, explicit retirement choice, and opaque metadata preservation are recorded in `workspace/agents/memory/decisions.md` and `workspace/agents/memory/changelog.md`.

## Local Acceptance Evidence

| Criterion | Evidence |
| --- | --- |
| 1 | Inspection of production Rust sources finds no Workspace or legacy trust-field references. Namespace, repository, standard pointers, and bundle-name special cases are removed. Domain mentions remain only in fixtures and human publisher/migration context. |
| 2 | Bundle fixtures loop over Workspace/acme/company and starter/all-workspace-skills under the same format contract, install a subset and its dependency, verify source links and exact pins, then repeat without changes. Search tests discover both subset bundles. Minimum SKM, adapter, release version, and descriptive provenance constraints fail uniformly before writes. |
| 3 | Config and wizard tests preserve arbitrary legacy/domain values; toolkit and ordinary installer tests preserve existing lock metadata and regenerate only installation fields. New toolkit locks omit unrelated config metadata. Init/help tests retain generic toolkit flags and reject all four retired domain flags. |
| 4 | Affected modules pass: 11 bundle, 29 toolkit, 10 search, 1 config, 4 ordinary installer, 5 init, and 18 wizard tests (78 total). Existing dependency, source-integrity, ownership, collision, safe-path, rollback, and read-only/idempotence tests remain. Before implementation, new bundle tests failed on the domain-specific completeness gate. |
| 5 | `workspace/docs/registry-format.md` defines SKM ownership and core/extension rules; generic toolkit documentation replaces runtime domain integration, with an ordinary-publisher example retained at its old link. Personal YAML/lock bytes and canonical staged contents match the previous handoff; prior standard packages and done history match HEAD. The existing applied 0.9.0 membership is reused. |
| 6 | Native Workspace/version/coordination and diff gates pass; memory resolves to updated. Independent review approved the implementation and full aggregates pass; reconciled records receive renewed review and aggregate verification before handoff; no shared-test integration, main merge, or publication is inferred. |

## Review and Verification Procedure

Reconcile known results before independent read-only exact-candidate review.
After approval, freeze the candidate and run `task check` and `task test`.
Tracked result records renew affected review and final verification. Report the
final repeated results and candidate identity at handoff; make no later tracked
edit. Final evidence reuse is disabled.

Candidate identity hashes sorted unique paths from
`git ls-files -z --cached --others --exclude-standard`: path bytes, NUL, `L` plus
symlink-target bytes, `F` plus regular file bytes, or `D` for a deleted tracked
path, and NUL, into SHA256. The deletion marker extends the prior recipe for the
removed standard-source module without excluding preserved user files or aliases.
The spec remains development pending actual shared-test integration.

## Independent Review and Aggregate Results

Independent read-only review approved SHA256
`122e30ef7f4f86518753caea6cd1d9c5ec16b60ce5c722019b620b0b17f047ab`
with no actionable findings. Production code, domain removal, generic metadata
handling, actual installation safety, tests, documentation, and user/historical
preservation were inspected. Native Workspace/version/coordination and diff
checks also passed independently.

`task check` passed installer/script contracts, formatting, warnings-free
all-target/all-feature Clippy, compilation, and native governance gates.
`task test` passed 154 Rust unit tests, 13 Rust integration tests, and 35 Python
validator tests (202 total). All 78 affected-module tests pass. No review or
aggregate finding required a fix. The applied 0.9.0 version is reused.

These result/catalog edits receive renewed exact-candidate review before
repeating both full aggregates on the resulting frozen candidate. Report its
exact final identity and repeated outcomes at handoff; make no later tracked
edit. Shared-test integration, main merge, and publication remain unconfirmed.

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
