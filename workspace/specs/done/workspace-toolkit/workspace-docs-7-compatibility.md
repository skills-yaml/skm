# Workspace Docs 7 Compatibility

State: done

## Problem and Authority

The user explicitly approved updating SKM compatibility after the published v7
bundle failed. SKM 0.9.0 rejects its valid `7.x` declaration before installation;
local toolkit validation rejects the same declaration.

## Scope and Affected Areas

Extend supported majors in `src/bundle.rs` and `src/toolkit.rs` to 4.x, 5.x,
6.x, and 7.x. Update README, regression fixtures, and production version to 0.8.1. Preserve
integrity, adapter, minimum-version, transaction, and source-containment checks.
Existing pins are preserved: production uses Workspace Docs 5 and development
uses Workspace Docs 6. No instruction adoption occurs.

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

## Confirmed Development Integration

PR #76 integrated reviewed candidate
`3f8e0ddd67bbc82d2ef099d4b7a99e86a6584d99` into development on 2026-10-01
at `9068581b1294d4a87705994db71e0b37f7b06abf`. The actual combined tree equals
the reviewed candidate. Native checks, tests, optimized builds, and PR CI
passed. The local development binary installed all twenty published v7 bundle
members, passed `skm check`, and proposed zero additions on repetition.

Production maintenance candidate `167c9dd562b536ff0e795ed7fc55fd91d3d911ad`
passed independent review, `task check`, all 153 Rust and six documentation
tests, `task build`, and PR #75 CI. Development artifact qualification and
production publication remain separate release gates.

## Published Development Qualification

Release run `36847929386` passed all four platform builds and published
development-latest at `9068581b1294d4a87705994db71e0b37f7b06abf`. The downloaded
Linux archive passed its SHA-256 check; its manifest and `skm self version`
agree on channel development, version 0.9.0, and that exact commit. The published
binary installed all twenty live v7 bundle members in a fresh project, created
40 Codex/Cursor links, passed `skm check`, and proposed zero additions or links
on repetition. Production remains pending; no main release is inferred.

## Confirmed Production Release

PR #75 merged the independently reviewed maintenance candidate
`54524bb4bce314f9d7bf6978e667f7148514707a` into main on 2026-10-01 at
`49468aab7f6bbbe9c474001823ad6f79ab207f32`; its actual combined tree equals
that candidate. Production CI `36849153059` and four-platform release
`36849152975` passed. The published Linux archive passed SHA-256 verification;
its manifest and `skm version` identify prod, 0.8.1, and that exact commit.

The production binary installed all twenty live Workspace v7 bundle members
in a fresh Codex/Cursor project, passed `skm check`, and proposed no additions
or links on repetition. The bundled adoption default selects released v7.
All five acceptance criteria are supported by the reviewed positive/negative
regression fixtures, native check/test/build gates, and both published-channel
qualifications above. Earlier pending statements are historical checkpoints.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | patch | skm-v7-maintenance | Production maintenance 0.8.1 adds explicit v7 support; development retains its independently applied shared 0.9.0 reservation. |
