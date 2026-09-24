# Delegate Workspace adoption to skills

## Status

State: `done`

Rationale: Commit `5152bc1` integrated the skill-led Workspace CLI change into `development`. PR #51 promoted SKM 0.7.0 through `main`, and PR #52's release-evidence commit `44a502af733e1ef3e0a360fe6e00d72959be261d` passed production CI `36007217965`, four-platform update qualification `36008025230`, and production Release Artifacts run `36007218144` on 2026-09-24. The nine published assets, archive checksums, manifest, and production binary identity were verified.

## Problem

`skm workspace audit|adopt|upgrade|repair` validates a Workspace Docs package and writes a handoff plan, while the published `wk-adopt` and `adopt-workspace-structure` skills already own assessment and migration. The extra CLI workflow creates overlapping interfaces and makes SKM responsible for Workspace-specific migration decisions.

## Goals

- Remove the `skm workspace` command and its handoff implementation from the CLI.
- Direct users to the published `workspace/wk-adopt` skill for Workspace assessment, adoption, upgrade, and repair; SKM installs its exact dependency through normal skill resolution.
- Preserve toolkit installation, lockfile Workspace pins, local source integrity, and their safety rules.
- Keep legacy `.skm/workspace-plan.yaml` files untouched; document that the removed command no longer creates or updates them.
- Remove the obsolete `skm init --trusted-source` input and wizard prompt while preserving existing `trusted_sources` manifest values for compatibility.
- Update current help, documentation, tests, and the future bundle-command proposal to avoid depending on the removed namespace.

## Non-goals

- Removing `workspace:` configuration or toolkit bundle/profile support.
- Changing published Workspace skills or the Workspace Docs package.
- Implementing bundle installation in this change.
- Rewriting historical release evidence in completed specs.

## Design

Remove the Clap `Workspace` subcommand and orchestration. Retain only the local source-validation and integrity code needed by toolkit locking in a focused module. Replace CLI migration tests with focused tests for that retained source-integrity boundary and a CLI parse regression asserting `workspace` is unavailable. Explain the skill-based adoption path in README, including current agent target requirements and dependency resolution. The proposed group-install command in the backlog specification uses a generic bundle namespace.

## Affected areas

- `src/main.rs`, `src/workspace.rs` or its replacement helper, `src/toolkit.rs`, `src/wizard/prompt.rs`, CLI and source-integrity tests.
- `README.md`, `workspace/specs/README.md`, the backlog bundle spec, and project memory.

## Compatibility and migration

Scripts invoking `skm workspace` must move to an agent with `workspace/wk-adopt` installed. The command removal is an intentional CLI break; `skm install`, `skm check`, toolkit configuration, and existing lockfiles remain supported. Existing `trusted_sources` values remain readable but no longer authorize anything in SKM. Existing handoff files are left in place for user review or cleanup.

## Risks

- Removing the command can strand scripts; README provides the replacement steps.
- Extracting integrity helpers can change lockfile digests or symlink checks; preserve the existing algorithm and test it with allowed version pointers and unsafe links.

## Acceptance criteria

- Top-level help no longer lists `workspace`; `skm workspace ...` fails as an unknown subcommand.
- `skm add workspace/wk-adopt --source default` plus ordinary installation resolves and links its exact `adopt-workspace-structure` dependency for an effective agent target.
- Toolkit lockfile planning still records the same local Workspace source integrity and rejects unsafe source paths or links.
- Current documentation describes skill-led adoption and no longer recommends the removed CLI or future Workspace-only bundle command.
- `skm init` no longer offers `--trusted-source` or prompts for a setting without an SKM consumer; existing manifest values survive ordinary edits.
- Existing unrelated local configuration remains untouched.
- `task check`, `task test`, `task build`, and `git diff --check` pass.

## Validation gates

- Taskfile check, test, and build commands.
- CLI help/parse assertions and focused local source-integrity fixture tests.
- Local skill-install smoke using a temporary project and cached registry where available.

## Memory Impact

Status: `updated`

Rationale: The durable CLI boundary and migration path are recorded in `workspace/agents/memory/decisions.md` and `workspace/agents/memory/changelog.md`.
