# Test Spec: Resolve Registry-Published Workspace Skills

## Status

State: `test`

Rationale: PR #11 passed the configured validation workflow and merged into the
`development` test target on 2026-09-10 at merge commit
`6008aca7ca2f7fb86e71ed60092b33b39076f21c`. Production release through
`main` remains pending.

## Problem

Workspace lifecycle facades need to be installable as individual versioned
registry skills while retaining focused skill dependencies. SKM currently
resolves one configured registry package at a time, records registry skills in
a toolkit lockfile without expanding package dependencies, and projects Codex
skills to the legacy `.codex/skills` path.

## Scope

- Read exact `skm-dependencies` and `skm-version` values from the Agent Skills
  string metadata map in registry packages.
- Resolve dependency closures from the same trusted registry, rejecting
  cycles, version conflicts, invalid specifications, and missing packages.
- Use the resolved closure for regular install, list, check, add, and toolkit
  lockfile planning.
- Record exact metadata versions and integrity for resolved registry packages.
- Project Codex skills to `.agents/skills`, while accepting prior managed
  `.codex/skills` lockfile entries long enough to migrate them safely.
- Change new default configuration to the canonical
  `skills-yaml/registry` Git URL without rewriting existing configurations.
- Release the backward-compatible feature as SKM `0.4.0`.

## Non-Goals

- Resolving dependencies across different registries.
- Supporting version ranges in dependency metadata; dependencies are exact
  semantic versions for reproducibility.
- Removing dependency entries or links automatically when a parent is removed.
- Changing Codex native role-profile locations.
- Defining the registry publication workflow owned by the registry repository.

## Affected Areas

- `src/linker.rs`
- `src/main.rs`
- `src/toolkit.rs`
- `src/config.rs` and `src/config_manager.rs`
- focused unit and integration tests
- `README.md`
- `Cargo.toml` and `Cargo.lock`
- `workspace/specs/`, `workspace/docs/`, and `workspace/agents/memory/`

## Implementation Plan

1. Add strict Agent Skills metadata parsing and recursive same-registry
   dependency resolution in the linker boundary.
2. Apply the resolved closure consistently across regular and toolkit install,
   list, check, and add flows.
3. Migrate the Codex skill adapter root and preserve safe loading of legacy
   managed lockfile entries.
4. Update defaults, documentation, release version, and regression coverage.
5. Run Taskfile gates, self-review, and reconcile lifecycle and memory.

## Security and Compatibility

- Validate dependency names before filesystem access and exact versions before
  path construction.
- Inherit the already selected registry; metadata cannot select a new source.
- Reject local packages that declare registry dependencies because their trust
  source is ambiguous.
- Resolve the complete closure before any links or lockfile writes.
- Bound recursion through cycle detection and deduplicate identical requests.
- Preserve existing manifests without dependency metadata.
- Existing project registry settings remain unchanged; only newly generated
  defaults use the canonical URL.
- Accept prior `.codex/skills` lock ownership only for verified migration and
  never generate new skill outputs there.

## Acceptance Criteria

- Installing a registry facade expands and links its exact same-registry
  dependency closure.
- List and check evaluate the same closure as install.
- Toolkit dry-run and apply lock every resolved dependency with its exact
  metadata version and package integrity.
- Invalid dependency syntax, version conflicts, cycles, missing packages, and
  local dependency declarations fail before writes.
- Codex project and global skill targets use `.agents/skills`.
- A previous toolkit lock containing managed `.codex/skills` entries migrates
  safely to `.agents/skills` without accepting arbitrary output roots.
- Existing dependency-free skill configurations remain compatible.
- New default configuration uses the canonical registry URL.

## Validation Gates

- `task check`
- `task test`
- `task build`
- focused success and representative failure tests for dependency resolution
- Codex target and legacy lockfile migration tests

## Memory Impact

Status: `updated`

Rationale: The exact same-registry dependency contract and shared Codex skill
path are durable package-management decisions. They are recorded in
`workspace/agents/memory/decisions.md` and
`workspace/agents/memory/changelog.md`. Production publication evidence will
be added to `workspace/agents/memory/facts.md` after release.
