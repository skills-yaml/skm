# Show skill descriptions and available bundles in search

## Status

State: `test`

Rationale: Commit `91da59b8a6e7a2226a650dad481ed2609cbcb62c` integrated description and published-bundle discovery into `development`; CI `35871598722` and Release Artifacts `35871598742` passed. PR #42 merged the tested code into `main` at `be2581d98e81c79d0756c1ef36fb4ea584200fb1`, and production CI `35962268392` passed. The production release is held for qualification and reviewer approval.

## Problem

`skm search spec` gives no explanation of what the matching skill does. Bundle information is not visible, making it hard to tell whether a published bundle is available. The current Workspace registry manifest is schema 1 and publishes no bundles.

## Goals

- Display the selected `SKILL.md` description in human and JSON search results.
- Display bundle IDs and membership counts only when a configured registry actually publishes them.
- State clearly when no bundles are published by the selected registries.
- Keep search read-only and deterministic.

## Non-goals

- Installing bundles or treating toolkit bundles as registry skill bundles.
- Inferring bundle membership from directories.
- Changing the registry schema or publishing bundles in another repository.

## Design

Read bounded frontmatter from the selected skill version for each registry entry. Normalize descriptions for one-line terminal output. Read an optional registry namespace manifest and list only explicit `bundles` metadata; absence means none published. Remote discovery reads Git objects from a temporary shallow clone without writing the registry cache. JSON includes a description and bundle list.

## Acceptance criteria

- Local and remote registry search show a description for `spec` when its selected `SKILL.md` provides one.
- Missing or malformed descriptions do not invent metadata or hide the skill result.
- A schema-1 registry reports no published bundles; a schema-2 fixture displays declared bundles and member counts.
- `--json`, `--registry`, `--limit`, ordering, and read-only behavior remain stable.

## Affected areas

- `src/search.rs`, `src/wizard/prompt.rs`, search command tests, `README.md`, `workspace/specs/`

## Validation gates

- `task check`, `task test`, `git diff --check`
- Local and temporary Git fixture tests for description and bundle discovery

## Rollout and risk

Descriptions and manifests are untrusted input, so reads and display lengths are bounded. Bundle listing is informational until the separate bundle-install contract is implemented and published.

## Memory Impact

Status: `updated`

Rationale: The distinction between published registry bundles and toolkit bundles is recorded in `workspace/agents/memory/facts.md` and `workspace/agents/memory/changelog.md`.
