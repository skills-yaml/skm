# Global User Configuration and Skill Installation

## Status

State: development

Primary feature: `configuration`

Status rationale: S1 path and bootstrap implementation is in peer review, reusing the applied shared 0.10.0 reservation. Cache-consumer guards, command scope, combined review, integration, and acceptance remain pending.

## Problem and Evidence

SKM supports global agent link destinations, but its ordinary skill commands
still use `skills.yaml` in the current directory, including with `--global`.
Users therefore need a dedicated directory and must remember to run global
maintenance there. Global settings and caches also use separate OS directories.

At repository baseline `68568ea`, `src/config_manager.rs` uses
`dirs::config_dir()/skm/config.yaml` and `dirs::cache_dir()/skm`;
`src/linker.rs` independently resolves registries under `~/.cache/skm/registries`,
which can differ from the OS cache location when XDG or platform defaults vary.
`src/main.rs` selects the current directory's manifest before dispatch;
`src/config_editor.rs` treats `config --global` as base settings;
`src/installer.rs` already supports global links without a project lockfile.
Startup currently prints initialization messages and exempts install previews
from machine-state initialization.

This is a follow-up to the completed
[global environment specification](../../done/configuration/global-env-auto-config.md)
and [configuration management specification](../../done/configuration/config-management.md).
Preserve those historical records. Existing compiled agent destinations and
shared-target deduplication remain defined by the
[agent integration specification](../../done/agents/agent-skill-directory-integration.md)
and [discoverable target specification](../../done/agents/install-discoverable-targets.md).

## Goals, Consumers, and Scope

- Give each OS user one discoverable SKM home, `~/.skm`, containing the shared
  cache, base settings, and a persistent global skill manifest.
- Silently bootstrap missing user state during ordinary command startup.
- Make global skill installation and maintenance independent of the working
  directory and install each declared skill for every selected global agent.
- Preserve project manifests, project installation, and existing linked sources
  while introducing the new paths.

Consumers are CLI users maintaining personal skills across repositories,
scripts using `--global`, and every supported agent selected in the global
manifest. Global means the current OS user, including root when run as root;
it does not mean all users on a machine.

Non-goals: change the default installation scope; merge project and global skill
lists; introduce a global lockfile, agent discovery rules, or new agent paths;
enable global published bundles or repository-local toolkits; change agent
precedence; redesign development-link registrations; add scheduled updates;
expand the scope to unrelated behavior. Governed instructions,
release workflows, and runtime dependencies need no changes for this proposal.

## User Home and Startup Contract

Resolve the current user's home through the existing platform-aware home
resolver, never from the working directory. On Windows this means the resolved
user profile home. Configuration/cache environment-directory overrides remain
relevant only for locating legacy data during migration.

```text
~/.skm/
  config.yaml       # Base settings: registries, default registry, update policy
  skills.yaml       # User-selected global agents and declared global skills
  cache/            # Shared cache used by project and global installations
    registries/
```

Base settings and the skill manifest retain separate ownership and their
existing YAML schemas. `config.yaml` retains `BaseConfig` defaults, subject to
legacy import below. Create a missing `skills.yaml` as a valid empty
`SkillsConfig`, without selecting agents or installing example skills:

```yaml
name: Global skills
agents: []
skills: []
```

For ordinary, successfully parsed command invocations, startup ensures these
directories and missing files exist. Successful bootstrap emits no stdout,
stderr, prompts, registry cloning, refreshes, or agent-link writes. Repeated or
concurrent startup must not truncate, replace, or reset existing user data.
Use create-if-absent publication with complete valid YAML, including races
where another process creates the same file first.

Preserve existing read-only preview guarantees: `--dry-run` and plan-only
`--json` operations do not bootstrap, migrate, fetch, or modify user state.
They may use empty defaults in memory when new user files are absent. Help,
version/identity probes, internal updater workers, and invalid CLI input do
not require bootstrap. Existing update notices remain a separate policy;
silence here concerns initialization messages, not unrelated command output.

Do not overwrite malformed YAML, a manifest symlink, or an unexpected file
type. An unusable home, unsafe path, invalid required document, or permission
failure must produce an actionable stderr error for commands needing that
state, rather than success with the wrong scope. Help and identity output
remain available. Existing valid manifests retain unknown publisher metadata.

## Scope and Command Contract

