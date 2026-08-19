# Specification: Workspace Toolkit Manager

## Status

State: development

SKM 0.2.0 is implemented and locally validated on this feature branch. It
remains in development until confirmed integration into `development` and
production release through `main`.

## Problem

SKM installs independent skills, but cannot resolve a versioned Workspace
toolkit bundle, render portable role profiles for different coding agents, or
record exactly which project files it owns. Workspace migrations also need a
read-only bootstrap assessment before an agent can safely edit a partial or old
repository.

## Scope

Extend the existing `skills.yaml` format and install command. Keep existing
skills-only projects backward-compatible.

The first release supports:

- repository-local toolkit manifests selected from `skills.yaml`;
- bundle and explicit profile selection;
- Codex and Cursor project adapters;
- native Codex agent TOML and a portable Cursor role-as-skill fallback;
- deterministic SHA-256 integrity and `skills.lock.yaml` ownership;
- preflight collision checks, dry-run/JSON plans, transaction rollback, and
  idempotent re-application;
- `workspace audit`, `adopt`, `upgrade`, and `repair` planning against complete
  local standard packages or an explicitly authorized, commit-pinned and
  integrity-pinned Git source;
- checks for stale transactions, lock drift, source integrity, and managed
  output drift.
- preservation of the toolkit's mandatory OpenTofu-only infrastructure
  constraint in every rendered skill and role projection.

Remote archive sources and automatic execution of migration instructions are
out of scope. Workspace structural mutations remain delegated to the verified
`adopt-workspace-structure` workflow; SKM prepares and verifies the plan and
installs the workflow.

## Configuration Contract

Optional backward-compatible fields are added to `skills.yaml`:

```yaml
toolkit:
  manifest: workspace/instructions/toolkit/manifest.yaml
  version: 0.1.0
bundles:
  - development-core
profiles:
  - security-reviewer
workspace:
  standard: workspace-docs@4.0.0
  source: workspace/instructions/standards/workspace-docs
  revision: null # required for Git sources
  integrity: null # required for Git sources
trusted_sources:
  - workspace/instructions/standards/workspace-docs
```

All paths are project-relative, must remain within the project after
canonicalization, and must not traverse symlinks. Trust is project-scoped; SKM
does not infer it from global configuration or nearby repositories.

## Install Behavior

1. Load and validate the existing manifest.
2. Resolve the toolkit once, verify the requested version and SKM compatibility,
   and expand bundles plus explicit profiles deterministically.
3. Validate source containment, required files, profile skill references, and
   SHA-256 integrity.
4. Build every skill link, generated profile, lockfile, and removal in memory.
5. Stop before writes on any unmanaged collision or unsupported adapter.
6. Display the plan for `--dry-run`; use stable JSON for `--json`.
7. Apply through a repository-local transaction journal. Restore replaced
   managed outputs and remove newly created outputs if any write fails.
8. Write `skills.lock.yaml` last. Repeating unchanged inputs is a no-op.

Only outputs present in the prior lockfile may be replaced or removed. An
existing identical output may be adopted without replacement.

## Workspace Commands

`skm workspace audit|adopt|upgrade|repair` inspects only the current repository.
It reports detected and target versions, source choice, completeness,
intermediate versions, missing migration resources, and whether the operation
would write. Mutating modes still default to a plan and require `--apply --yes`
to create a verified handoff file; they never execute migration prose.

A partial repository-local standard package is a blocker. An explicit complete
source may resume the plan. Git retrieval uses a full immutable commit id,
expected SHA-256 package integrity, safe blob materialization without executing
checkout filters, and a verified project-local cache. Required package resources are `manifest.yaml`,
`agents-template.md`, `audit-checklist.md`, and `migration.md` for each version
boundary, plus the package-level `AGENT_MIGRATION.md`.

## Acceptance Criteria

- Existing `skills.yaml` files deserialize unchanged and existing commands keep
  their semantics.
- Codex and Cursor install the same resolved skills from one bundle selection.
- Codex receives valid native agent TOML; Cursor receives an explicitly labeled
  generated-skill fallback.
- The lockfile records toolkit/workspace versions, source hashes, adapter
  versions, and every managed output.
- A second unchanged install leaves all files and the lockfile byte-identical.
- Dry-run performs no writes and reports every planned output and removal.
- Unmanaged collisions, path traversal, source symlinks, integrity drift,
  unsupported versions, and stale transactions fail before writes.
- A failed apply rolls back earlier outputs and leaves the prior lockfile intact.
- Workspace assessment of a partial local package reports exact missing
  resources and makes no changes; an explicit complete source produces a
  resumable verified plan.
- Unit and end-to-end tests cover success and representative failures.

## Affected Areas

- `src/config.rs`
- `src/main.rs`
- `src/linker.rs`
- new focused toolkit, lockfile, transaction, adapter, and workspace modules
- `Cargo.toml` and `Cargo.lock`
- `README.md`

## Validation Gates

- `task check`
- `task test`
- `task build`
- `git diff --check`

## Risks

- Agent formats may evolve independently; adapters therefore carry independent
  versions and fail visibly on unsupported targets.
- Symlink and path handling is security-sensitive; resolution fails closed and
  never follows package symlinks.
- Rollback cannot make arbitrary unmanaged files safe; preflight prohibits
  touching them.
- Workspace migration prose is not executable code. This release creates a
  verified handoff instead of interpreting instructions automatically.

## Validation Result

`task check`, `task test`, `task build`, and `git diff --check` pass. The test
suite covers backward-compatible configuration, two-agent idempotent install,
target removal, collisions, unsafe paths and symlinks, lock and source drift,
transaction rollback, partial workspace packages, unauthorized remote sources,
and pinned Git retrieval into a verified project-local cache. The Workspace
package was also installed for Codex and Cursor and checked through a no-diff
second apply.

## Memory Impact

Status: `pending`

Rationale: The 0.2.0 toolkit manager is implemented locally; the durable release record stays pending until production release through main.
