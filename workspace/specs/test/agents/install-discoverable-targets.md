# Install discoverable agent skill targets

## Status

State: `test`

Rationale: Commit `91da59b8a6e7a2226a650dad481ed2609cbcb62c` integrated the fix into `development`; CI `35871598722` and Release Artifacts `35871598742` passed. PR #42 merged the tested code into `main` at `be2581d98e81c79d0756c1ef36fb4ea584200fb1`, and production CI `35962268392` passed. The production release is held for qualification and reviewer approval.

## Problem

`skm install` reports success when `agents: []` or when the only project agent is Hermes, whose project target is unavailable. A package such as `workspace/wk-spec` is linked below `skills/workspace/wk-spec`, although agent skill discovery generally expects a direct child of `skills`.

## Goals

- Fail before registry or filesystem work when a skills-only install is configured but the selected scope has no agent targets.
- Install namespaced registry skills as direct children of each selected agent skills directory.
- Reject target-name collisions before writing and preserve existing real files.
- Keep list, check, remove, and toolkit output paths consistent with install.
- Document the supported agent target list and scope caveats.

## Non-goals

- Changing the sixteen supported agent identifiers or adding a new agent.
- Claiming that every supported agent has been exercised end to end.
- Changing the registry package identity or version layout.

## Design

The target directory name is the final component of the validated package name, matching the skill's portable `name` frontmatter. Validate that two resolved packages cannot claim the same target name. Resolve effective targets before mutations for skills-only installation. For a project with only Hermes, report that Hermes is global-only. Toolkit reconciliation keeps its existing behavior because it may remove previously managed outputs when no target remains. Existing nested links remain legacy links; unmanaged paths remain untouched.

## Acceptance criteria

- Empty-agent and project-only-Hermes skills-only installs of nonempty skill sets fail with an actionable message and create no links.
- `workspace/wk-spec` links at `<agent>/skills/wk-spec`, and check/list use that location.
- Two packages ending in the same skill name fail before mutation.
- Existing unrelated files and symlinks are preserved.
- Agent scope limitations are stated in README.

## Affected areas

- `src/main.rs`, `src/linker.rs`, `src/cleaner.rs`, relevant command tests
- `README.md`, `workspace/specs/`, `workspace/agents/memory/` if durable facts change

## Validation gates

- `task check`, `task test`, `git diff --check`
- Temporary-directory install, collision, list/check, and removal tests

## Rollout and risk

The flat target path changes where namespaced packages appear. Toolkit install reconciles managed links through its lockfile. Skills-only install leaves older nested links untouched; users may remove them after confirming ownership. Existing configuration remains valid.

## Memory Impact

Status: `updated`

Rationale: The effective-target and flat-layout contract is recorded in `workspace/agents/memory/facts.md` and `workspace/agents/memory/changelog.md`.
