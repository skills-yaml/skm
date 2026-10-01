# Workspace Docs 7 Compatibility

State: development

## Problem and Authority

The user explicitly approved updating SKM compatibility after the published v7
bundle failed. SKM 0.9.0 rejects its valid `7.x` declaration before installation;
local toolkit validation rejects the same declaration.

## Scope and Affected Areas

Extend supported majors in `src/bundle.rs` and `src/toolkit.rs` to 4.x, 5.x,
6.x, and 7.x. Update README, regression fixtures, and production version to 0.8.1. Preserve
integrity, adapter, minimum-version, transaction, and source-containment checks.
Existing repository pins are preserved: development uses Workspace Docs 6,
while the production maintenance branch uses Workspace Docs 5. No adoption occurs.

## Acceptance Criteria

1. Registry bundles and local toolkits accept 7.x and retain older majors.
2. Missing, malformed, and future 8.x declarations fail before writes.
3. Minimum SKM version and existing safety gates remain enforced.
4. V7 toolkit install/check is byte-idempotent; namespace tests cover supported
   majors, unsupported declarations, and minimum-version failures.
5. The released binary installs all twenty live v7 bundle members, passes
   `skm check`, and proposes no additions on repetition.

## Plan and Validation Gates

Add regression coverage, extend both allowlists, reconcile docs and memory,
obtain independent exact-candidate review, then run `task check`, `task test`,
and `task build`. Integrate through development and verify its published
artifact before main promotion. Verify production checksum, source identity,
and the live bundle before completion.

## Risks

Cover both consumers and keep future unsupported majors rejected. Preserve
released package data and all existing transaction and integrity validation.

## Memory Impact

Status: updated

Rationale: Record the explicit v7 compatibility contract in `workspace/agents/memory/facts.md` and its
corresponding `workspace/agents/memory/changelog.md`; actual channel events are recorded after verification.

## Maintenance Release Route

The production candidate is based on main and targets 0.8.1. Integrate this
maintenance candidate into development first, retaining development's existing
0.9.0 version when reconciling Cargo metadata. Verify the development artifact
against the live v7 bundle, then release the independently reviewed and fully
verified 0.8.1 maintenance branch through a main PR. Both actual combined
revisions must pass native checks; no completion is inferred from branch names.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | patch | skm-next | Add explicit v7 compatibility within the already-applied shared development release; production maintenance is separately 0.8.1. |
