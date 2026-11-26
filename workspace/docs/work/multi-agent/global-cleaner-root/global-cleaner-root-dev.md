---
schema_version: 1
coordination_id: global-cleaner-root
agent_id: global-cleaner-root-dev
role: implementer
status: active
base_revision: b308d3a6dc6203b38fdacdc7bab6b5eed6e4615e
task_ref: detached
branch_authorization: none
updated_at: 2026-11-26T16:08:35Z
scope:
  - src/cleaner.rs
  - tests/global_cleanup_paths.rs
---

# Peer Work Record

## Assignment

Repair global cleanup origin checks for symlinked home ancestors, covering AC-2 and AC-5.

## Actions

Reused the shared manifest scope selection for orphan cleanup and global
dependency protection during cache pruning. Added an isolated Unix regression
covering both home spellings, pinned dependencies, unchanged previews, and
conflicting origins.

## Validation

Before the fix, the new regression failed with a different-or-missing origin
error. After the fix, the aggregate test suite, cache suite, and global-home
suite passed. Final candidate gates are run at handoff.

## Blockers and Dependencies

Independent exact-revision review and combined integration remain required.
The existing shared 0.10.0 release reservation is reused.

## Next Step

Complete final candidate gates and request independent review.

## Handoff

Memory impact: none for this bounded repair; it implements the existing
global-manifest contract without a new durable decision.
