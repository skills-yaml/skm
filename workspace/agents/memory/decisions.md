# Decisions

## 2026-09-26 - Keep Workspace pins out of interactive init

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

The optional `skm init` wizard step edits toolkit manifest/version, bundles,
and profiles, but does not prompt for Workspace standard, source, revision, or
integrity. Existing `workspace` manifest values remain intact during edits,
and explicit `--workspace-*` initialization flags stay available.

## 2026-09-25 - Confirm planned skill and bundle adds

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

`skm add` shows the planned configuration, resolved skills, and agent link
impact before an interactive confirmation for both individual skills and
published bundles. The default answer is No. `--yes` applies without a prompt;
bundle `--dry-run` and `--json` remain read-only previews. Non-interactive adds
need `--yes`.

## 2026-09-24 - Use one registry search and add interface for skills and bundles

- Type: decision
- Source: user
- Confidence: high
- Review: after development integration
- Supersedes: 2026-09-24 - Install published registry bundles as exact project skills (CLI command shape only)

Content:

SKM should show published manifest bundles and skills together in `skm search`
results and install either through `skm add`. The result kind stays visible;
`--kind skill|bundle` resolves an ID collision. Bundle installation retains the
exact-pin, project-only preview and rollback transaction, while `skm bundle
add` remains a compatibility command.

## 2026-09-24 - Install published registry bundles as exact project skills

- Type: decision
- Source: user
- Confidence: high
- Review: after development integration
- Supersedes: none

Content:

SKM handles Registry metapackage use cases through published schema-2 namespace
manifest bundles. `skm bundle add` expands the selected bundle and exact
same-registry dependencies into explicit project `skills.yaml` pins. The
instructionless dependency package removed from the Registry draft is not a
bundle. Project application previews the complete change and rolls back
configuration and managed links together on failure.

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

## 2026-09-10 - Resolve Exact Registry Skill Dependencies

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Registry skills may declare exact, same-registry dependencies through the
string-valued Agent Skills metadata keys `skm-version` and
`skm-dependencies`. SKM resolves and validates the complete closure before
writes, rejects cycles and conflicts, and records immutable package versions.
Codex skill projections use the shared `.agents/skills/` discovery path;
managed `.codex/skills/` entries are accepted only for safe migration.

## 2026-09-17 - Keep Search Read-Only

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: 2026-09-15 - Standalone registry search adds unique results (installation behavior only)

Content:

`skm search` is a discovery-only command and must not install or link skills.
It does not expose `--add` or the add-only `--global` option. Search results
direct users to `skm add <skill-name> --source <registry>`, which remains the
command responsible for manifest changes and skill linking.

## 2026-09-19 - Use sequential prompts for init

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: 2026-09-09 - Init edits manifests through a terminal wizard

Content:

`skm init` uses ordinary cooked-terminal, line-by-line prompts instead of a
full-screen TUI. Enter keeps the displayed value, `-` clears an optional value,
and `:q` cancels without writing. Small numbered menus cover agents,
registries, and skills; advanced toolkit/workspace fields are opt-in; and the
complete YAML is shown before save. Existing-file preservation, validation,
atomic replacement, concurrent-edit detection, and non-interactive creation
remain part of the contract.

## 2026-09-22 - Compile and deduplicate agent skill targets in SKM

- Type: decision
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

Agent skill-directory paths are compiled into SKM rather than supplied by a
registry. When multiple configured agents claim the same filesystem directory,
SKM resolves, writes, checks, lists, and records that directory once while
retaining every claimant in the lockfile.

## 2026-09-22 - Notify about updates on help requests

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Interactive SKM help requests, including `skm help` and `--help`, should show
the same available-update notice as ordinary commands before Clap prints help.
The notice remains best-effort, terminal-only, and controlled by the existing
startup-check opt-out.

## 2026-09-24 - Delegate Workspace structure management to skills

- Type: decision
- Source: user
- Confidence: high
- Review: after development integration
- Supersedes: none

Content:

SKM removes its dedicated `workspace audit|adopt|upgrade|repair` commands.
Repository assessment, adoption, upgrade, and repair are handled by the
published `workspace/wk-adopt` skill and its exact
`workspace/adopt-workspace-structure` dependency. SKM still installs skills and
toolkits and preserves `workspace:` pins and local source integrity in toolkit
lockfiles. Existing `.skm/workspace-plan.yaml` handoffs are left untouched.
Future registry bundle installation is proposed under generic `skm bundle add`
rather than a Workspace-only command.

## 2026-09-24 - Reserve SKM 0.7.0 For Registry Bundle Installation

- Type: decision
- Source: implementation
- Confidence: high
- Review: after production release
- Supersedes: none

Content:

The `skm bundle add` consumer is the first SKM release that can install
Registry namespace bundles. The existing production 0.6.0 binary lacks that
command, so the bundle-capable candidate uses version 0.7.0 and the generated
Workspace schema-2 manifest requires SKM 0.7.0 or newer. This decision does
not claim a production release.
