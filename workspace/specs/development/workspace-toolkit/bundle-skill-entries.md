# Bundle Entries in the Skills List

## Status

State: development

Primary feature: `workspace-toolkit`

Status rationale: The bundle entry implementation and additive schema-2 exact
release history are locally integrated and independently reviewed: source
`ec71776` → peer integration `bf319e7`, and source `69578de` → `f9d16e6`
(the board records `bundle-version-review` approval of `69578de`). Both are
included in combined peer revision `6e76f98`. This local integration is not a
remote `development` merge. Combined acceptance, final candidate review and
shared-test delivery remain pending. The separate default registry publisher's
history has not been updated; exact pins require published history.

## Problem and Evidence

At baseline `68568ea`, `skm add <ns>/<bundle> --kind bundle --yes` expands the
bundle into individual `SkillSpec` entries in `skills.yaml` (`src/bundle.rs:320-332`).
The bundle identity is lost after the add. Consequences:

- `skm skill upgrade --all` cannot detect when the published bundle gains new
  members — it only iterates existing `config.skills` pins
  (`src/version_manager.rs:408+`).
- `skm skill outdated` has the same blind spot.
- Re-running `skm add --kind bundle` is the only way to pick up membership
  changes, and it requires the user to remember the bundle name.
- The lockfile (`skills.lock.yaml`) records only unpacked members, so `skm check`
  cannot detect bundle membership drift.

The registry format already supports schema-2 bundles
(`skills/<ns>/manifest.yaml` `bundles:` map). The project manifest should mirror
that: a bundle is a single declarative entry that resolves at install time.

## Goals, Consumers, and Scope

- Add an optional `kind` field to `SkillSpec` in `skills.yaml`. `kind: skill`
  (default, omitted for backward compatibility) is the existing behavior.
  `kind: bundle` declares a registry-published bundle as a single entry.
- At resolve/install time, expand bundle entries into their members plus
  transitive same-registry dependencies, exactly as `bundle::prepare` does today.
  The `skills.yaml` entry itself is never rewritten to unpack members.
- The lockfile records both the bundle entry and its resolved member skills, so
  `skm check` can detect both version drift and membership drift.
- `skm add --kind bundle` writes a `kind: bundle` entry instead of unpacking.
  Repeating the same add is a no-op.
- `skm remove <bundle-name>` removes the bundle entry and unlinks all resolved
  members.
- `skm skill outdated` / `skm skill upgrade` re-resolve bundle entries against
  the refreshed registry and report/apply membership and version changes.
- `skm list` shows bundle entries with their resolved member count.

Consumers are CLI users who want to track a published bundle as a unit, scripts
that need deterministic bundle tracking in the manifest, and CI (`skm check`)
that needs to detect bundle drift.

Non-goals: bump the registry schema or remove its legacy projection; change toolkit bundles (`bundles:` key
in `skills.yaml` remains toolkit-only); support local-path bundles; support
global bundle entries; change search behavior; unpack bundles in the manifest.

## Configuration Contract

```yaml
skills:
  - name: workspace/fix-bug          # kind omitted = skill (backward compatible)
    version: 0.5.1
    source: default
  - name: workspace/all-workspace-skills   # kind: bundle — stays as one entry
    kind: bundle
    version: latest                 # bundle version alias or exact
    source: default
```

`SkillSpec` gains:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub kind: Option<String>,   // "skill" (default) or "bundle"
```

Validation:

- `kind` must be `skill` or `bundle`. Other values are rejected at load time.
- `kind: bundle` requires `source` (registry name); `path` is mutually exclusive
  with `kind: bundle`.
- Bundle names must pass `validate_skill_name` and have exactly
  `namespace/name` form.
- Only schema-2 namespace manifests can provide bundles; a schema-1 namespace
  with a `kind: bundle` entry fails at resolve time with an actionable error.

## Resolution Contract

`linker::resolve_skill_dependency_closure` gains a branch: for `kind: bundle`
entries, read the registry's `skills/<ns>/manifest.yaml`, look up the bundle in
`bundles:`, and produce `SkillSpec` entries for each member package (with the
manifest's exact pinned version and the bundle's source). Then expand the normal
dependency closure over those members. The original bundle entry is preserved in
the returned list alongside its resolved members (marked so the lockfile can
distinguish them).

Equivalently, introduce a `ResolvedEntry` enum:

```rust
enum ResolvedEntry {
    Skill(SkillSpec),                              // kind: skill or local path
    Bundle { spec: SkillSpec, members: Vec<SkillSpec> }, // kind: bundle + resolved members
}
```

All consumers (install, list, check, remove, upgrade) iterate resolved entries;
bundle members participate in linking, target uniqueness, and symlink validation
exactly as today.

### Published bundle history model

Schema-2 publishers may add `bundle_versions: {bundle-id: {release: {member: version}}}`.
Release keys and member versions are exact numeric semantic versions. Each history
is nonempty and identifies a current bundle; the greatest numeric release must
exactly match its legacy `bundles` membership and `packages` versions. Historical
members need not remain in current `packages`, but their exact source directories
must remain available and pass ordinary path, integrity and dependency checks.
Publishers retain the existing fields for older readers. Alias/default selects
the greatest release; a legacy bundle without history retains alias behavior and
rejects exact selectors with an actionable missing-history error. Invalid history
or an inconsistent latest projection is rejected before installation.

## Lockfile Contract

`skills.lock.yaml` `kind: skills` locks gain a `bundles` section alongside
`skills:`:

```yaml
schema_version: 1
kind: skills
project: my-project
agents: [codex]
bundles:
  - name: workspace/all-workspace-skills
    version: latest              # as declared in skills.yaml
    source: default
    resolved_version: 1.0.0     # omitted for legacy unversioned bundles
    members:                     # resolved at install time
      - name: workspace/fix-bug
        version: 0.5.1
      - name: workspace/write-spec
        version: 0.7.1