| Operation | Without `--global` | With `--global` |
| --- | --- | --- |
| `init` | Create/edit the current project manifest | Create/edit `~/.skm/skills.yaml`; select global agents and skills through the existing prompt flow |
| `install` | Read the project manifest; write project links and the existing lockfile | Read only the user manifest; install to all its selected global agent targets |
| `add` | Update the project manifest | Update the user manifest; install applies links |
| `remove` | Update the project manifest and project links | Update the user manifest and global links |
| `list` / `check` | Inspect project declarations and expected targets | Inspect user declarations and expected global targets |
| Manifest-backed `skill use`, `outdated`, and `upgrade` | Read/update project pins as today | Read/update user pins, independent of the current directory |
| `config get/set/unset/show/reset/validate` | Keep the existing project selection | Keep base-settings semantics, now at `~/.skm/config.yaml` |

`config --all` / `--paths` must continue distinguishing project configuration
from base settings. They must not imply that base settings contain installed
skills. The global skill manifest is edited through `init --global`, ordinary
global skill commands, or a text editor. Base registry-management commands
continue owning base settings; global installation resolves registry overrides
from the user manifest, then inherited base registries, never the current
project's registry map. Preserve historical source-less pin behavior.

A shared registry cache alias must also match the selected scope's effective
registry URL before its contents are trusted. An existing cache directory is
not sufficient evidence. Preserve the current effective Git fetch-URL comparison,
including URL rewrites and exact-byte semantics, from the completed
[cache refresh correction](../../done/registry/cache-refresh-url-rewrites.md).
Reject a same-alias cache from a different source before manifest/link writes
or refresh; do not silently consume it, repoint its origin, or delete it. Explain
the conflict and allow the user to choose a distinct registry alias. Apply the
same guard to canonical and legacy fallback entries and to registry-backed
global inspection/version operations.

Example workflow, valid from any directory:

```sh
skm init --global
skm add example/skill --global --yes
skm install --global --dry-run
skm install --global --yes
skm list --global
skm check --global
```

The example skill ID is illustrative. No new registry publication is required.
`init --global --non-interactive` must handle the empty bootstrap manifest
without a false already-exists failure, while retaining protections for an
existing populated manifest. Cancellation preserves existing contents.

The user manifest is the authoritative declarative record of global skill
selections: name, requested version, source or local path, and selected agents.
`add --global` persists the selected skill there without installing links;
`remove --global` removes its declaration only after successful unlinking.
`install --global` reconciles that declared set; it does not import the current
project's skills, discover arbitrary pre-existing links as owned skills, or
silently rewrite requested versions. Dependencies retain existing closure and
version rules; they need not become duplicate top-level declarations.

## Installation and Model Invariants

This is a filesystem ownership and representation change, using the current
Rust/Serde YAML models. It introduces no database, event API, or new schema
dialect. Distinguish the global manifest directory from the home used as the
root of global agent destinations.

| ID | Rule and concrete failure example | Owner / representation | Enforcement and consumers |
| --- | --- | --- | --- |
| I1 | One authoritative manifest per scope. Running global add inside another repository must not edit that repository. | CLI scope resolution; `SkillsConfig` at the selected path | Shared path selection used by main, wizard, remover, version manager, and global cleanup |
| I2 | Bootstrap only fills missing user state. A simultaneous startup must not replace customized registries or skills. | Config manager; base settings and user manifest | Safe file creation, validation, concurrent-startup tests |
| I3 | Every selected agent receives every resolved skill at its compiled global destination. Selecting Codex and Goose must not duplicate their shared `.agents/skills` write. | Linker; agent IDs and deduplicated target groups | Existing target resolver and installer; install/list/check tests |
| I4 | A successful global removal keeps declarations and managed links consistent. Install failure on the second target must not leave a partially applied installation. | Installer / mutation transaction; selected manifest plus managed links | Complete preflight, conflict detection, rollback, failure injection |
| I5 | Scope never depends on working directory. A relative local path in the global manifest resolves against `~/.skm`, not the invoking project. | Manifest/source resolver; existing `SkillSpec.path` | Normalize new `add --path` selections against their invocation directory before storing a stable global path; preserve project path rules |
| I6 | Cache migration does not destroy live sources or trust another registry. Existing symlinks must not become dangling, and a project's `default` cache must not supply a different global `default` registry. | Config manager / cache resolver; canonical cache and retained legacy sources | Both legacy-root discovery paths, effective-origin validation, non-destructive migration, refresh/install/check tests |

Validate skill names, registry names, agent IDs, final target-name collisions,
dependencies, and sources before applying links. Require at least one effective
target for a nonempty resolved skill set; empty selections must not claim
installation success. Agent detection may suggest choices during init, but
installation must honor the stored selection, including creating missing target
directories. Do not select all supported or detected agents automatically.

