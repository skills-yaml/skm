# Reorganize Cache, Skill, and Self Commands

## Status

State: `test`

Rationale: PR #63 merged this implementation into the configured `development` test channel on 2026-09-26 at `2c852c248b54e6982a92584c956dd2bc036ab8fc` after Validate run `36212734539` passed. Production promotion remains pending the separately specified legacy updater identity handshake and release qualification.

## Problem and Users

Cache refresh, skill version changes, and SKM binary updates currently use overlapping top-level `update` names. Users need command names that identify the resource being changed and distinguish checking from changing state.

## Goals

- Group cache operations under `skm cache`, skill version operations under `skm skill`, and binary update operations under `skm self`.
- Use `refresh` for registry cache fetches and `upgrade` for changing installed versions.
- Provide a read-only `skill outdated` command based on cached registry versions, with optional explicit refresh.
- Keep `skm check` as the full local manifest, source, link, and toolkit validation command.
- Update active scripts, help, documentation, and tests to the new names.

## Non-goals

- Changing manifest or lockfile formats, registry package formats, or update verification.
- Regrouping add, remove, install, search, bundle, dev, config, or registry configuration commands.
- Maintaining aliases for removed command names.

## Design

`skm cache refresh [registry]` refreshes the named registry or all configured registries when omitted, using a fast forward pull for existing caches. A cached checkout with a different origin fails with guidance to clear it. `skm cache status [registry]` shows local cache statistics. `skm cache prune <registry>|--all [--keep N]` removes unprotected older package versions. `skm cache clear <registry>|--all` removes selected registry caches. Destructive operations retain preview and confirmation flags.

`skm skill versions <name>`, `skm skill use <name>@<version>`, and `skm skill upgrade <name>` retain their version-selection behavior and flags. `skm skill outdated [name] [--refresh] [--pre]` compares pinned registry skills with available versions, skips local-path skills and unpinned `latest` references, and reports whether newer versions are available. `--refresh` fetches each relevant registry before comparison; the default reads only local cache. Outdated returns success when the comparison completes, even if updates are available.

`skm self version`, `skm self check [--channel]`, and `skm self upgrade [--channel] [--yes]` expose existing binary identity and verified updater behavior. The startup notice names `skm self upgrade`.

Remove top-level `version`, `versions`, `use`, `update-skill`, `update`, and `cache-update`; remove `registry update` and `clean cache`. Keep `skm check` and the rest of the existing CLI.

## Affected Areas

- `src/main.rs`: Clap hierarchy, routing, and CLI tests.
- `src/version_manager.rs`: read-only outdated comparison and tests.
- `src/updater.rs`: update notice and embedded identity command.
- `tests/updater.rs`, setup and qualification scripts: new self/cache command names.
- `README.md`: command reference and examples.
- `workspace/specs/README.md`: lifecycle catalog.

## Compatibility, Safety, and Rollback

Old names deliberately fail for ordinary CLI use. The older released updater invokes top-level `version` with `SKM_NO_UPDATE_CHECK=1` on a staged candidate and again after replacement on Windows. The separate [legacy updater identity handshake](../../development/updates/legacy-updater-identity-handshake.md) accepts only that internal probe; new CLI help and normal command parsing retain the breaking command names. Production promotion requires qualification of an old bootstrap updating to the new candidate. Cache refresh and self checks may use the network only when explicitly invoked; `skill outdated` is cache-only by default. Cache clear and prune keep the existing dry-run, confirmation, and pin-protection behavior. Rollback is a source revert before release; existing configuration files need no migration.

## Acceptance Criteria

1. New command help and parsing expose the cache, skill, and self operations above; removed names fail for ordinary use without aliases.
2. Cache refresh, status, prune, and clear route to their existing underlying behaviors with valid scope and safety flags.
3. Skill versions, use, and upgrade retain existing functionality; outdated reports newer stable or optionally prerelease pins using cached data and never changes a manifest unless explicit refresh changes the registry cache.
4. Self version, check, and upgrade preserve embedded identity, release-channel selection, and verified update behavior; all active scripts use the new commands.
5. `skm check` still validates configured skill sources and expected links.
6. Documentation and tests match the new command contract, and unrelated worktree changes are preserved.

## Validation Gates

- Focused CLI and version comparison tests, including old-name rejection and outdated success/failure cases.
- `task check`, `task test`, and `git diff --check` pass.

## Memory Impact

Status: `updated`

Rationale: The accepted command grouping and cache refresh behavior are recorded in `workspace/agents/memory/decisions.md`. The prior bridge prerequisite in `workspace/agents/memory/facts.md` is superseded by the separate legacy updater identity handshake. Both have entries in `workspace/agents/memory/changelog.md`.
