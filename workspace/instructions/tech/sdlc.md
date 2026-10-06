# Software Development Lifecycle

This repository follows `workspace-docs@7.0.0`. Read the complete
[pinned SDLC](../standards/workspace-docs/v7.0.0/sdlc.md),
[process](../standards/workspace-docs/v7.0.0/process.md), and
[versioning contract](../standards/workspace-docs/v7.0.0/versioning.md).

## Project Workflow and Authority

Non-trivial changes require a development spec with scope, acceptance criteria,
affected areas, implementation plan, risks, validation gates, Version Impact,
and Memory Impact initialized to pending, plus a matching root catalog row.

- `development` is the configured shared test/prerelease target.
- `main` is the verified completion target and production publication source.
- The default route is backlog -> development -> test -> done. No direct-to-main
  exception is configured. A branch name alone never proves integration.
- Done requires verified main merge and reconciled acceptance, verification,
  documentation, catalog, version records, and memory. Deployment/publication
  are separate events; an applied version suffices for completion.
- Block from backlog, development, or test using Previous State, Block Kind,
  Block Reason, and Resume Condition. Resume there and renew stale evidence.
- Preserve historical done specs; create linked follow-up specs for new work.
  Explicit older migration specs retain their original completion contracts.
  Existing test specs require individual acceptance assessment before transition.

A clear task request authorizes ordinary task branches (`feat/<name>` or
`fix/<name>`), commits, pushes, PR updates, safe CI repair, shared-test integration,
main merge, and non-destructive release, subject to repository protections.
Governed instructions require prospective approval naming their scope; a named
Workspace migration supplies that approval for its necessary instruction edits.
Destructive production actions require separate scoped approval. Never bypass
host, branch, environment, or credential protections.

## Review and Verification

All changes receive self-review. Non-trivial changes require independent agent
or human review of the exact candidate, except small low-risk changes with
focused tests. Security, data integrity, public interfaces, governed instructions,
and release controls always require independent review. Resolve every finding
through implementation or reviewer-agreed documented rejection; relevant edits
renew affected review. Repository protections may additionally require humans.

Run affected Taskfile modules during implementation and review fixes, including
transitive consumers. The [module contracts](../../validation/README.md) define
commands, inputs, isolation, pass conditions, and freshness. Unknown scope uses
aggregate fallback; final evidence reuse is disabled in this repository.
After records, documentation, artifacts, review, and fixes stabilize, freeze the
candidate and run `task check` and `task test`. Later relevant edits renew review
and verification; tracked result records are not automatically exempt.

Every acceptance criterion needs recorded test, inspection, demonstration, or
user-decision evidence. Verify the actual combined shared-test revision and the
resulting main revision; contributor results do not prove another revision.
Repair failed gates. Record concise candidate-bound results without raw logs.

## Multi-Agent Coordination

When two or more agents may mutate, use the repository coordination skill and
portable runtime for atomic claims, dedicated detached linked worktrees, and
bounded, possibly overlapping scopes. Keep the primary checkout coordination-only.
Maintain task indexes and individual WIP records under
`workspace/docs/work/multi-agent/`. Read-only reviewers need no worktree.
Ordinary task branches follow task authority; preserve older direct-request
records and existing worktrees. The runtime still creates detached worktrees.

The peer board, lock, worktrees, and recovery archives stay local to the Git
common directory. Internal integration is not shared-test integration, main
completion, or publication. Preserve dirty work and unique commits on interruption.

## Version Planning and Memory

Every non-legacy spec declares per-component Version Impact before implementation.
`workspace/releases.json` records one logical owner and one open target per
component. Pre-1.0 SKM uses minor for incompatible changes/additions and patch
for compatible fixes; an explicit stable-contract decision advances to 1.0.0.
Check shared reservations before selecting targets and at handoff; reuse an
already-applied shared candidate without another bump. Apply once at the declared
development-start or merge boundary before integration artifacts. Test/done need
applied versions; publication separately marks reservations released with evidence.
The native version gate checks `[package].version` and its Cargo.lock mirror.

Classify completed tasks as updated or none with rationale. Updated durable
context goes to its memory category plus changelog; pending remains only while
unresolved. Reconcile memory before done and state classification at handoff.

## CI, Quality, and Commits

CI validates PRs and pushes to both `development` and `main` through `task check`,
`task test`, and `task build`. The separate publication workflow does not replace
PR validation. See [CI guidance](./ci.md) for release channels and protections.

Formatting, warnings-free Clippy, deterministic Rust unit/integration tests,
and filesystem safety remain mandatory. Use Taskfile entrypoints only.
Iterative checkpoint commits may rely on affected modules; complete final evidence
is required before a push or PR handoff presented for acceptance.
Conventional commits use `type(scope): description`, for example
`chore(workspace): adopt Workspace Docs 7.0.0`.