Reuse the existing compiled paths and direct-child link layout. Replace existing
symlinks only under current safety rules; never overwrite a real file/directory
or traverse an unsafe destination parent. Apply a prepared plan only while its
manifest and destinations still match the plan. Repeating a successful install
must be idempotent. Global checks verify each link's expected source, including
wrong-but-existing targets; existence alone is insufficient. Global install
must not write a project manifest, lockfile, or agent directory.

## Compatibility, Migration, and Recovery

- Import a valid legacy base settings document into `~/.skm/config.yaml` only
  when the new file is absent. Preserve values and opaque fields; leave the
  legacy file intact. An existing new file wins without merging or dual writes.
  Invalid legacy settings must be reported without replacing them with defaults.
- New cache writes use `~/.skm/cache`. Locate legacy registry sources under both
  `~/.cache/skm/registries` (the existing linker root) and
  `dirs::cache_dir()/skm/registries` when distinct. Non-registry caches also have
  consumers of the OS cache root. Startup must not relocate or delete either
  root: their absolute paths may be embedded in existing links. When a canonical
  registry entry is absent, inspect the historical linker root first, then the
  distinct OS cache root, and validate the selected effective origin before
  reading an entry. A conflicting existing entry must be reported, not silently
  bypassed. Explicit refresh or install may populate the new cache and relink
  the selected scope; retain old backing sources for installations that have
  not been reconciled. Test distinct legacy roots using an isolated XDG override
  and the platform's native layout where applicable.
- A check after cache transition must still enforce expected-source validation.
  If a legacy link no longer matches the selected source, report it accurately
  and direct the user to rerun install in that scope. Never report a wrong target
  healthy merely to hide migration differences.
- Old directory-specific global manifests are not auto-discovered or imported
  from the current repository. Document how to review/copy a dedicated old
  global manifest into `~/.skm/skills.yaml` and re-run global installation.
  This intentionally changes existing `--global` scripts; provide explicit
  migration examples and explain the new location.
- Global orphan detection uses the user manifest plus existing development-link
  registrations; it must not treat global skills as orphaned because a different
  project is open. Cache pruning/clearing and reset previews must distinguish
  cache content from authoritative manifests. Do not add legacy-cache deletion
  to startup or routine migration. Keep development-link registration format
  and storage behavior unchanged.
- Downgrading the binary does not restore old command semantics or copy new
  selections back. Retained settings/caches preserve recovery options; document
  explicit export/copy and reinstall steps before using the old CLI. No automatic
  cross-version synchronization or destructive rollback is promised.

## Affected Areas

| Area | Intended responsibility |
| --- | --- |
| `src/config_manager.rs`, `src/config.rs` | User-home paths, bootstrap, base import, empty global manifest, existing schema preservation |
| `src/main.rs`, `src/wizard/`, `src/config_editor.rs` | Scope routing, global init, help/output, distinct base-settings contract |
| `src/installer.rs`, `src/linker.rs`, `src/remover.rs`, `src/version_manager.rs` | Separate manifest/source roots from agent root, global state consistency, expected-source checks |
| `src/registry.rs`, `src/cleaner.rs`, `src/updater.rs`, relevant `src/dev.rs` consumers | Shared-cache path consumers, registry inheritance, safe cleanup and preserved dev registrations |
| Existing unit tests and `tests/` command suites; `Taskfile.yml` if needed | Temporary-home, scope, migration, multi-agent-target, and failure regression coverage; native focused selectors |
| `README.md`, `workspace/docs/installation-scopes.md`, applicable architecture docs | New user layout, workflow and migration guidance; reconcile rather than overwrite local documentation work |
| Spec catalog, release ledger, agent memory | Lifecycle, one planned release, durable approved decisions |

## Acceptance Criteria

1. **AC1 — Silent bootstrap:** A first ordinary invocation in a temporary home
   creates the specified layout, valid base settings, and an empty user manifest
   without initialization output or network work. Repetition and concurrent
   startup preserve existing bytes; unsafe/invalid/unwritable paths fail safely.
2. **AC2 — Scope isolation:** From two distinct working directories, including
   one with a conflicting project manifest, global operations use the same user
   manifest and global registry configuration. Plain commands retain project
   behavior and lockfiles; global commands leave project files byte-identical.
   A same-name registry cache from a different effective URL is rejected before
   skill consumption or writes; legitimate Git URL rewrites still work.
3. **AC3 — Persisted selection:** Global init saves selected agents/skills;
   successful global add/remove and version changes update the user manifest.
   Cancelled, conflicting, or failed operations preserve the previous manifest
   and affected links. Relative global local-skill paths are working-directory
   independent, and unknown metadata survives edits.
