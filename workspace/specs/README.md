# Development Specifications

Every non-legacy spec uses
`workspace/specs/<state>/<primary-feature>/<spec>.md`, where state is `backlog`,
`development`, `test`, `blocked`, or `done` and primary feature is lowercase hyphen-case.

The normal flow is `backlog -> development -> test -> done`. Test means
confirmed integration into the configured test branch or environment. Done
means verified main merge with reconciled acceptance, verification, documentation,
catalog, versions, and memory. Publication is tracked separately. Explicit older
migration specs retain their original completion contract; preserve historical
done specs and assess existing test work individually before advancing it. This repository's configured targets are:

- test: `development` (prerelease channel; equivalent to the conventional
  `develop` default)
- production: `main`

A branch name alone is not lifecycle evidence. Record the confirmed
integration or main-merge event in the spec and in this catalog. Blocked specs
record Previous State, Block Kind (impediment or deferred), Block Reason, and
Resume Condition. Resume at their prior stage and renew stale evidence.
Synchronize release member paths with every state change.

## Feature Categories

| Primary feature | Purpose |
| --- | --- |
| `workspace-governance` | Workspace adoption, documentation policy, lifecycle governance, and validation. |
| `workspace-toolkit` | SKM toolkit and bundle installation contracts, lockfile ownership, and compatibility. |
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
| [registry-format-ownership.md](development/registry/registry-format-ownership.md) | `registry` | `development` | Publisher-specific runtime rules/options are removed locally with independent approval and passing aggregate checks/tests. Result records receive renewed review and verification before handoff; integration and main merge are unconfirmed. |
| [installation-contract-validation.md](development/workspace-toolkit/installation-contract-validation.md) | `workspace-toolkit` | `development` | Workspace Docs major-version gates are removed locally; independent review and aggregate checks/tests pass with installation validation retained. Result records receive renewed review and verification before handoff; shared-test integration and main merge remain unconfirmed. |
| [adopt-workspace-docs-7.md](development/workspace-governance/adopt-workspace-docs-7.md) | `workspace-governance` | `development` | User-authorized 7.0.0 migration is implemented locally with independent review and passing aggregate checks/tests; shared-test integration and main merge remain unconfirmed. Result-record edits receive renewed review and verification before handoff. |
| [bare-help-update-notice.md](development/updates/bare-help-update-notice.md) | `updates` | `development` | Fixing bare invocation notice handling locally; test-channel integration and release remain pending. |
| [adopt-workspace-docs-6.md](test/workspace-governance/adopt-workspace-docs-6.md) | `workspace-governance` | `test` | PR #72 merged the 6.0.0 candidate migration into `development` at `c49f1cc` on 2026-09-30 after Validate run `36682939489` passed; production release remains pending. |
| [notify-registry-docs.md](test/updates/notify-registry-docs.md) | `updates` | `test` | PR #67 merged into the configured `development` test channel at `a8ad8d7` on 2026-09-29 after Validate passed; the first production release that sends the dispatch is pending. |
| [command-contract-reconciliation.md](test/skill-lifecycle/command-contract-reconciliation.md) | `skill-lifecycle` | `test` | PR #69 merged into `development` at `46d70fa` on 2026-09-28 after Validate run `36476786848` passed; production release is pending. |
| [legacy-updater-identity-handshake.md](test/updates/legacy-updater-identity-handshake.md) | `updates` | `test` | PR #66 merged into `development` at `ed5b0a0` after Validate run `36270555944` passed; release run `36271054364` published nine assets and an older Linux binary self-updated successfully. Cross-platform qualification remains pending. |
| [separate-workspace-docs.md](test/workspace-toolkit/separate-workspace-docs.md) | `workspace-toolkit` | `test` | PR #60 merged into the configured `development` test channel at `506d47a` on 2026-09-26 after Validate passed; production release is pending. |
| [command-reorganization.md](test/skill-lifecycle/command-reorganization.md) | `skill-lifecycle` | `test` | PR #63 merged into `development` at `2c852c2` on 2026-09-26 after Validate run `36212734539` passed; production promotion awaits the separately specified legacy updater handshake and release qualification. |
| [add-confirmation.md](done/registry/add-confirmation.md) | `registry` | `done` | PR #58 promoted add confirmation to `main`; SKM 0.8.0 at `07b4bba` passed production CI and four-platform qualification, then published through reviewed `release-prod` with verified assets on 2026-09-25. |
| [search-two-column-output.md](done/registry/search-two-column-output.md) | `registry` | `done` | PR #58 promoted two-column search to `main`; SKM 0.8.0 at `07b4bba` passed production CI and four-platform qualification, then published through reviewed `release-prod` with verified assets on 2026-09-25. |
| [unified-skill-bundle-discovery.md](done/registry/unified-skill-bundle-discovery.md) | `registry` | `done` | PR #51 promoted SKM 0.7.0; commit `44a502a` passed production CI, four-platform qualification, and verified nine-asset publication. |
| [remove-workspace-cli.md](done/workspace-toolkit/remove-workspace-cli.md) | `workspace-toolkit` | `done` | PR #51 promoted skill-led Workspace management; commit `44a502a` passed production CI, four-platform qualification, and verified nine-asset publication. |
| [search-skill-details-and-collections.md](done/registry/search-skill-details-and-collections.md) | `registry` | `done` | PR #42 released the search details through `main` at `9c8153a`; production CI, four-platform qualification, nine-asset publication, checksum, and binary smoke passed. |
| [install-discoverable-targets.md](done/agents/install-discoverable-targets.md) | `agents` | `done` | PR #42 released discoverable install targets through `main` at `9c8153a`; production CI, four-platform qualification, and verified publication passed. |
| [search-metadata-and-bundles.md](done/registry/search-metadata-and-bundles.md) | `registry` | `done` | PR #42 released search metadata through `main` at `9c8153a`; production CI, four-platform qualification, and verified publication passed. |
| [help-update-notice.md](done/updates/help-update-notice.md) | `updates` | `done` | PR #40 released the tested help notice through `main` at `a6cdef2`; production CI, four-platform publication, manifest verification, and an interactive Linux help smoke passed. |
| [workspace-docs-6-compatibility.md](done/workspace-toolkit/workspace-docs-6-compatibility.md) | `workspace-toolkit` | `done` | PR #40 released the tested compatibility through `main` at `a6cdef2`; production CI and the verified complete four-platform publication passed. |
| [workspace-skill-bundle-install.md](done/workspace-toolkit/workspace-skill-bundle-install.md) | `workspace-toolkit` | `done` | The published 20-member Registry bundle passed live SKM testing; commit `44a502a` passed production CI, four-platform qualification, and verified nine-asset publication. |
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