skills:
  - name: workspace/fix-bug     # unpacked members also listed for link verification
    version: 0.5.1
    source: default
outputs: [...]
```

`skm check` recomputes the bundle resolution and fails if:
- the bundle's resolved members differ from the lockfile's recorded members
  (membership drift), or
- any member's resolved version differs (version drift), or
- the selected published release differs, even when members are identical.
Exact pins remain stable against unrelated newer releases but detect changes to
the selected release's resolved members.

## Command Behavior

| Command | Bundle entry behavior |
| --- | --- |
| `skm add <ns>/<bundle> --kind bundle --yes` | Writes `{name, kind: bundle, version, source}` to `skills:`. Does not unpack. Idempotent. |
| `skm install` | Resolves bundle entries → members, links all, writes lockfile with bundle + members. |
| `skm list` | Shows `workspace/all-workspace-skills (bundle, 20 members)` with link status. |
| `skm check` | Verifies bundle membership + member versions + all symlinks. |
| `skm remove <bundle>` | Removes the bundle entry, unlinks all resolved members. |
| `skm skill outdated` | For bundle entries, re-resolves from cache; reports new members and version changes. |
| `skm skill upgrade <bundle>` / `--all` | Re-resolves bundle, updates lockfile members. Manifest entry unchanged (still `kind: bundle`). |
| `skm skill use <bundle>@<version>` | Changes the bundle entry's version pin. |

## Compatibility and Risks

- Fully backward compatible: existing `skills.yaml` files without `kind` parse
  identically (`kind` defaults to `None` → treated as `skill`).
- Existing unpacked bundle members in `skills.yaml` continue to work as ordinary
  skills. A project can mix unpacked skills and bundle entries.
- If a bundle entry and an unpacked member resolve to the same skill name, the
  pins must be compatible (same version/source) or resolution fails.
- `skm add --kind bundle` on a project that already has unpacked members of the
  same bundle: detects overlap, keeps existing pins if compatible, writes the
  bundle entry alongside.
- No new runtime dependencies.

## Acceptance Criteria

1. `skills.yaml` with a `kind: bundle` entry parses and validates; invalid `kind`
   values are rejected.
2. `skm install` resolves the bundle, links all members, writes a lockfile with
   both `bundles:` and resolved `skills:` sections.
3. `skm check` passes after install and fails when the registry bundle gains a
   new member (membership drift) or a member publishes a new version.
4. `skm add --kind bundle --yes` writes a single bundle entry, does not unpack;
   repeating is a no-op.
5. `skm remove <bundle>` removes the entry and unlinks members.
6. `skm skill outdated` reports new bundle members after `skm cache refresh`.
7. `skm skill upgrade --all` updates the lockfile for changed alias bundles without
   rewriting the manifest's bundle entry; exact pins remain pinned, with guidance
   to use `skill use` for a newer release.
8. Existing skills-only projects (no `kind: bundle`) are unaffected; all current
   tests pass unchanged or with minimal adaptation.
9. `task check` and `task test` pass.
10. Exact `skill use bundle@X.Y.Z` atomically changes the manifest, links and lock;
    aliases select the greatest numeric release. `outdated` reports newer releases
    and member differences. Missing, invalid or inconsistent history fails safely.
11. Historical membership and versions resolve through normal dependency, path and
    integrity validation; rollback, shared ownership and dry-run remain safe.
12. Legacy schema-2 bundles and old locks continue to work; lock drift distinguishes
    the selected release even when its member set is unchanged.

## Affected Areas

- `src/config.rs`: `SkillSpec.kind` field, validation.
- `src/linker.rs`: bundle resolution in dependency closure; `ResolvedEntry`.
- `src/bundle.rs`: write bundle entry instead of unpacking; resolve members at
  plan time.
- `src/installer.rs`: lockfile `bundles` section; plan includes bundle members.
- `src/main.rs`: add routing, list/check display, remove.
- `src/version_manager.rs`: outdated/upgrade/use for bundle entries and release guidance.
- `src/search.rs`: aligned namespace-manifest size boundary without output changes.
- `src/remover.rs`: remove bundle entry + members.
- `README.md`, `workspace/docs/registry-format.md`: documentation.
- Tests in each affected module.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | minor | skm-global-user-installation | Adds `kind: bundle` to the public skills.yaml contract; backward-compatible minor addition at 0.y.z, sharing the open 0.10.0 reservation. |

## Memory Impact

Status: updated

Rationale: Prior result recorded in workspace/agents/memory/decisions.md and
workspace/agents/memory/changelog.md on 2026-11-10: registry bundles are
first-class `kind: bundle` entries in skills.yaml, resolved at install time,
with the lockfile tracking both bundle and members.

The exact published-version decision is recorded in decisions.md and changelog.md on 2026-11-26; combined acceptance and remote delivery remain pending.

## Validation Gates

Run focused bundle, registry-format and maintenance tests, `task versions:check`,
then `task check` and `task test` on the candidate. Independent exact-candidate
review is required for the public schema and data-integrity changes.
