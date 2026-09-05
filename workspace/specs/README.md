# Development Specifications

Every non-legacy spec uses
`workspace/specs/<state>/<primary-feature>/<spec>.md`, where state is `backlog`,
`development`, `test`, or `done` and primary feature is lowercase hyphen-case.

The normal flow is `backlog -> development -> test -> done`. Test means
confirmed integration into the configured test branch or environment. Done
means confirmed production release. This repository's configured targets are:

- test: `development` (prerelease channel; equivalent to the conventional
  `develop` default)
- production: `main`

A branch name alone is not lifecycle evidence. Record the confirmed
integration or release event in the spec and in this catalog.

## Feature Categories

| Primary feature | Purpose |
| --- | --- |
| `workspace-governance` | Workspace adoption, documentation policy, lifecycle governance, and validation. |
| `workspace-toolkit` | SKM toolkit install, lockfile ownership, and Workspace Docs toolkit compatibility. |
| `configuration` | Programmatic and global configuration management. |
| `skill-lifecycle` | Skill removal and version selection. |
| `local-dev` | Local development skill linking and dev mode. |
| `registry` | Registry management commands. |
| `maintenance` | Cleanup and maintenance commands. |
| `updates` | Automatic update notification at launch. |

## Status Catalog

Update the catalog whenever a spec is created, moves state, changes feature, is
reopened, is superseded, or materially changes why it is in its state.

<!-- SPEC-CATALOG:START -->
| Spec | Primary feature | State | Status rationale |
| --- | --- | --- | --- |
| [adopt-workspace-docs-5.md](development/workspace-governance/adopt-workspace-docs-5.md) | `workspace-governance` | `development` | Local implementation of the 1.x-to-5.0.0 migration is active; not merged to `development` or released through `main`. |
| [workspace-docs-5-compatibility.md](development/workspace-toolkit/workspace-docs-5-compatibility.md) | `workspace-toolkit` | `development` | SKM 0.2.1 compatibility is implemented locally on a feature branch; not merged to `development` or released through `main`. |
| [skill-removal-conformance.md](development/skill-lifecycle/skill-removal-conformance.md) | `skill-lifecycle` | `development` | Removal safety and partial-success corrections are implemented and validated locally; not integrated into `development`. |
| [cleanup-safety-conformance.md](development/maintenance/cleanup-safety-conformance.md) | `maintenance` | `development` | Cleanup data-preservation corrections are implemented and validated locally; not integrated into `development`. |
| [workspace-toolkit-manager.md](done/workspace-toolkit/workspace-toolkit-manager.md) | `workspace-toolkit` | `done` | Released through `main` in SKM 0.2.0. |
| [config-management.md](done/configuration/config-management.md) | `configuration` | `done` | Released through `main`. |
| [global-env-auto-config.md](done/configuration/global-env-auto-config.md) | `configuration` | `done` | Released through `main`. |
| [local-dev-mode.md](done/local-dev/local-dev-mode.md) | `local-dev` | `done` | Released through `main`. |
| [cleanup-commands.md](done/maintenance/cleanup-commands.md) | `maintenance` | `done` | Released through `main`. |
| [registry-management.md](done/registry/registry-management.md) | `registry` | `done` | Released through `main`. |
| [skill-removal.md](done/skill-lifecycle/skill-removal.md) | `skill-lifecycle` | `done` | Released through `main`. |
| [skill-version-management.md](done/skill-lifecycle/skill-version-management.md) | `skill-lifecycle` | `done` | Released through `main`. |
| [auto-update-notification.md](done/updates/auto-update-notification.md) | `updates` | `done` | Released through `main`. |
<!-- SPEC-CATALOG:END -->
