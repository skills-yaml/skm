# Facts

## 2026-09-19 - Managed release updates use a strict release manifest

- Type: fact
- Source: spec
- Confidence: high
- Review: required before the first production release
- Supersedes: tag-only self-update discovery

Content:

SKM self-updates only official managed `prod` or `development` builds. It
requires `skm-release.json` to bind the channel, tag, version, commit, exact
platform archive set, sizes, and SHA-256 digests; it also requires the release
checksum asset and downloaded archive to agree. Publication stages a
recoverable release transaction, and production publication is held by the
`release-prod` environment until development update qualification completes.

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

## 2026-09-09 - Init edits manifests through a terminal wizard

- Type: fact
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

`skm init` opens a six-step terminal wizard with a live YAML preview, loading
the current directory's existing `skills.yaml` or starting a new draft. Saves
retain unknown configuration values, use atomic replacement, and check for
external file edits. Cancellation and unchanged saves preserve original bytes;
edited saves may normalize comments and formatting. `--non-interactive` remains
create-only; `--global` keeps the manifest in the current directory. Linux PTY
coverage runs through `task test:tui` after `task build`.

## 2026-09-09 - SKM 0.3.0 wizard released

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

SKM 0.3.0 introduced the configuration TUI in production via PR #8 and main
commit `69c0b87a64a65fda391a6c9685a861d0c0a618f1`. Release workflow
`34411227027` successfully published Linux, macOS Intel/Apple Silicon, and
Windows packages to `prod-latest`. All downloaded archive checksums matched,
and the Linux artifact reports `skm 0.3.0`.

## 2026-09-10 - SKM 0.4.0 registry dependencies released

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

SKM 0.4.0 released exact same-registry skill dependency resolution and the
Codex `.agents/skills/` target through PR #13 and main commit
`cd3d4a6e5d71dd3f27e8122e8cd8b6b323b15cb4`. CI run `34425103403` and release
run `34425103415` passed. All four platform packages were published, the
downloaded Linux checksum matched, and the artifact reports `skm 0.4.0`.

## 2026-09-15 - Standalone registry search adds unique results

- Type: fact
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

SKM 0.5.0 adds `skm search <query>` for case-insensitive canonical-name search
across project and inherited global registries. Results contain the registry,
available version, and copyable add command; `--registry`, `--limit`, and
`--json` support narrowing and automation. `--add` uses the discovered source
and version only for one exact or otherwise unique result, then runs the normal
dependency-aware validation and linking path. Ambiguous matches and missing
project manifests fail without adding a skill.

## 2026-09-16 - SKM 0.5.0 registry search released

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

SKM 0.5.0 released registry skill search through PR #18 and main commit
`488860f8fa078729879680c70f1075fdd9a3d3f2`. CI run `35151714476` and release
run `35151714469` passed. Linux, macOS Intel/Apple Silicon, and Windows packages
with checksums were published to `prod-latest`. The downloaded Linux package
matched its checksum, and its binary reports `skm 0.5.0` with the expected
`search` help.
