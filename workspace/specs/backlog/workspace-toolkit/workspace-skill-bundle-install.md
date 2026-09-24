# Backlog Spec: Install Workspace Skill Bundles

## Status

State: `backlog`

Rationale: The cross-repository bundle contract is specified, but implementation
has not started. The proposed SKM interface is now generic `skm bundle add`,
following the decision to remove Workspace-specific CLI commands. Workspace
must publish canonical bundle membership and Registry must publish that
membership before SKM can release the consumer command.

## Companion Specifications

This specification is the SKM consumer part of one coordinated change:

- Workspace: `All Workspace Skills Bundle`
- Registry: `Publish Workspace Skill Bundles`

The three specifications must keep the bundle identifier, manifest fields,
version rules, validation behavior, and rollout order aligned.

## Problem

Workspace skills are independently installable from the `workspace` registry
namespace, but adding the complete supported set requires users to discover
and add each package separately. Existing Workspace toolkit bundles are stored
in the Workspace source repository and include profiles, while registry users
need a trusted, versioned skill-only bundle that SKM can add to a project in
one command.

Inferring the set from registry directories would make filesystem layout an
API, could include stale or unpublished entries, and would not bind the result
to one reviewed Workspace release. Repeated single-skill additions also risk a
partially updated project when a later package conflicts or fails.

## Goals

- Let a user add every skill in the released Workspace package set to the
  current project with one non-interactive command.
- Support named Workspace bundles through the same code path.
- Resolve bundle membership only from the trusted registry namespace manifest.
- Persist exact, explicit skill entries in the existing project `skills.yaml`
  schema so normal install, list, check, and version workflows continue to
  operate without hidden bundle state.
- Preview the complete plan before mutation and apply the plan atomically.
- Make an identical repeated command a successful no-op.
- Preserve existing registry, dependency, path, and symlink safety rules.

## Non-Goals

- Installing Workspace agent profiles.
- Adding a global-scope variant of the command.
- Scanning registry directories to discover bundle members.
- Adding a second project configuration schema for persistent bundle intent.
- Removing a bundle or pruning skills that once belonged to a bundle.
- Automatically upgrading an already configured bundle when future Workspace
  releases change its membership.
- Supporting arbitrary third-party namespace manifests in the first release.
- Changing Workspace adoption, toolkit bootstrap, or repository instruction
  management.

## User Experience

The primary interface is:

```text
skm bundle add workspace/all-workspace-skills --source default --dry-run
skm bundle add workspace/all-workspace-skills --source default --yes
```

The general named-bundle interface is:

```text
skm bundle add workspace/<named-bundle> --source default --yes
```

Rules:

- A fully qualified `namespace/bundle` identifier is required.
- `--source` selects a configured registry and defaults to `default`; it does
  not persistently change registry configuration.
- `--dry-run` prints a human-readable plan and performs no writes.
- `--json` prints the versioned machine-readable plan and performs no writes.
- `--yes` authorizes the project mutation. Without one of `--dry-run`,
  `--json`, or `--yes`, the command fails with an actionable message.
- `--global` is not accepted.
- Help and error output follow `DESIGN.md`; normal data goes to stdout and
  diagnostics go to stderr.

The plan reports the source registry, namespace-manifest release metadata,
bundle identifier, requested members, dependency-only members, additions,
unchanged entries, conflicts, target links, and whether mutation is allowed.
Output ordering is deterministic by skill identifier.

## Registry Manifest Contract

SKM consumes the registry-owned `skills/workspace/manifest.yaml`. The Registry
companion specification evolves that document to schema 2 with its existing
release and package fields plus a `bundles` mapping:

```yaml
schema_version: 2
namespace: workspace
toolkit_version: "<exact-semver>"
source_repository: "https://github.com/skills-yaml/workspace.git"
source_revision: "<full-commit>"
workspace_docs_compatibility: "<range>"
minimum_skm_version: "<exact-semver>"
skm_adapter_compatibility: "<range>"
packages:
  <skill-id>: "<exact-semver>"
bundles:
  all-workspace-skills:
    packages:
      - <skill-id>
```

SKM must reject:

- unsupported schema versions or namespaces;
- malformed bundle identifiers, duplicate members, unknown members, or empty
  bundles;
- package versions that are not exact semantic versions;
- an `all-workspace-skills` bundle whose member set differs from the complete
  `packages` key set;
- manifests that fail existing Workspace Docs, toolkit, adapter, minimum-SKM,
  source, revision, path, or integrity compatibility checks.

