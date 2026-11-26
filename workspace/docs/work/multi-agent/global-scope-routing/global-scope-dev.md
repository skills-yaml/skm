---
schema_version: 1
coordination_id: global-scope-routing
agent_id: global-scope-dev
role: implementer
status: handoff
base_revision: e81eed3352c7ee4d8bdae29349b0f4dcb3ac342d
task_ref: detached
branch_authorization: none
updated_at: 2026-11-26T14:32:25Z
scope:
  - src/config.rs
  - src/config_editor.rs
  - src/config_manager.rs
  - src/linker.rs
  - src/main.rs
  - src/registry.rs
  - src/wizard
  - tests
  - workspace/specs/development/configuration/global-user-installation.md
---

# Peer Work Record

## Assignment

Implement S2 user-manifest selection, separate source and destination roots,
and preserve project/base-settings behavior; use the shared 0.10.0 release.

## Actions

Created an isolated worktree through an atomic peer claim. Routed global
manifest-backed commands, non-interactive and interactive global init, registry
selection, relative local add paths, and previews through the selected scope.
Kept the existing base-settings interface distinct and protected global
previews from unsafe manifest types. Updated an older command fixture that
assumed global install read the project manifest. Independent review rejected
revision 5ee36ed577aa7c82b85fd14dcaaf91528dfcdc62 for unsafe preview
ancestors, missing global-add scope validation and unscoped guidance. Reworked
the candidate to validate home, manifest, cache and registry-directory parents
without creating state, reject project-only global manifests before discovery,
and report scope-specific commands.

## Validation

Partial AC2, AC3 and AC7: isolated home and two-project command fixtures cover
manifest isolation, conflicting registry maps and inherited settings, selected
agents, cancellation and metadata, stable local paths, version inspection and
byte-identical previews. Added rejected-candidate regressions for symlinked
manifest/cache ancestors, toolkit manifests on local and registry adds, and
global init/install guidance. Focused global-home, registry-format, bundle,
toolkit, and cache Taskfile modules passed. Full `task check` and `task test` passed;
review and final candidate gates remain required after subsequent fixes.

## Blockers and Dependencies

Global outdated refresh fails safely and gives a scoped workaround: the existing
version manager passes no manifest root to registry refresh. A separate dependent
scope-refresh task owns that interface. S3 owns global install transaction and
all-agent behavior; S4 owns mutation consistency and cleanup. This step does not
claim complete AC2, AC3, AC4, AC5 or AC7. The applied shared version reservation
remains 0.10.0.

## Next Step

Obtain independent exact-candidate review and rerun gates after any fixes.
Integrate only through the serialized peer workflow.

## Handoff

Pending independent review. Memory impact: none for this bounded work item;
the spec already records the planned scope contract, and resulting behavior
remains subject to the dependent tasks and release-wide reconciliation.
