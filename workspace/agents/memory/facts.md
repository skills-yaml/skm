# Facts

## 2026-08-19 - Workspace lifecycle targets

- Type: fact
- Source: repo
- Confidence: high
- Review: none
- Supersedes: none

Content:

This repository's workspace-docs test target is the `development` branch
(prerelease channel). The production target is `main`. Feature-branch checkout
is not evidence of either event.

## 2026-08-29 - Skill removal preflights before mutation

- Type: fact
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

`skm remove` resolves and validates every configured agent target before
confirmation or writes. Declining and dry-run are non-mutating. After confirmed
configuration removal, agent unlink failures are collected and reported after
all preflighted targets have been attempted. Link and unlink operations refuse
symlinked namespace parents so nested skill names cannot redirect mutations
outside the selected agent skills namespace.

## 2026-09-05 - Cleanup preserves authoritative references and unmanaged content

- Type: fact
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

Orphan cleanup derives ownership from the current `skills.yaml` and the
scope-matched development configuration; a skill's presence in registry cache
is not ownership evidence. Old-version cleanup preserves `latest`, `default`,
and manifest-pinned versions. Reset removes descendant symlink entries while
preserving agent skills-root symlinks and real files or directories.
