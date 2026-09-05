# Decisions

## 2026-06-17 - Adopt workspace-docs@1.0.0

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

skm adopts the workspace documentation standard at `workspace-docs@1.0.0`. Adoption is additive: preserve project-specific guidance and legacy specs, and keep generated agent context inside `AGENT-CONTEXT` markers.

## 2026-06-19 - Standardize Diagnostic Logs to Stderr

- Type: decision
- Source: review
- Confidence: high
- Review: none
- Supersedes: none

Content:

Align skm CLI command outputs with the UI brand and style guide. Diagnostic outputs, installation logs, cache warnings, and interactive confirmations are sent to standard error (stderr). Standard output (stdout) is reserved strictly for successful query outputs meant for piping, such as listing details in `skm list`.

## 2026-06-19 - Implement Skill Removal Feature

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented `skm remove <SKILL_NAME>` command to safely unlink skill directories from configured agent paths and programmatically update `./skills.yaml` config entries.

## 2026-06-19 - Implement Cleanup and Maintenance Feature

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented the `skm clean` suite of commands for repository and system maintenance: `skm clean symlinks` for pruning broken/orphaned links, `skm clean cache` for registry pruning (retaining dynamic versions), and `skm clean reset` for fresh states (with home/workspace backups).

## 2026-06-19 - Implement Local Development Mode

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented local development mode (`skm dev`) commands (`link`, `unlink`, `list`, `show`, `mode`) to enable skill developers to link local workspaces as dev skills and test them on multiple agents without publishing to registries first.

## 2026-06-19 - Implement Skill Version Management

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented semantic version management (`skm versions`, `skm use`, `skm update-skill`) to allow listing registry skill versions, pinning to a specific version in `skills.yaml`, and upgrading to the latest version dynamically.

## 2026-08-19 - Support Workspace Docs 5 Toolkits

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

SKM 0.2.1 accepts toolkit manifests declaring Workspace Docs 4.x or 5.x
compatibility. The explicit allowlist preserves toolkit 0.1 compatibility while
enabling toolkit 0.2 and the four-stage specification lifecycle.

## 2026-08-19 - Adopt workspace-docs@5.0.0

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: 2026-06-17 - Adopt workspace-docs@1.0.0

Content:

This repository adopts `workspace-docs@5.0.0`. Canonical locations are
`workspace/instructions/`, `workspace/specs/`, `workspace/docs/`,
`workspace/company/`, and `workspace/agents/memory/`. Spec lifecycle is
`backlog -> development -> test -> done`. The configured test target is
`development`; the configured production target is `main`. Generated agent
context is pinned to `workspace-docs@5.0.0`.
