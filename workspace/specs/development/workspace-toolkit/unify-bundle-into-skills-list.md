# Unify Bundle Handling into the Skills List

## Status

State: development

Primary feature: `workspace-toolkit`

Status rationale: Follow-up to bundle-skill-entries.md (same branch, same
release). The `kind: bundle` manifest contract is implemented; this spec removes
the dedicated bundle code path (`src/bundle.rs`, separate add transaction,
bundle-specific CLI flags) and routes everything through the unified `skills:`
list resolved by `linker::resolve_entries`. Implementation is active in
isolated peer worktrees; integration and acceptance verification remain pending.
Independently reviewed by agent neo on 2026-11-10; findings
incorporated.

## Problem and Evidence

At the current branch state, bundles have two parallel code paths:

1. **Manifest path** (new): `kind: bundle` entries in `skills.yaml`, resolved by
   `linker::resolve_entries` during `install`/`check`/`list`.
2. **Dedicated path** (legacy): `src/bundle.rs` (1301 lines) with its own
   `prepare`/`apply` transaction, invoked from `main.rs::add_registry_item` when
   `--kind bundle` or auto-detected. It writes the `kind: bundle` entry AND links
   members in one bundle-specific transaction with its own rollback journal
   (`src/bundle.rs:646-754`).

This duplication means:

- Two transaction implementations (installer.rs vs bundle.rs) that must stay in
  sync for rollback safety.
- Bundle add has special flags (`--dry-run`, `--json` scoped to bundles) that
  duplicate what `install --dry-run`/`--json` already do.
- `skm add --kind bundle` links immediately, while `kind: bundle` entries added
  manually to `skills.yaml` only link on `skm install` — inconsistent UX.
- `src/bundle.rs` is 1301 lines, mostly transaction/registry-cache logic that
  duplicates `installer.rs` and `search.rs`.

## Goals, Consumers, and Scope

- **One add path**: `skm add <id>` auto-detects skill vs bundle in the selected
  registry. For either kind, it writes the entry (`{name, kind, version, source}`)
  to `skills.yaml` — no linking, no separate transaction. The user then runs
  `skm install` to link, exactly like any other entry.
- **Remove `src/bundle.rs`** entirely. Namespace-manifest reading
  (`resolve_bundle_members_with`) already lives in `linker.rs`; delete the
  bundle-specific prepare/apply/transaction/registry-root code.
- **Simplify `add` CLI**: `--dry-run` and `--json` become generic manifest
  previews (not bundle-specific). `skm add <id>` writes the entry, shows the
  plan, asks confirmation (or `--yes`). `--dry-run` previews the manifest change
  without writing. `--json` emits the planned entry as JSON.
- **Keep `--kind skill|bundle`** as an explicit disambiguator when a registry
  publishes both with the same ID (the ambiguity error stays).
- **Keep the `--global` bundle rejection**: `skm add --global <bundle>` remains
  rejected ("published bundles can only be added to a project").
- **Search output** changes the suggested add command from
  `skm add <id> --source <r> --kind bundle` to `skm add <id> --source <r>`
  (auto-detected). The JSON `apply_command` changes from
  `skm add <id> --source <r> --kind bundle --yes` to
  `skm add <id> --source <r> --yes && skm install --yes` since add no longer links.
  JSON `kind: "bundle"` field stays.
- **`skm remove <bundle-name>`** removes the `kind: bundle` entry and unlinks all
  resolved members, but does NOT unlink members that are also declared as
  standalone skill entries in the remaining config. The existing `remover.rs`
  flow needs bundle support: resolve entries via `resolve_entries`, unlink every
  member, then sync the lockfile.
- **`skm install`** resolves bundle entries via `resolve_entries`, links all
  members, writes the lockfile with `bundles:` + `skills:` sections in one
  transaction (already implemented by bundle-skill-entries.md).
- **`skm skill outdated`/`upgrade`**: bundle entries are handled by re-resolving
  from cache via `resolve_entries`. `skm check`/`verify_lock` is the primary
  drift detector (membership + version); `outdated`/`upgrade` report per-member
  version changes for bundle entries and can detect new members after
  `skm cache refresh`. `skm skill use <bundle>@<version>` changes the bundle
  entry's version pin.
- **`skm clean symlinks`** expands bundle entries via `resolve_entries` when
  collecting known skill names so bundle member symlinks are not classified as
  orphaned.