Member ordering is canonical in the manifest, but set equality defines the
special `all-workspace-skills` invariant. SKM may sort for display and config
serialization where existing conventions require it.

## Design

### Planning

1. Resolve and refresh the selected configured registry using existing trust
   and cache rules.
2. Read and validate the Workspace namespace manifest without deriving any
   members from directories.
3. Select the fully qualified named bundle.
4. Convert every member to an exact same-registry skill request using the
   manifest's `packages` mapping.
5. Resolve the complete existing `skm-dependencies` closure before writes.
6. Load the project configuration once and classify each requested entry as an
   addition, an identical no-op, or a source/version conflict.
7. Validate every package and target path, preflight all links, and produce the
   complete deterministic plan.

Any conflict or validation failure rejects the whole operation before a
configuration or link is changed. An existing entry is identical only when its
normalized skill identity, selected registry, and exact version match.

### Persistence

On apply, SKM adds the requested and dependency-resolved packages as explicit
entries in the existing `skills` list. It does not persist only a bundle name:
the committed project configuration must remain self-describing and usable by
current install, list, check, and version commands.

The operation must use one transaction across the project configuration and
managed links:

- detect a concurrent config change between plan and commit;
- stage or back up all affected outputs before replacement;
- write configuration atomically;
- create or replace only SKM-managed symlinks;
- never overwrite a real file or directory; and
- restore the prior configuration and links if any commit step fails.

No-op applies perform no writes. Existing unrelated skills and configuration
fields remain byte-equivalent where the serializer permits and semantically
unchanged in all cases.

### Module Boundaries

- `src/main.rs`: Clap command shape and orchestration only.
- `src/config.rs`: schema-preserving config merge and serialization types.
- `src/linker.rs`: package resolution, dependency closure, path checks, link
  preflight, and link application.
- A focused Workspace-bundle module may own namespace-manifest parsing,
  validation, plan construction, and transaction coordination if that keeps
  the established boundaries smaller and testable.

No new runtime dependency is expected. A dependency may be proposed only when
the standard library and current crates cannot implement the contract safely,
and its need must be documented before addition.

## Compatibility and Migration

- Existing commands and schema-1 registries continue to work for their current
  individual-skill operations.
- The new bundle command requires a schema-2 Workspace namespace manifest and
  emits a clear upgrade error for schema 1.
- Existing projects require no migration because bundle expansion persists the
  current explicit `skills` representation.
- Existing toolkit manifests, lockfiles, bundles, profiles, and Workspace
  lifecycle commands remain unchanged.
- SKM must not claim compatibility with the bundle release until its own
  supported Workspace/toolkit ranges include the versions published by the
  companion repositories.

## Security and Reliability

- Treat registry manifests and package metadata as untrusted input.
- Validate identifiers and exact versions before constructing paths.
- Keep dependency resolution inside the selected registry.
- Resolve and preflight the entire closure before any mutation.
- Bound dependency traversal and retain cycle and version-conflict detection.
- Verify package provenance and integrity through existing registry rules.
- Reject symlink escapes and real target collisions.
- Avoid printing local secrets, credentials, or sensitive configuration in
  human or JSON output.

## Acceptance Criteria

1. `skm bundle add workspace/all-workspace-skills --source default --dry-run` returns the
   complete, deterministic skill and dependency plan without changing files.
2. The same command with `--yes` adds and links the exact packages declared by
   the registry's `all-workspace-skills` bundle in one transaction.
3. A named Workspace bundle uses the same generic command and planning path.
4. Every persisted bundle member has an explicit exact version and source in
   the existing project `skills` configuration.
5. Profiles are not installed or persisted.
6. A second identical apply succeeds as a no-op and changes no files or links.
7. Existing identical entries are retained; any source or version conflict
   fails the complete plan before writes.
8. Malformed, unsupported, incomplete, duplicated, or inconsistent manifest
   bundle data fails with actionable diagnostics.
9. A failure during commit or a detected concurrent config edit leaves the
   pre-command project configuration and links intact.
10. `--json` produces stable structured preview output and no writes.
11. Existing individual skill, toolkit, list, install, and check
    behavior remains compatible.
12. CLI help documents all arguments, exclusivity rules, safety modes, and
    project-only scope.

## Affected Areas

- `src/main.rs`
- `src/config.rs`
- `src/linker.rs`
- focused bundle/transaction module if introduced
- unit and integration tests
- `README.md`
- `DESIGN.md` only if the established output contract needs an additive entry
- `workspace/specs/`
- `workspace/agents/memory/` when the implementation resolves memory impact

