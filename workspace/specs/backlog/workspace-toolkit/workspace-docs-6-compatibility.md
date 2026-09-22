# Specification: Workspace Docs 6 Toolkit Compatibility

## Status

State: backlog

Status rationale: SKM 0.6.0 rejects toolkit manifests declaring
`workspace_docs_compatibility: 6.x`. The compatibility extension and its
end-to-end installation evidence are specified but not yet implemented.

Primary feature: `workspace-toolkit`

## Problem

The Workspace development toolkit 0.4.0 candidate targets Workspace Docs 6.x
and declares minimum SKM 0.5.0. The current SKM 0.6.0 production build still
accepts only toolkit compatibility values `4.x` and `5.x`, so both
`skm install --dry-run --json` and `skm check` stop before resolving the
otherwise valid candidate.

The restriction is explicit in `src/toolkit.rs`, and current regression coverage
treats `6.x` as unsupported. Updating only documentation or the toolkit's
minimum-version field cannot make installation compatible.

## Evidence

On 2026-09-22, the production binary identified itself as
`skm 0.6.0 (prod - 7ec135e0567b4c3f09421146ed3bd67d65976c5f)`. Against a
toolkit 0.4.0 manifest declaring Workspace Docs `6.x`, both the non-mutating
install plan and the explicit compatibility check exited unsuccessfully with:

```text
toolkit must declare a supported workspace_docs_compatibility: 4.x or 5.x
```

No installation or project-file mutation occurred.

## Goals

- Accept valid toolkit manifests declaring Workspace Docs `6.x`.
- Preserve compatibility with existing `4.x` and `5.x` toolkit manifests.
- Keep unsupported, missing, and malformed declarations fail-closed before any
  installation writes.
- Prove compatibility through unit coverage and a representative end-to-end
  toolkit installation, lockfile, projection, and `skm check` cycle.
- Publish the support through SKM's normal development and production release
  channels before dependent toolkits claim compatibility.

## Non-Goals

- Adopting Workspace Docs 6 for the SKM repository itself.
- Interpreting or executing Workspace policy or migration prose.
- Weakening toolkit schema, integrity, source-containment, transaction,
  collision, or minimum-SKM-version validation.
- Dropping support for Workspace Docs 4.x or 5.x.
- Publishing the dependent Workspace toolkit or its skills to a registry.
- Accepting arbitrary future compatibility majors without an explicit SKM
  change and regression coverage.

## Consumers

- Toolkit maintainers publishing bundles that target Workspace Docs 6.x.
- Projects installing those bundles through SKM.
- Release automation that validates generated skills, profiles, lockfiles, and
  agent projections before registry publication.

## Proposed Behavior

SKM recognizes `4.x`, `5.x`, and `6.x` as supported
`workspace_docs_compatibility` declarations. Compatibility validation remains
an explicit allowlist and executes before SKM builds or applies an installation
plan.

The toolkit's `minimum_skm_version` remains an independent constraint. A
manifest targeting `6.x` is accepted only when the running SKM version also
satisfies that declared minimum and all existing manifest, source, integrity,
adapter, output, and transaction checks pass.

Diagnostics for rejected compatibility values must report the complete current
supported set. Human-readable and JSON dry-run behavior must remain consistent:
successful planning is non-mutating, and compatibility failure produces no
partial links, projections, transaction files, or lockfile changes.

## Compatibility and Migration

This is additive for valid Workspace Docs 6 toolkits and must not change the
resolved output of existing 4.x or 5.x projects. Existing lockfiles remain
valid when their inputs and outputs are unchanged.

Projects blocked on `6.x` require a newly released compatible SKM binary. They
must not work around the check by declaring `5.x` for a toolkit that depends on
Workspace Docs 6 contracts. No configuration migration is required after the
compatible binary is installed.

## Affected Areas

- `src/toolkit.rs`: compatibility validation and diagnostics.
- Toolkit validation unit tests colocated with `src/toolkit.rs`.
- Integration fixtures covering install, lockfile generation, projections,
  idempotence, and `skm check`.
- `README.md` when its supported Workspace toolkit compatibility guidance needs
  updating.
- Release metadata and verification evidence for the first compatible SKM
  artifact.

No new runtime dependency is expected.

## Acceptance Criteria

1. Valid toolkit manifests declaring `4.x`, `5.x`, or `6.x` pass compatibility
   validation when all other requirements are satisfied.
2. Missing, malformed, and unsupported declarations, including an unimplemented
   future major, fail before writes with an accurate supported-values message.
3. `minimum_skm_version` remains enforced independently for `6.x` manifests;
   compatibility support cannot bypass a higher required SKM version.
4. `skm install --dry-run --json` produces a deterministic, non-mutating plan
   for a representative Workspace Docs 6 toolkit containing skills, bundles,
   profiles, agent projections, and a workspace package.
5. In an isolated temporary project, `skm install --yes` followed by
   `skm check` succeeds for the same fixture, and a second unchanged install is
   a no-op with a byte-identical lockfile.
6. Existing Workspace Docs 4 and 5 compatibility fixtures retain their previous
   behavior and output.
7. Representative invalid source, integrity, adapter, collision, and
   transaction cases remain rejected; accepting `6.x` does not weaken any
   downstream safety check.
8. The development-channel artifact passes the representative Workspace Docs 6
   install/check cycle before production release.
9. The production artifact reports the intended SKM version and exact release
   commit, and repeats the install/check verification successfully before the
   compatibility work is considered released.

## Validation Gates

- `task check`
- `task test`
- `task build`
- `git diff --check`
- A temporary-project `skm install --dry-run --json` run using the built binary.
- A temporary-project install, repeated install, and `skm check` run using the
  built binary.
- Development and production release-artifact identity and checksum
  verification through the existing release workflow.

## Delivery Plan

1. Move this spec to `development` when implementation begins and update the
   catalog in the same change.
2. Extend the explicit compatibility contract and update positive and negative
   regression fixtures.
3. Add the representative Workspace Docs 6 toolkit installation fixture and
   confirm failure remains pre-write for unsupported majors.
4. Run all local validation gates and self-review compatibility, diagnostics,
   and filesystem safety.
5. Integrate through the configured `development` branch, record the merge and
   shared validation evidence, and move the spec to `test`.
6. Qualify the development-channel artifact against the representative toolkit.
7. Release through `main`, verify the production artifact, resolve memory
   impact, and move the spec to `done` only after every release gate passes.

## Risks and Rollback

- A broad or parser-only change could accept future contracts SKM does not
  understand. Keep the supported-major set explicit and tested.
- A unit-only result could miss projection, lockfile, or adapter incompatibility.
  Require the representative end-to-end install/check fixture.
- Misstating a toolkit as `5.x` would conceal a real contract mismatch. Reject
  that workaround and ship a compatible SKM binary instead.
- Compatibility errors must remain pre-write. Regression tests must inspect the
  temporary project for partial state after failures.

Before release, rollback consists of reverting the scoped compatibility change
and retaining the clear rejection. After release, preserve the published
artifact and correct regressions in a successor release rather than rewriting
release assets.

## Open Questions

- Which next SKM version will carry the additive compatibility support? Select
  it when implementation begins under the repository's current release plan.
- Should the representative Workspace Docs 6 toolkit fixture be maintained
  entirely in SKM or generated from a pinned public toolkit revision? Prefer a
  deterministic local fixture unless cross-repository validation can remain
  immutable, offline, and reviewable.

## Memory Impact

Status: pending

Rationale: Resolve during implementation after the supported-major contract,
release version, and durable compatibility evidence are finalized.