Consumers are CLI users and scripts. **Breaking change**: `skm add` (skill or
bundle) no longer links immediately — every `skm add` must be followed by
`skm install`. Scripts that chain `add --yes` and expect links must add
`skm install --yes` after. After upgrading, run `skm install` once before
`skm check` (the lockfile may be stale between add and install).

Non-goals: change the registry format; change `skm search` result content
(bundles still listed); change toolkit bundles (`bundles:` key in skills.yaml
remains toolkit-only); change the lockfile schema; remove `--kind` flag entirely
(it stays for collision disambiguation); support global bundle entries.

## Command Behavior After

| Command | Before (dedicated path) | After (unified) |
| --- | --- | --- |
| `skm add acme/starter --source local` | Auto-detects bundle, runs bundle.rs transaction, writes entry + links | Writes `{kind: bundle}` entry to skills.yaml, shows plan, prompts/`--yes`. No linking. |
| `skm add acme/alpha --source local` | Writes skill entry + links immediately | Writes skill entry, shows plan, prompts/`--yes`. No linking. |
| `skm add acme/starter --kind bundle --yes` | Bundle transaction with rollback | Same as auto-detect: writes entry, no linking. |
| `skm add acme/starter --dry-run` | Bundle preview (no writes) | Manifest preview (no writes). Same UX, now also works for skills. |
| `skm install` | Links skills; bundle links done separately by add | Links all entries via resolve_entries (skills + bundle members). One transaction. |
| `skm remove acme/starter` | Removes unpacked members (old) / errors on kind:bundle entry (current branch) | Removes the `kind: bundle` entry + unlinks resolved members (except shared standalone entries). Updates lockfile. |
| `skm check` | Verifies bundle entries via resolve_entries + verify_lock (already works) | Same. Detects membership drift and version drift. |
| `skm clean symlinks --orphaned` | Would delete bundle member symlinks as "orphaned" | Expands bundles via resolve_entries; bundle members are known, not orphaned. |
| `skm skill outdated` | Hard error on pinned bundle; silently skips latest bundle | Re-resolves bundle entries; reports member version changes and new members. |

## Migration and Compatibility

- Existing `skills.yaml` files with unpacked bundle members (pre-0.10 behavior)
  are fully supported — they are ordinary skill entries. No migration needed.
- Existing `skills.yaml` files with `kind: bundle` entries work via
  `resolve_entries`.
- **Breaking**: `skm add` no longer links. All scripts doing `skm add ... --yes`
  must append `skm install --yes`.
- After upgrading the binary, run `skm install` once before `skm check` —
  the lockfile may be stale relative to the manifest.
- `skm add --kind bundle` still parses but no longer triggers a separate
  transaction.
- Idempotent re-add: `skm add <bundle> --yes` when a `kind: bundle` entry with
  the same name/source already exists is a no-op (reports "already present"),
  preserving the previous spec's idempotency contract.
- The lockfile retains its existing schema and `bundles:` section; the linked
  bundle-skill-entries spec adds optional resolved release metadata. After `skm remove <bundle>`, the lockfile's
  `bundles:` section no longer contains that bundle.

## Implementation Plan

1. **linker.rs**: no changes needed (resolve_entries already expands bundles).
   Ensure `resolve_bundle_members_with` handles all registry resolution paths.
2. **main.rs add path**: unify `add_registry_item` — remove the bundle branch
   that calls `bundle::add`. For both skill and bundle, write the entry via
   `add_skill_with_confirmation` (extended to accept `kind`). Make `--dry-run`
   and `--json` generic preview flags. Keep `--kind` for disambiguation. Keep
   `--global` bundle rejection.
3. **main.rs add_skill_with_confirmation**: accept `kind: Option<String>`;
   idempotent no-op when the same entry (name + kind + source) already exists.
   Remove the immediate link loop — add only writes the manifest entry.
4. **remover.rs**: resolve entries via `resolve_entries`; for a bundle entry,
   unlink all resolved members except those still declared as standalone
   entries; remove the entry; sync lockfile.
5. **cleaner.rs**: expand bundle entries via `resolve_entries` in
   `collect_known_skill_names` so member symlinks are not orphaned.
6. **version_manager.rs**: handle `kind: bundle` entries in
   `show_outdated`/`upgrade_all`/`use_version` by re-resolving via
   `resolve_entries` and comparing member versions.
7. **search.rs**: change bundle add suggestion to `skm add <id> --source <r>`;
   change JSON `apply_command` to include `skm install --yes`.
