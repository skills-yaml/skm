# Specification: Cleanup Safety Conformance

## Status

State: done

The cleanup data-preservation corrections were integrated through PR #2, then
released through `main` by PR #4 on 2026-09-05 as merge commit `e12ba7d`. The
`prod-latest` release artifacts were published successfully.

## Problem

Global orphan detection compares skill paths with registry names, old-version
cleanup ignores active aliases and manifest pins, and reset removes whole agent
skill directories instead of limiting removal to symlinks.

## Scope

- Derive orphan candidates from known skill and development configurations.
- Refuse orphan cleanup when no authoritative project configuration exists.
- Preserve versions referenced by `latest`, `default`, or the current manifest.
- Make reset remove symlink entries only, preserving real files and directories.
- Correct dry-run size accounting for old-version cleanup.

## Acceptance Criteria

- Valid global skill links are not classified as registry-name orphans.
- Orphan cleanup without a usable project manifest fails without writes.
- Active aliases and manifest-pinned versions survive old-version cleanup.
- Reset never deletes real agent skill files or directories.
- Dry-run reports the actual candidate byte total and changes nothing.
- Focused deterministic tests cover each safety invariant.

## Affected Areas

- `src/cleaner.rs`
- `src/dev.rs`
- `workspace/specs/README.md`
- `workspace/agents/memory/`

## Validation Gates

- `task check`
- `task test`
- `git diff --check`

## Risks

- Conservative discovery may leave an unprovable orphan or old version in
  place; preserving data takes precedence over aggressive cleanup.
- Registry aliases may be relative symlinks and must be resolved without
  following unrelated directory trees.

## Memory Impact

Status: `updated`

Rationale: Recorded authoritative orphan ownership, protected-version, and
reset data-preservation behavior in `workspace/agents/memory/facts.md` and
`workspace/agents/memory/changelog.md`.

## Validation Result

- `task check`: passed formatting, warnings-free Clippy, compilation, and
  workspace documentation gates.
- `task test`: passed 66 Rust tests and 6 workspace-validator tests.
- `git diff --check`: passed.
- Focused coverage confirms configured and development links are preserved,
  cached-but-unconfigured links remain orphan candidates, orphan cleanup
  without a manifest is non-mutating, aliases and pins survive old-version
  cleanup, dry-run candidate sizing uses the planned byte total, and reset
  preserves real content and agent skills-root symlinks.
- Review fixed project-link discovery after configuration reset, removed cache
  presence as false ownership evidence, and excluded agent skills-root
  symlinks from cleanup and reset candidates.
- Production evidence: PR #4 merged the validated `development` branch into
  `main` as `e12ba7d`; main CI and Release Artifacts run `33967911337` passed
  and published the `prod-latest` assets.
