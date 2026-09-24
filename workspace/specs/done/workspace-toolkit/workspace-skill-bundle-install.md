# Install Registry Skill Bundles

## Status

State: `done`

Rationale: Registry PR #14 published the schema-2 Workspace bundle from
source `bd958c01249e1cb1e1fbbcf91ba70e83abec8d5f`; the SKM 0.7.0
development binary passed the live Registry qualification below. PR #51
promoted the consumer through `main`. PR #52's release-evidence commit
`44a502af733e1ef3e0a360fe6e00d72959be261d` passed production CI
`36007217965`, four-platform update qualification `36008025230`, and
production Release Artifacts run `36007218144` on 2026-09-24. The nine
published assets, archive checksums, manifest, and production binary identity
were verified.

The Registry bundle contract uses `minimum_skm_version: 0.7.0` because the
previous production 0.6.0 binary does not contain bundle installation.

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
- Reading bundles from unconfigured registries.
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

## Development Validation (2026-09-24)

`task check`, `task test`, `task build`, and `git diff --check` passed locally.
Tests cover schema-2 bundle and Workspace manifest validation, exact dependency
expansion, local and Git registry preview/apply behavior, explicit pins,
extension-field preservation, no-op repeats, target collisions, concurrent
configuration edits, symlinked parents, and injected rollback. The released
Registry now has a schema-2 Workspace manifest.

## Published Registry Qualification (2026-09-24)

Workspace workflow run `36005593693` published source revision
`bd958c01249e1cb1e1fbbcf91ba70e83abec8d5f` through Registry PR #14,
merged at `19a07659631db05decd7b03600ee503679e19414`. In an isolated
temporary project, the checksum-verified Linux binary from SKM development
Release Artifacts run `35994384028` identified itself as 0.7.0 at
`2f882e5ef7e2724d4d9f504b70e9059a50d61fe1`. Search found the published
`workspace/all-workspace-skills` bundle with 20 members. Dry-run add left the
project unchanged; apply recorded 20 exact Workspace pins; `skm check` passed;
and repeating add left `skills.yaml` unchanged.

## Development Integration (2026-09-24)

PR #44 merged into `development` at
`29ba7dc77315be65e6e77d4d6932fbbc9018627d`. Its Validate check passed.
This confirmed integration permits the `development -> test` transition;
the released development binary has since passed live Registry qualification.
The exact-commit release update qualification passed in run `36008025230`.

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

No unresolved SKM interface decision or delivery dependency.

## Memory Impact

Status: `updated`

Rationale: The user selected Registry namespace-manifest bundles over
instructionless dependency metapackages. The project-only exact-pin expansion
and transactional application decision is recorded in
`workspace/agents/memory/decisions.md` and
`workspace/agents/memory/changelog.md`. Registry published schema-2 bundles,
the SKM consumer passed live compatibility checks, and the 0.7.0 production
release was verified.

## Amendment: Bundles From Any Namespace

Accepted for this implementation on 2026-09-24 by the user's request to support
Registry's manifest bundles. The matching Registry amendment is tracked in
Registry PR #10.

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
