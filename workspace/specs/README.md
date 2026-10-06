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
| [decouple-workspace-pin.md](development/workspace-governance/decouple-workspace-pin.md) | `workspace-governance` | `development` | The reproduced manifest-pin coupling is removed locally with fifteen passing Workspace tests; canonical governance declarations and SKM formats are retained. Review and final gates precede delivery; integration remains unconfirmed. |
| [workspace-toolkit-0-7-2.md](test/workspace-governance/workspace-toolkit-0-7-2.md) | `workspace-governance` | `test` | PR #85 merged the independently reviewed toolkit 0.7.2 guidance into development at c145429 on 2026-10-06 after PR CI 37515809249 passed; fresh combined task check/test passed (202 tests). Main completion remains pending. |
| [legacy-release-qualification.md](done/updates/legacy-release-qualification.md) | `updates` | `done` | Verified main 7a289ab, CI 37454486476/37454491771, four-platform current/legacy qualification 37455327437, and protected 0.9.0 publication 37454486538. Acceptance and records reconciled; historical contracts are explicitly superseded where applicable. |
| [registry-format-ownership.md](done/registry/registry-format-ownership.md) | `registry` | `done` | PR #80 merged the confirmed development implementation into main at 79658625c5bb30cd84b0ad5185237e28e20038eb on 2026-10-06; actual main CI run 37451646628 passed task check, all 202 tests, and task build. Acceptance, documentation, catalog, applied SKM 0.9.0, and memory are reconciled. Production publication remains separately subject to qualification and human release-prod approval. |
| [installation-contract-validation.md](done/workspace-toolkit/installation-contract-validation.md) | `workspace-toolkit` | `done` | PR #80 merged the confirmed development implementation into main at 79658625c5bb30cd84b0ad5185237e28e20038eb on 2026-10-06; actual main CI run 37451646628 passed task check, all 202 tests, and task build. Acceptance, documentation, catalog, applied SKM 0.9.0, and memory are reconciled. Production publication remains separately subject to qualification and human release-prod approval. |
| [adopt-workspace-docs-7.md](done/workspace-governance/adopt-workspace-docs-7.md) | `workspace-governance` | `done` | PR #80 merged the confirmed development implementation into main at 79658625c5bb30cd84b0ad5185237e28e20038eb on 2026-10-06; actual main CI run 37451646628 passed task check, all 202 tests, and task build. Acceptance, documentation, catalog, applied SKM 0.9.0, and memory are reconciled. Production publication remains separately subject to qualification and human release-prod approval. |
| [bare-help-update-notice.md](done/updates/bare-help-update-notice.md) | `updates` | `done` | Verified main 7a289ab, CI 37454486476/37454491771, four-platform current/legacy qualification 37455327437, and protected 0.9.0 publication 37454486538. Acceptance and records reconciled; historical contracts are explicitly superseded where applicable. |
| [adopt-workspace-docs-6.md](done/workspace-governance/adopt-workspace-docs-6.md) | `workspace-governance` | `done` | Verified main 7a289ab, CI 37454486476/37454491771, four-platform current/legacy qualification 37455327437, and protected 0.9.0 publication 37454486538. Acceptance and records reconciled; historical contracts are explicitly superseded where applicable. |
| [notify-registry-docs.md](test/updates/notify-registry-docs.md) | `updates` | `test` | Production SKM 0.9.0 is published, but notification was skipped because REGISTRY_APP_CLIENT_ID is unset. Development skips correctly; the first authenticated dispatch and Registry receipt/docs-review issue remain unverified. |
| [command-contract-reconciliation.md](done/skill-lifecycle/command-contract-reconciliation.md) | `skill-lifecycle` | `done` | Verified main 7a289ab, CI 37454486476/37454491771, four-platform current/legacy qualification 37455327437, and protected 0.9.0 publication 37454486538. Acceptance and records reconciled; historical contracts are explicitly superseded where applicable. |
| [legacy-updater-identity-handshake.md](done/updates/legacy-updater-identity-handshake.md) | `updates` | `done` | Verified main 7a289ab, CI 37454486476/37454491771, four-platform current/legacy qualification 37455327437, and protected 0.9.0 publication 37454486538. Acceptance and records reconciled; historical contracts are explicitly superseded where applicable. |
| [separate-workspace-docs.md](done/workspace-toolkit/separate-workspace-docs.md) | `workspace-toolkit` | `done` | Verified main 7a289ab, CI 37454486476/37454491771, four-platform current/legacy qualification 37455327437, and protected 0.9.0 publication 37454486538. Acceptance and records reconciled; historical contracts are explicitly superseded where applicable. |
| [command-reorganization.md](done/skill-lifecycle/command-reorganization.md) | `skill-lifecycle` | `done` | Verified main 7a289ab, CI 37454486476/37454491771, four-platform current/legacy qualification 37455327437, and protected 0.9.0 publication 37454486538. Acceptance and records reconciled; historical contracts are explicitly superseded where applicable. |
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
| [Workspace Docs 7 Compatibility](done/workspace-toolkit/workspace-docs-7-compatibility.md) | `workspace-toolkit` | `done` | PR #76 integrated into development at 9068581; PR #75 released focused production 0.8.1 through main at 49468aa. Both channels passed native gates, checksum/source identity, fresh twenty-member v7 bundle installation, skm check, and no-op repetition. |
<!-- SPEC-CATALOG:END -->
