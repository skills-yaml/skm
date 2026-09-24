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
| `configuration` | Interactive, programmatic, and global configuration management. |
| `skill-lifecycle` | Skill removal and version selection. |
| `local-dev` | Local development skill linking and dev mode. |
| `registry` | Registry management commands. |
| `maintenance` | Cleanup and maintenance commands. |
| `updates` | Automatic update notification at launch. |
| `agents` | Agent link targets and per-agent skill directory integration. |

## Status Catalog

Update the catalog whenever a spec is created, moves state, changes feature, is
reopened, is superseded, or materially changes why it is in its state.

<!-- SPEC-CATALOG:START -->
| Spec | Primary feature | State | Status rationale |
| --- | --- | --- | --- |
| [remove-workspace-cli.md](development/workspace-toolkit/remove-workspace-cli.md) | `workspace-toolkit` | `development` | User requested skill-led Workspace adoption and removal of the dedicated CLI command; implementation is active and integration is unconfirmed. |
| [search-skill-details-and-collections.md](done/registry/search-skill-details-and-collections.md) | `registry` | `done` | PR #42 released the search details through `main` at `9c8153a`; production CI, four-platform qualification, nine-asset publication, checksum, and binary smoke passed. |
| [install-discoverable-targets.md](done/agents/install-discoverable-targets.md) | `agents` | `done` | PR #42 released discoverable install targets through `main` at `9c8153a`; production CI, four-platform qualification, and verified publication passed. |
| [search-metadata-and-bundles.md](done/registry/search-metadata-and-bundles.md) | `registry` | `done` | PR #42 released search metadata through `main` at `9c8153a`; production CI, four-platform qualification, and verified publication passed. |
| [help-update-notice.md](done/updates/help-update-notice.md) | `updates` | `done` | PR #40 released the tested help notice through `main` at `a6cdef2`; production CI, four-platform publication, manifest verification, and an interactive Linux help smoke passed. |
| [workspace-docs-6-compatibility.md](done/workspace-toolkit/workspace-docs-6-compatibility.md) | `workspace-toolkit` | `done` | PR #40 released the tested compatibility through `main` at `a6cdef2`; production CI and the verified complete four-platform publication passed. |
| [workspace-skill-bundle-install.md](backlog/workspace-toolkit/workspace-skill-bundle-install.md) | `workspace-toolkit` | `backlog` | Proposed interface is generic `skm bundle add`; canonical bundle publication and consumer implementation have not started. |
| [init-sequential-prompts.md](done/configuration/init-sequential-prompts.md) | `configuration` | `done` | PR #32 released SKM 0.6.0 through `main` at `769f5a4`; production CI, four-platform update qualification, publication, checksum, binary identity, and manifest verification passed. |
| [nib-release-update-parity.md](done/updates/nib-release-update-parity.md) | `updates` | `done` | PR #32 released the corrected updater at `769f5a4`; required-reviewer gating held production until all four platforms passed qualification, then the complete manifest-bound release was verified. |
| [adopt-workspace-docs-5.md](done/workspace-governance/adopt-workspace-docs-5.md) | `workspace-governance` | `done` | Released through `main` by PR #4 on 2026-09-05 (`e12ba7d`); `prod-latest` artifacts published successfully. |
| [workspace-docs-5-compatibility.md](done/workspace-toolkit/workspace-docs-5-compatibility.md) | `workspace-toolkit` | `done` | Released through `main` by PR #4 on 2026-09-05 (`e12ba7d`); `prod-latest` artifacts published successfully. |
| [registry-published-workspace-skills.md](done/workspace-toolkit/registry-published-workspace-skills.md) | `workspace-toolkit` | `done` | PR #13 released SKM 0.4.0 through `main` at `cd3d4a6e5d71dd3f27e8122e8cd8b6b323b15cb4`; CI and platform publication passed, and the Linux checksum and version were verified. |
| [skill-removal-conformance.md](done/skill-lifecycle/skill-removal-conformance.md) | `skill-lifecycle` | `done` | Released through `main` by PR #4 on 2026-09-05 (`e12ba7d`); `prod-latest` artifacts published successfully. |
| [cleanup-safety-conformance.md](done/maintenance/cleanup-safety-conformance.md) | `maintenance` | `done` | Released through `main` by PR #4 on 2026-09-05 (`e12ba7d`); `prod-latest` artifacts published successfully. |
| [workspace-toolkit-manager.md](done/workspace-toolkit/workspace-toolkit-manager.md) | `workspace-toolkit` | `done` | Released through `main` in SKM 0.2.0. |
| [config-management.md](done/configuration/config-management.md) | `configuration` | `done` | Released through `main`. |
| [global-env-auto-config.md](done/configuration/global-env-auto-config.md) | `configuration` | `done` | Released through `main`. |
| [local-dev-mode.md](done/local-dev/local-dev-mode.md) | `local-dev` | `done` | Released through `main`. |
| [cleanup-commands.md](done/maintenance/cleanup-commands.md) | `maintenance` | `done` | Released through `main`. |
| [registry-management.md](done/registry/registry-management.md) | `registry` | `done` | Released through `main`. |
| [skill-removal.md](done/skill-lifecycle/skill-removal.md) | `skill-lifecycle` | `done` | Released through `main`. |
| [skill-version-management.md](done/skill-lifecycle/skill-version-management.md) | `skill-lifecycle` | `done` | Released through `main`. |
| [auto-update-notification.md](done/updates/auto-update-notification.md) | `updates` | `done` | Released through `main`. |
| [init-tui-wizard.md](done/configuration/init-tui-wizard.md) | `configuration` | `done` | Released as SKM 0.3.0 through `main` by PR #8 on 2026-09-09 (`69c0b87`); all production packages and checksums published successfully. |
| [skill-search.md](done/registry/skill-search.md) | `registry` | `done` | PR #18 released SKM 0.5.0 through `main` at `488860f`; production CI and all platform packages passed, and the Linux checksum, version, and search help were verified. |
| [search-dedicated-add.md](done/registry/search-dedicated-add.md) | `registry` | `done` | PR #32 released the read-only search boundary in SKM 0.6.0 at `769f5a4`; production CI, publication, checksum, binary identity, and manifest verification passed. |
| [agent-skill-directory-integration.md](done/agents/agent-skill-directory-integration.md) | `agents` | `done` | PR #40 released the tested agent targets through `main` at `a6cdef2`; production CI and the verified complete four-platform publication passed. |
<!-- SPEC-CATALOG:END -->
