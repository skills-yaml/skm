# Notify The Registry After A Production Release

## Status

State: `development`

Rationale: The user asked on 2026-09-26 that every SKM release notify the
Registry so its documentation is updated. Implementation is active on
`feat/notify-registry-docs`.

## Problem and Users

The Registry README, `VERSIONING.md`, `SKILL_STRUCTURE.md` and the
skills-yaml.tech site document SKM commands, but nothing tells the Registry
when SKM publishes a production release. Documentation maintainers find out
only when a user hits a removed or renamed command.

## Goals

- After a successful production release, send a `skm-released`
  `repository_dispatch` to `skills-yaml/registry`, whose `Upstream Release`
  workflow opens a `docs-review` issue with a help diff.
- Never let the notification affect the release or its qualification.

## Non-goals

- Notifying on `development` pre-releases.
- Sending release data the Registry must trust; it reads `skm-release.json`
  from `prod-latest` itself.
- Changing the build, publication, or qualification jobs.

## Design

Add a `notify-registry` job to `.github/workflows/release.yml` that needs
`prepare` and `release`, runs only when the channel is `prod` and the
`REGISTRY_APP_CLIENT_ID` repository variable is set, and uses
`continue-on-error`. It creates a token for the existing
`workspace-registry-publisher` GitHub App, limited to the Registry with
`contents: write` (the permission `repository_dispatch` requires), and sends
the event with the release commit. The job has no repository permissions and
no deployment environment, so it adds no approval gate and does not change the
`release-prod` job count that qualification checks.

Setup outside this repository: add the `REGISTRY_APP_CLIENT_ID` repository
variable and the `REGISTRY_APP_PRIVATE_KEY` repository secret, with the same
values the Workspace repository uses. Until then the job is skipped and the
Registry's hourly poll opens the issue.

## Affected Areas

- `.github/workflows/release.yml`
- `tests/release_contract.rs`
- `workspace/specs/README.md`
- `workspace/agents/memory/decisions.md`, `workspace/agents/memory/changelog.md`

## Compatibility and Risks

No CLI or artifact change. A wrong or missing credential makes only this job
fail or skip; `continue-on-error` keeps the run's conclusion, which
qualification reads, unchanged.

## Acceptance Criteria

1. A production release run ends with a `notify-registry` job that sends
   `skm-released` to `skills-yaml/registry`.
2. Development releases skip the job.
3. Without the variable the job is skipped, and a failing dispatch leaves the
   release run successful.
4. The release contract test asserts the dispatch and its production-only
   condition.

## Validation Gates

- `task check`, `task test`, and `git diff --check` pass.
- `actionlint` reports no new finding in `release.yml`.
- After the next production release, the Registry receives the event and
  opens `Docs review: skm <version>`.

## Memory Impact

Status: `updated`

Rationale: The user's decision that production releases notify the Registry is
recorded in `workspace/agents/memory/decisions.md` with a matching
`workspace/agents/memory/changelog.md` entry.