## Implementation Plan

1. Freeze the shared schema-2 manifest and CLI contract with the Workspace and
   Registry companion specifications.
2. Add strict manifest and bundle parsing fixtures, including negative cases.
3. Add the generic bundle Clap command and deterministic human/JSON planning output.
4. Implement config merge, complete dependency preflight, concurrent-edit
   detection, and transactional config/link application.
5. Add unit, integration, rollback, idempotency, and compatibility tests.
6. Validate against the released Registry manifest after Workspace and
   Registry complete their rollout.
7. Update user documentation, compatibility metadata, and durable memory as
   required before release.

## Validation Gates

- `task check`
- `task test`
- `task build`
- `git diff --check`
- CLI parse and help snapshots or assertions
- manifest schema and invariant fixtures
- dry-run and JSON no-write tests
- config conflict, no-op, rollback, concurrent-edit, and real-file collision
  tests using temporary directories
- end-to-end install, `skm check`, and second-apply convergence against a
  released Workspace registry fixture

## Rollout and Rollback

Rollout order is Workspace canonical bundle, Registry publication, then SKM
consumer release. SKM may develop against a pinned fixture, but production
acceptance requires the released registry data. A fresh temporary project must
pass preview, apply, `skm check`, and no-op convergence before release.

Before SKM release, rollback is removal of the unreleased command changes.
After release, keep schema-2 parsing backward compatible and correct defects in
a new SKM release; do not mutate published Registry versions. Users can roll
back their project through version control because the operation records
ordinary explicit skill entries.

## Risks and Mitigations

- **The meaning of “all” changes over time:** exact package versions and the
  source revision bind each registry release; projects persist the expanded
  entries rather than a floating set.
- **Partial project mutation:** full preflight and transactional rollback cover
  configuration and links as one operation.
- **Existing-version conflicts:** reject the whole plan instead of silently
  replacing a user's pin.
- **Large config diffs:** deterministic ordering and explicit entries trade a
  larger file for auditable, reproducible state.
- **Cross-repository skew:** compatibility checks and the ordered rollout keep
  SKM from consuming an unpublished or incompatible bundle.

## Alternatives Considered

- **Scan every package under `skills/workspace/`:** rejected because directory
  contents are not a reviewed release contract and may contain stale data.
- **Run `skm add` repeatedly:** rejected because it cannot guarantee all-or-
  nothing behavior and creates poor conflict reporting.
- **Persist only `bundle: all-workspace-skills`:** deferred because existing
  commands operate on explicit skills and a floating bundle would obscure the
  exact project state.
- **Reuse toolkit bundles directly:** rejected for this interface because they
  are source-repository artifacts that may include profiles rather than a
  registry-published skill package set.

## Open Questions

None for backlog entry. Implementation may refine JSON field names, but it
must preserve the data and safety semantics defined here.

## Memory Impact

Status: `pending`

Rationale: The final command, manifest schema, transaction boundary, and
released compatibility are durable decisions, but they are not implemented or
released yet. Resolve this section and update project memory before the spec
can reach `done`.

## Amendment: Bundles From Any Namespace

Proposed 2026-09-24, with the matching Registry amendment "Generic Namespace
Manifests". Not yet agreed with this specification's owner.

The interface is already namespace-generic: `skm bundle add
<namespace>/<bundle>`. The released search reader already reads
`skills/<namespace>/manifest.yaml` for every namespace and requires no Workspace
provenance field. The remaining restriction is the non-goal "Supporting arbitrary
third-party namespace manifests in the first release".

Proposed replacement for that non-goal:

- Bundles resolve from any namespace manifest in a **configured** registry. The
  configured registry is the trust boundary; restricting by namespace name adds
  no protection, because every namespace in a registry is reviewed through the
  same process.
- Provenance checks apply where the manifest carries a provenance block, which
  the `workspace` namespace always does. A core-only manifest is validated for
  structure, exact versions, and membership, as the released reader already does.
- Registries the user has not configured remain out of scope.

Everything else in this specification is unchanged: explicit exact entries in
`skills.yaml`, no hidden bundle state, the plan-then-apply flow, no global scope,
and no automatic membership upgrades.

First consumer outside `workspace`: `skills-yaml/authoring-toolkit`, bundling
`skills-yaml/skill-creator` and `skills-yaml/skill-reviewer`.
