---
schema_version: 1
coordination_id: global-remove-closure
agent_id: global-remove-closure-dev
role: implementer
status: active
base_revision: 1a1369082415c57d6dd8d0dc250c885683527af1
task_ref: detached
branch_authorization: none
updated_at: 2026-11-26T16:39:26Z
scope:
  - src/remover.rs
  - tests/bundle_maintenance.rs
  - tests/global_home.rs
---

# Peer Work Record

## Assignment

Repair global removal ownership and protect project bundle removal from concurrent manifest edits.

## Actions

Created an isolated worktree through an atomic peer claim. Plan orphan removal from the resolved closure difference; preserve shared dependencies, reject changed project manifests before writes, and permit uncached removals when all configured targets are empty. Following independent review, recheck the lock including absence, recompute target plans, and compare symlink snapshots after confirmation before project bundle mutations. A second review found path substitutions: now reject a symlinked manifest and validate planned target parents after confirmation.

## Validation

The original candidate reproduced both global failures in temporary-home fixtures. The first handoff reproduced two additional review findings involving lock changes and symlink replacement during confirmation; interactive fixtures now pass. Subsequent manifest and target-parent symlink substitution regressions also reproduced and pass. `task test:global-home`, `task test:bundle`, `task check`, `task test`, and `task versions:check` passed. Review and final combined-candidate gates remain pending.

## Blockers and Dependencies

An uncached declaration cannot be removed when other global targets exist and ownership cannot be resolved; the operation fails without changes. Independent exact-candidate review remains required.

## Next Step

Commit and hand off for independent review; rerun gates after any review fixes.

## Handoff

Shared SKM 0.10.0 reservation is reused. Memory impact: none; these fixes implement already-recorded behavior and add no durable decision.
