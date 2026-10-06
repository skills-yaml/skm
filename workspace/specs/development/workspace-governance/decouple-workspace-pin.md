# Decouple Workspace governance from the SKM manifest

## Status

State: `development`

Rationale: The user accepted removing the remaining Workspace governance coupling from skills.yaml. Implement and review the repository gate/configuration cleanup before claiming any integration.

## Problem and Scope

`task workspace:check` requires `skills.yaml` to contain a Workspace Docs 7.0.0
pin even though SKM treats `workspace` and `trusted_sources` as opaque publisher
metadata. A valid registry-only manifest therefore fails contributor governance
validation. Governance is already pinned by the generated `AGENTS.md` context
and contained standard aliases under `workspace/instructions/standards/`.

The user's `fix` accepts the preceding explicit proposal to remove this coupling
and authorizes the necessary project-structure guidance reconciliation. Follow up
the completed [publisher-independent format contract](../../done/registry/registry-format-ownership.md)
and [Workspace Docs 7 adoption](../../done/workspace-governance/adopt-workspace-docs-7.md).
Preserve their history and every released standard file.

Scope: repository Workspace validator/tests, committed example manifest,
current project-structure/validation/reference guidance, this spec/catalog,
durable memory, and removal of the two obsolete root metadata blocks from the
local staged/working manifest. Retain generic optional toolkit/bundle/profile
installation and runtime preservation of arbitrary publisher metadata.

## Acceptance Criteria

1. Workspace validation passes for a valid governed repository without
   `skills.yaml` and with a registry-only manifest. Stale or arbitrary legacy
   publisher metadata cannot select or override the governance version.
2. Generated AGENTS context and contained `default`/`latest` aliases still
   enforce 7.0.0; stale context, old/absolute aliases, and incomplete standard
   packages remain rejected. No additional governance configuration is invented.
3. The committed example and local staged/working manifests omit only root
   `workspace` and `trusted_sources` blocks. Preserve other staged/working
   values, including distinct skill versions; preserve `.gitignore`, the
   installation lockfile, installed links, and all unrelated changes.
4. Current docs explain the canonical governance declarations and SKM manifest
   independence. Runtime format parsing, generic toolkit/bundle support, CLI,
   dependencies, standard packages, AGENTS bytes, and release controls are unchanged.
5. Regression tests fail before the fix and pass afterward. Run changed
   Workspace tests/checks first, then affected version/coordination consumers,
   followed by independent exact-candidate review and frozen aggregate gates.
6. Reconcile spec/catalog/memory truthfully; test requires confirmed development
   integration, while done requires separately confirmed main completion.

## Source / Destination / Merge Map

| Source | Destination | Merge decision |
| --- | --- | --- |
| Duplicate `skills.yaml` governance requirement in validator | Existing AGENTS context and Workspace standard alias checks | Remove only the SKM-manifest pin requirement; retain the already-authoritative governance and generic repository privacy checks. |
| Root `workspace` / `trusted_sources` metadata | Removed from committed example and local staged/working skills.yaml | Delete only these named blocks; preserve all other bytes/values and existing opaque-metadata runtime behavior. |
| Validator fixtures | `scripts/test_validate_workspace.py` | Seed governance independently of a skill manifest; cover ordinary/missing manifests, ignored legacy metadata, and retained stale-context/alias failures. |
| Current manifest-pin documentation | README, project-structure guidance, validation module contract | Describe AGENTS and Workspace aliases as authoritative, preserving all other project facts. |
| Durable ownership decision | Memory decisions and changelog | Append the removal of the repository-level manifest coupling; preserve historical records. |
| This follow-up | Spec catalog and lifecycle records | Record actual integration events; never derive state from branch names. |

## Implementation and Validation Plan

1. Register this full contract and catalog row before implementation.
2. Reproduce the coupling with a failing regression under `task workspace:test`.
3. Remove the duplicate requirement, clean the committed example, and reconcile
   current guidance. Preserve all existing real governance checks.
4. Pass `task workspace:test` and `task workspace:check` before running downstream
   version/coordination modules. Add no runtime dependencies or custom YAML parser.
5. Resolve knowable memory and acceptance evidence, obtain independent review,
   freeze the candidate, and pass `task check` / `task test`.
6. Deliver through the configured development route. Reconcile local metadata
   removal using captured staged/working contents without publishing personal
   selections; validate the actual combined revision and report lifecycle status.

## Risks and Recovery

Removing the wrong declaration could weaken governance: retain exact generated
context, complete pinned package, and contained alias checks with negative tests.
Rewriting personal configuration could lose staged distinctions: snapshot the
two layers, remove only the named blocks, and compare unaffected content plus
lockfile/ignore bytes. Keep recovery data local and untracked. Future changes
to governance ownership use linked follow-ups; preserve done history.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | none | none | Repository-only gate, fixture, and example cleanup changes no SKM runtime, public installation format, or binary behavior. No release reservation or product version bump is needed. |

## Memory Impact

Status: `updated`

Rationale: The removed coupling and canonical governance declarations are recorded in `workspace/agents/memory/decisions.md` and `workspace/agents/memory/changelog.md`; unrelated SKM format and historical standard ownership remain intact.

## Acceptance and Delivery Evidence

- AC-1/5: The regression failed before implementation with six coupling-related
  failures across missing, plain, and legacy-metadata manifests. After the fix,
  all fifteen Workspace tests pass, including the valid fixture without a skill
  manifest and three publisher-metadata variants. Workspace check passes.
- AC-2: Tests still reject stale AGENTS context and stale/absolute aliases for
  both default and latest; complete standard-package checks remain unchanged.
- AC-3: The committed example omits both obsolete metadata blocks. Cleaned local
  staged/working replacements are prepared from captured originals and verified
  to preserve every other SKM value; apply and byte-verify during delivery.
- AC-4: Scoped inspection confirms only the manifest pin requirement was removed;
  generic privacy checks, runtime parsing, optional local toolkits, standard
  packages, and generated/manual AGENTS policy are retained. Current docs agree.
- AC-6: Catalog and durable decision/changelog are reconciled before review.
  Independent exact-candidate review and final aggregate results are reported at
  handoff after execution; relevant edits renew evidence.

Shared-test integration and main completion are unconfirmed for this follow-up.
Personal metadata cleanup is finalized at delivery without publishing personal
configuration. There is no SKM runtime version bump or requested production change.