8. **Delete src/bundle.rs** and remove `mod bundle;` from main.rs.
9. **installer.rs**: verify it already handles bundles correctly (it does via
   resolve_entries); no changes expected.
10. **Tests**: delete bundle.rs tests; add equivalent tests at add/install/
    remove/check/clean level for bundle entries.
11. **README.md**: update add documentation.

## Affected Areas

- `src/bundle.rs`: **delete entirely**.
- `src/main.rs`: unify `add_registry_item`; remove bundle branch and
  `mod bundle;`; make `--dry-run`/`--json` generic on `add`.
- `src/search.rs`: change bundle add suggestion and JSON apply_command.
- `src/remover.rs`: bundle-aware removal (resolve entries, unlink members,
  skip shared standalone entries, sync lockfile).
- `src/cleaner.rs`: expand bundle entries when collecting known skill names.
- `src/version_manager.rs`: bundle handling in outdated/upgrade/use.
- `src/installer.rs`: already handles bundles; verify only.
- `src/linker.rs`: already has resolve_entries; may need minor adjustments.
- Tests: delete bundle.rs tests, add unified-path tests.
- README.md: update add documentation.

## Validation Gates

- `task check` passes (fmt, clippy -D warnings, cargo check, workspace/versions/
  coordination gates).
- `task test` passes — all existing tests adapted plus new tests for the
  unified path.
- Deterministic fixture tests for: bundle add writes entry (no links), install
  links members, remove unlinks members, clean does not orphan members,
  outdated detects bundle drift, idempotent re-add.
- Manual smoke: `skm add <bundle> --yes` then `skm install --yes` then
  `skm check` in a temp project with a local registry.

## Risks

- **Breaking change for scripts**: `skm add` no longer links. Mitigation:
  documented in Migration; README updated; the intermediate state (entry in
  manifest, not linked) is expected — `skm check` failing until `skm install`
  is by design.
- **Add→install intermediate state**: after `skm add`, the manifest has an entry
  with no links and a stale lockfile. `skm check` fails with "stale lockfile"
  until `skm install` runs. This is intentional; installer's transaction
  provides rollback safety during install.
- **Shared members**: a bundle member that is also a standalone skill entry must
  not be unlinked when the bundle is removed. Remover must check the remaining
  config.
- **Old bundle.rs transaction vs new model**: old path had its own rollback
  journal for the combined manifest-write + link operation. New model splits
  this: add writes only the manifest (single-file write with changed-file
  detection), install has its own transaction. Net atomicity is equal or better.
- **Lockfile sync ordering**: after remove, `sync_existing_lock` errors if
  links are incomplete; remover must unlink before syncing.

## Acceptance Criteria

1. `skm add <bundle-id> --yes` writes a single `kind: bundle` entry to
   skills.yaml and does NOT create symlinks. `skm install --yes` then links all
   members.
2. `skm add <skill-id> --yes` writes a skill entry and does NOT link. `skm
   install` links it.
3. `src/bundle.rs` is deleted; `cargo check` passes with no dead code.
4. `skm remove <bundle-id>` removes the entry and all member symlinks, updates
   the lockfile `bundles:` section, and does NOT unlink members still declared
   as standalone skills.
5. `skm check` passes after install with bundle entries; detects membership
   drift (new bundle member) and version drift (member version change).
6. `skm add --kind bundle` and `--kind skill` still work for disambiguation.
7. Search results suggest `skm add <id> --source <r>` (no `--kind bundle`);
   JSON `apply_command` includes `skm install --yes`.
8. `skm add <bundle> --yes` twice is a no-op (idempotent).
9. `skm add <skill> --dry-run` and `--json` preview the manifest change without
   writing.
10. `skm add --global <bundle>` is rejected.
11. `skm clean symlinks --all --dry-run` does not report bundle members as
    orphaned.
12. `skm skill outdated` on a bundle entry reports member version changes and
    new members after `skm cache refresh`.
13. `task check` and `task test` pass.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | minor | skm-global-user-installation | Behavioral change: add no longer links, bundle path unified, bundle.rs removed. Per this repo's 0.y.z convention (versioning.md), minor covers incompatible pre-1.0 changes. Shares the applied 0.10.0 reservation (Cargo.toml already 0.10.0). |

## Memory Impact

Status: pending

Rationale: Durable decision to record when done: bundle handling is fully
unified under the skills list; src/bundle.rs is removed; add writes entries
only, install links everything.