4. **AC4 — All selected agents:** A temporary-home fixture covers every supported
   global agent, shared target groups, missing agent directories, invalid IDs,
   no selected targets, dependency closure, and target-name collisions. Install
   creates the expected links once per destination and no unselected targets.
   Repetition is a no-op; injected partial failure rolls back changed links.
5. **AC5 — Global inspection and safety:** Global list/check/version/cleanup
   work outside projects. Check rejects a link to the wrong existing source;
   preflight preserves real target files/directories. Orphan cleanup preserves
   declared global and development skills regardless of the current project.
6. **AC6 — Non-destructive migration:** Legacy settings are imported without
   deletion or loss, new settings win, old backing cache sources remain usable
   across distinct linker/OS cache roots, and canonical-cache refresh/reinstall
   works. Invalid legacy files, conflicting cache origins, mixed old and new
   state, interrupted bootstrap, and rollback limits have test or documented
   demonstration evidence.
7. **AC7 — Preview and communication:** Dry-run and plan JSON leave the complete
   temporary home/project unchanged and do not fetch. JSON remains parseable.
   Help/identity behavior is preserved. Documentation distinguishes global base
   settings from global skills, explains selected-agent installation, and shows
   migration from the former working-directory-based `--global` flow.

## Ordered Delivery Steps

These steps share one release and lifecycle. Complete each prerequisite's
bounded acceptance evidence before beginning its dependent step. AC references
below select only the stated behavior subset; a step does not require behavior
assigned to a later step. Full criterion closure occurs in S5, and isolated
passes do not establish complete feature acceptance.

| Step | Bounded outcome and exclusions | Prerequisites | Acceptance and smallest meaningful checks |
| --- | --- | --- | --- |
| S1 | Establish centralized paths, safe silent bootstrap, non-destructive settings import and origin-validated retained-cache reads. Exclude CLI scope changes and relinking. | Refresh release/remote occupancy; move to development with Memory Impact pending; apply the reserved bump once. | AC1 bootstrap and AC6 import/cache-discovery portions only; config/path/legacy fixtures first, then `task test:cache` and `task test:updater` for changed path consumers. Add a focused Taskfile selector if existing modules cannot isolate the new fixture tests. |
| S2 | Route user-manifest commands and global init through one scope contract; separate manifest-relative sources from home-relative targets. Exclude installation/mutation transaction changes. | S1 paths, cache-origin guards and migration-discovery tests pass. | AC2 path/registry isolation, AC3 init/local-path/metadata, and AC7 preview-routing portions only; path/wizard/config/CLI fixtures first, then `task test:registry-format`, followed by bundle/toolkit consumer modules because manifest/root selection is shared. |
| S3 | Complete the global install transaction and all-selected-agent application. Exclude add/remove/version transaction changes and cleanup ownership. | S2 scope and representation tests pass. | AC4, AC5 install/check safety, AC6 refresh/reinstall, and AC7 install-preview portions; installer/linker failure fixtures first, then global install/check command tests and affected cache, bundle/toolkit consumers. |
| S4 | Make add/remove and version mutations consistent with the selected global manifest, and make maintenance preserve its ownership. Exclude new installation modes and agent targets. | S3 shared installer transaction and global target checks pass. | AC3 mutation/cancellation/rollback, AC5 list/version/cleanup, and AC2 cross-directory mutation portions; remover/version/cleaner fixtures first, then global command integration and affected cache/dev-registration consumers. Add focused Taskfile selectors where needed. |
| S5 | Reconcile user docs, complete acceptance evidence, independent review, final gates and lifecycle delivery. No additional runtime features. | S1–S4 bounded gates pass. | Close AC1–AC7 with candidate-bound evidence; inspect docs/examples, `task workspace:check`, then `task versions:check`; resolve independent review before complete final `task check` and `task test`. Verify actual shared-test and main revisions through native CI. |

### Current S1 implementation candidate

The bootstrap and settings-import peer is integrated in the shared peer
revision. The cache-consumer candidate selects a validated canonical entry,
then historical linker cache, then distinct OS cache, and writes new registry
caches under the canonical user directory. Git fetch origins retain the existing
byte-exact rewrite comparison. Cache cleanup avoids pruning symlinked local
registry backing sources and leaves legacy cache roots intact. Read-only settings
loading can inspect legacy settings without bootstrapping during previews.

