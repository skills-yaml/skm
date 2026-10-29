# Refresh cached registries with Git URL rewrites

## Status

State: `development`

Rationale: The reported refresh failure is reproduced by comparing a rewritten
cached origin with its original configured SSH URL. Implementation and review
are active; shared-test integration and main completion remain pending.

## Problem and Scope

Git applies `url.<base>.insteadOf` when `remote get-url origin` prints a cached
registry's origin. SKM compares that effective URL with the literal URL from
configuration, rejecting even an unchanged origin. Clearing and cloning again
does not repair the comparison. Follow up the completed
[command contract reconciliation](../../done/skill-lifecycle/command-contract-reconciliation.md).

Resolve the configured URL through Git in the cached repository's context and
compare effective fetch URLs. Preserve rejection of genuinely different or
missing origins, project-over-global configuration, fast-forward-only updates,
and existing cache contents on rejection. Do not normalize unrelated protocols
or repository names, modify user Git/manifest configuration, or clear caches.

## Acceptance Criteria

1. An unchanged origin whose configured SSH URL is rewritten by Git refreshes
   successfully and receives a new source commit without clearing the cache.
2. A configured URL and cached origin that resolve to the same fetch URL are
   accepted, including project overrides and repository-local rewrite rules.
3. Different effective URLs, missing origins, invalid cached repositories, and
   relative source paths matching a remote name fail before updates and preserve
   cached content and configuration. Existing remote names cannot override
   command-local URL validation.
4. Existing unrewritten refresh, project precedence, partial failure reporting,
   and fast-forward-only behavior remain intact without new dependencies.
5. Regression fails before the fix, passes afterward, receives independent
   review, and passes frozen `task check`, `task test`, and version validation.

## Affected Areas

- `src/registry.rs`: cached-origin validation before refresh.
- `tests/command_layout.rs`: isolated local-Git refresh regressions.
- `Taskfile.yml`: a focused cache command test entrypoint.
- `Cargo.toml`, `Cargo.lock`, `workspace/releases.json`: one patch reservation.
- This spec, catalog, and project memory: contract and delivery evidence.

## Implementation and Validation Plan

1. Check published baseline, shared reservations, and unrelated work; reserve
   the next compatible patch before implementation.
2. Add deterministic local-repository regressions with isolated Git URL rules;
   run the focused cache module and capture the original failure.
3. Resolve both URLs using Git in the same repository context before comparison.
4. Run the focused cache tests first, then command contract consumers. Resolve
   memory and acceptance evidence before independent exact-candidate review.
5. Freeze the reviewed candidate; run `task check`, `task test`, and `task build`.
   Deliver through development and main with actual revision checks. Track
   publication separately and respect existing protections.

## Validation Gates

`task test:cache` runs cache-refresh and command-contract integration tests.
Its inputs are registry refresh, CLI routing, project/base configuration,
integration fixtures, Git, Cargo manifests/lock, and Taskfile. Fixtures use
independent temporary homes, repositories, and Git configuration; no network is
needed. Cache tests precede configuration/CLI consumers. All commands must pass;
later relevant edits invalidate evidence. Full Cargo gates run sequentially.

## Risks and Recovery

Resolving in a different directory could miss repository-local rewrites. Use
the cached repository for both lookups. Retain explicit mismatch and missing
origin tests to ensure a false-positive fix does not weaken source validation.
Git performs URL expansion without contacting remotes; only the existing pull
updates content. Revert the bounded implementation to recover prior behavior.

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | patch | skm-cache-url-rewrite | Compatible correction to registry refresh under existing Git URL rewrite configuration; reserve 0.9.1 from published 0.9.0 and apply once at development start. |

## Memory Impact

Status: `updated`

Rationale: Record the durable requirement to compare Git-resolved fetch URLs
in the cached repository's context in `workspace/agents/memory/facts.md` and
`workspace/agents/memory/changelog.md`. No local Git rules, paths, or personal
configuration are recorded.

## Evidence

- Published production source is SKM 0.9.0; remote development matches the
  implementation base `37922cdaf9a7cb1fad3c5684ec4edc0afa854a73`.
- Shared tracked ledger contains no open SKM reservation; no local peer board
  or concurrent mutating peer exists. This isolated single-author task reserves
  SKM 0.9.1 in the ledger before implementation.
- AC-1/2/5: Before the fix, both global-rewrite and repository-local-rewrite
  regressions fail with the reported origin error. After the fix,
  `task test:cache` passes all four cache tests and all three command-contract
  consumers. The rewritten refresh receives new commits; a literal cached
  origin is also accepted when its configured alias resolves to that source.
- AC-3: Regressions reject different, missing, and non-repository origins,
  preserve Git configuration/HEAD and cached files, and do not fetch the new
  source commit. The existing unrewritten mismatch test also passes.
- AC-4: Existing project-precedence and install/upgrade command consumers pass.
  Inspection confirms the unchanged fast-forward-only pull and partial failure
  accounting. No runtime dependency or user configuration changes are included.
- `task workspace:check` and `task versions:check` pass. Independent review and
  complete frozen-candidate gates are required before delivery.
- Independent review of `9380ada` identified Git's remote-name interpretation
  of a configured relative source such as `origin`. The correction uses an
  unused command-local remote name to expand the configured URL literally;
  regressions cover remote-name rejection and collisions with validation names.
  Renew independent review and focused gates for the corrected candidate.
- Integration, main completion, and publication are pending.
