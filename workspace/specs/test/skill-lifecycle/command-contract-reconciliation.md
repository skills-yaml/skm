# Reconcile Skill Command Contracts

## Status

State: `test`

Rationale: PR #69 merged the implementation into the configured `development` test channel at `46d70fadc5a3c92bd4013e89e578ce03bb587733` on 2026-09-28 after Validate run `36476786848` passed. Production release remains pending.

## Problem and Users

SKM exposes a separate `bundle add` command after `add` gained bundle support, and `dev mode` reports a setting that no active behavior uses. Skills-only `install` advertises preview, transaction, and lockfile behavior that it does not provide. Default-registry selection, registry scope, and some configuration flags do not match their names. Projects with many exact pins cannot upgrade them as one reviewed change.

Users are project maintainers who install, inspect, and update skills, and scripts that use the command interface.

## Goals

- Retire `skm bundle` and `skm dev mode`; keep published bundles available through `skm add --kind bundle` and direct development linking through `skm dev link`.
- Make skills-only `skm install --dry-run`, `--json`, and `--yes` work consistently, with complete preflight, rollback on failure, and a deterministic project lockfile written last. Preserve toolkit install behavior and its lock format.
- Make newly added skills and explicit bundle additions use the configured default registry when no registry is supplied. Preserve the historical meaning of existing manifest entries whose `source` is omitted.
- Let `skm skill upgrade` upgrade one named skill or all outdated pinned registry skills through a previewed, rollback-protected operation.
- Make `skm cache refresh` honor the effective project registry map, including project overrides.
- Use `--registry` as the documented add option, retaining `--source` as an accepted compatibility spelling. Rename `registry default` to `registry set-default` with the old spelling accepted but hidden.
- Hide redundant `init` and configuration get, set, and unset `--project` flags while accepting them for compatibility; reject simultaneous global and project scope, retire `init-config`, and keep the project default.
- Update CLI help, README, and focused tests to match implemented behavior.

## Non-goals

- Changing the schema-2 published bundle format, toolkit lockfile format, self-update behavior, or release process.
- Tracking an installed bundle as hidden persistent state or automatically upgrading a bundle when its registry manifest changes.
- Changing the meaning of existing `skills.yaml` entries with no explicit `source`.
- Deleting local development links or configuration when the inert `dev mode` subcommand is retired.

## Design and Compatibility

The ordinary install plan resolves every configured skill and dependency, validates every source and target, and describes link and lockfile changes. Dry-run and JSON only present this plan. Application preflights all links, backs up replaced symlinks, writes the project lockfile after successful links, and restores prior links and lockfile on failure. The lockfile records the resolved source identity, exact version or local path, configured agents, and link claims in deterministic order. Existing toolkit installations retain their own lock representation and path. Global installation uses the same link plan but does not write a project lockfile. Successful project add, remove, and named version changes refresh an existing skills-only lockfile; direct manifest edits require another install.

`skill upgrade --all` selects pinned registry skills with newer cached stable versions, with `--pre` available. It skips local and `latest` skills, previews exact pin and link changes, and applies them together or not at all. The named form remains available. Registry cache refresh is explicit; upgrade does not fetch remote changes. A published bundle add continues to reject conflicting existing pins; bulk upgrade handles the separate update workflow.

`registry set-default` changes the global default for new additions and version discovery without rewriting existing pins. New additions write the chosen source explicitly. Existing source-less pins continue to mean the literal registry named `default` for compatibility. `cache refresh` uses project registries over inherited global registries and reports partial failures.

Removing `bundle` and `dev mode` is an intentional command-line break authorized by the user. All other command spellings with existing behavior retain compatibility where specified above. `init-config` is redundant because startup already ensures base configuration; `setup` remains the explicit config-plus-cache operation. `config reset` writes defaults, while `clean reset` removes selected state; both remain documented distinctly.

## Affected Areas

- `src/main.rs`, `src/config_manager.rs`, `src/registry.rs`, `src/version_manager.rs`, `src/dev.rs`, `src/linker.rs`, and a focused install transaction module if needed.
- CLI, install, registry, and version tests in `src/` and `tests/`.
- `README.md` and human-facing command documentation under `workspace/docs/` where affected.
- `workspace/specs/README.md` and agent memory for the accepted command decision and implemented facts.

## Risks and Rollback

Install and bulk upgrade can replace symlinks and write configuration. Preflight must reject real files, unsafe parent paths, missing sources, and changed inputs before writes; rollback must restore prior symlinks and manifest or lock bytes after an injected failure. Do not read or persist secrets in lockfiles. Revert the implementation before release if compatibility or recovery tests fail. Existing personal `skills.yaml` and agent links in this checkout are not test fixtures and must remain untouched.

## Acceptance Criteria

1. Help and parsing omit `bundle`, `dev mode`, and `init-config`; `add --kind bundle` and `dev link` still work. Hidden compatibility flags and aliases behave as specified.
2. Ordinary `install --dry-run` and `--json` are read-only; application preflights all targets, is idempotent, restores state on failure, and writes a deterministic project lockfile last. Toolkit installation remains compatible.
3. `registry set-default` affects unqualified new additions; project registry overrides are honored by `cache refresh`; source-less existing pins remain bound to `default`.
4. `skill outdated` remains read-only and `skill upgrade --all` previews and applies all eligible cached updates atomically, leaving local and `latest` skills unchanged.
5. README and CLI help match each command's actual scope and options, including the two reset commands and the lockfile behavior.
6. Existing unrelated worktree changes are preserved.

## Validation Gates

- Focused tests cover successful plans, no-op repeats, conflict rejection, injected rollback, default selection, project registry overrides, command retirement, and bulk upgrade.
- `task check`, `task test`, and `git diff --check` pass before handoff.
- Manually compare generated help with the README command reference and verify the personal worktree files remain unmodified.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | minor | skm-next | Retired public commands and changed the install contract in a pre-1.0 candidate. |

## Memory Impact

Status: `updated`

Rationale: The accepted command retirement and install/update contract are recorded in `workspace/agents/memory/decisions.md`. Confirmed test-channel integration is recorded in `workspace/agents/memory/facts.md`, with corresponding entries in `workspace/agents/memory/changelog.md`. Production release remains pending.