Bounded AC2/AC5/AC6 evidence includes temporary-home Git fixtures covering
ordered legacy roots, alias conflicts before manifest/link changes, canonical
refresh, wrong-source checks and reinstall, and preservation of both old cache
sources. Existing rewrite, relative local Git, plain local directory, bundle,
registry, updater, and CLI tests cover shared consumers. This candidate requires
independent exact-revision review and integration validation. These slices do
not close full AC2, AC5, or AC6: S2 must select the user manifest and effective
registry URL independently of the current project; S3 and S4 must complete
global install, mutation and maintenance ownership. Preview and release-wide
acceptance remain open until S5.

### Current S2 scope-routing candidate

The bounded candidate selects the project or user manifest and uses its parent
for local sources and effective registry overrides, while agent targets remain
home-relative. Temporary-home fixtures exercise two project directories,
conflicting registry overrides, base inheritance, bootstrap init, selected
agents, cancellation, metadata, local-path persistence and non-writing plans.
These are partial AC2, AC3 and AC7 evidence, pending exact-candidate review.
`skill outdated --global --refresh` resolves registry URLs relative to the user
manifest and preserves origin validation. S3 and S4 retain installation
transaction, all-target, mutation, and cleanup work;
these checks do not establish full acceptance of those areas.

### Current S3 install candidate

The installer snapshots the selected manifest and checks planned sources and
destinations, including already-correct links, before and during application.
It restores changed links on failure and refuses unsafe rollback parents or
unexpected replacement paths. Global checks also reject unsafe target parents.
Temporary-home command fixtures cover every supported agent, shared targets,
dependency closure, conflicting names, empty/invalid selections, real targets,
wrong-source links, preview, reinstall, and project isolation. Failure-injection
fixtures cover rollback and stale manifests, sources and destinations. Existing
cache-transition tests exercise retained legacy sources and reinstall.
Symlink replacement and rollback use platform-aware removal and preserve the
original link type and target, with portable directory and file-link regressions. Native Windows execution remains unverified and
requires evidence before complete cross-platform acceptance.
These are bounded AC4–AC7 evidence pending independent exact-candidate review;
S4 mutation and cleanup and S5 combined acceptance remain open.

## Validation Gates and Review

Use only Taskfile entrypoints and the repository's
[validation module contracts](../../../validation/README.md). Tests use temporary
homes and projects, local registry fixtures, and serialized environment changes;
never touch the real user installation. New logic requires regression coverage.

During implementation, run changed-component fixtures first, then the direct
and transitive consumer modules identified per step in dependency order.
Deduplicate overlapping selectors; serialize Cargo tasks sharing target storage.
If a needed focused selector is absent, add a minimal Taskfile entrypoint or
document aggregate fallback. Native command integration, migration, concurrency,
and rollback tests are required even when schema parsing passes. Current tests
do not prove the proposed contracts; coverage gaps remain open until implemented.

Independent exact-candidate review is required for CLI interfaces and filesystem
data integrity. Resolve every finding, freeze source/docs/records, then run full
`task check` and `task test`, including `task versions:check`. Required CI also
runs `task build`. Renew affected review and checks after relevant changes.
Record the actual `development` integration before moving to test and the
verified `main` merge before done; reconcile acceptance, docs, ledger and memory.
Publication remains separately governed by existing release protections.

Implementation and its catalog, release, and memory records require independent review; runtime criteria above remain open until supported by candidate-bound evidence.

## Risks and Constraints

The new global manifest changes a public CLI contract and existing scripts.
Missing agent choices could otherwise produce false-success installations;
wrong scope resolution could overwrite project selections. Cache relocation
could break links in repositories unknown to the current invocation. Concurrent
startup/mutation, unsafe symlinks, and cleanup ownership require explicit failure
tests. Keep these boundaries in the shared path/transaction abstractions rather
than duplicating per-command decisions. No migration may delete data to make
the new layout appear complete.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | minor | skm-global-user-installation | Adds a user-owned manifest and changes pre-1.0 global CLI/path contracts; reserve 0.10.0 from released 0.9.1. |

## Release Reservation

Reservation status is `applied`, owner `global-user-installation`, timing
`development-start`. The seed already contains the 0.10.0 Cargo source and lockfile.
On 2026-11-26 the shared peer board confirmed baseline 0.9.1 and target 0.10.0;
remote tag lookup found no 0.10.0 tag. Reuse this release without another bump.
Integration, acceptance, and publication remain pending.

## Memory Impact

Status: pending

Rationale: Implementation and migration behavior remain unverified. Before
development, retain pending; before done, record stable resulting behavior in
memory and resolve this section to updated or none with evidence. The separate
specification-writing task records the user's requested global-home design and
confirmed project-local default in decisions and changelog, explicitly as
planned behavior rather than shipped functionality.
