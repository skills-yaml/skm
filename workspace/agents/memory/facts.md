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

## 2026-09-19 - Init searches registries on demand

- Type: fact
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

The sequential init flow discovers project and inherited global registry
skills only after the user chooses search or refresh. Search matches names,
registries, and versions; numbered results retain source/version metadata and
reject same-name source collisions. Local and matching cached registries are
read directly, while uncached or refreshed Git sources are inspected in
temporary storage without installing skills or mutating the registry cache.

## 2026-09-20 - SKM 0.6.0 sequential init and trusted updater released

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

SKM 0.6.0 released sequential cooked-terminal init prompts, the read-only
`skm search` boundary, and manifest-bound transactional self-updates through
PR #32 and main commit `769f5a433c635a56b86e744513dd71d7a42a181c`.
Production CI run `35496461707` passed. Qualification run `35496742318`
verified a real update and idempotent no-op on Linux, macOS Intel, macOS Apple
Silicon, and Windows while Release Artifacts run `35496461689` remained held
for required reviewer approval. The approved run published all four archives,
checksums, and `skm-release.json`; the Linux checksum, production binary
identity, release tag, and manifest were verified against the exact commit.

## 2026-09-22 - Agent skill targets cover sixteen agents

- Type: fact
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

SKM supports Claude, Codex, Copilot, Cursor, Antigravity, Pi, OpenCode, Cline,
Kilo Code, Gemini CLI, Goose, Crush, OpenHands, Grok, Qwen Code, and Hermes.
Codex uses project and global `.agents/skills`; it does not use
`.codex/skills`. Hermes uses global `~/.hermes/skills` and has no project-local
target. Codex, Antigravity, Goose, and OpenHands share project
`.agents/skills`, which SKM deduplicates.

## 2026-09-22 - Workspace Docs 6 toolkit manifests are supported

- Type: fact
- Source: spec
- Confidence: high
- Review: before the first production release containing this change
- Supersedes: 2026-08-19 - Support Workspace Docs 5 Toolkits (supported-major set only)

Content:

SKM's explicit toolkit compatibility allowlist accepts Workspace Docs `4.x`,
`5.x`, and `6.x`. Missing, malformed, and future-major declarations remain
fail-closed, and `minimum_skm_version` is enforced independently.

## 2026-09-22 - Backlog implementations released to development

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

PR #38 integrated Workspace Docs 6 toolkit compatibility and sixteen-agent
skill-directory support into `development` at
`63b243a9dcd1de6e0c09dd248713e5503f6c85c6`. CI run `35761237089` passed.
Release run `35761237088` built Linux, macOS Intel, macOS Apple Silicon, and
Windows packages and published `development-latest`; its manifest, Linux
checksum, and embedded development-channel commit identity were verified.

## 2026-09-23 - SKM production promotion at a6cdef2

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

PR #40 promoted Workspace Docs 6 toolkit compatibility, sixteen-agent skill
directory support, and interactive help update notices through `main` at
`a6cdef2df8fb0ec35f3b6e9e33343e6fbe0e9d96`. Production CI run
`35865013795` passed, and Release Artifacts run `35865013776` published the
complete `prod-latest` asset set. The manifest names that exact commit and all
four archives; the Linux archive matched its published checksum and manifest
digest. The production Linux artifact reported the expected embedded identity
and printed the update notice before interactive help output.

## 2026-09-23 - Skill target discovery and search metadata contract

- Type: fact
- Source: user and implementation
- Confidence: high
- Review: after development integration
- Supersedes: none

Content:

Skills-only install, list, and check require at least one effective agent skill
target for a nonempty skill set. Namespaced registry skills link directly below
the agent's skills directory using the final skill name, matching portable
`SKILL.md` discovery; duplicate final names are rejected before install. Search
shows bounded `SKILL.md` descriptions and only registry bundles explicitly
published in schema-2 namespace manifests. Toolkit bundles are separate; the
current Workspace registry schema-1 manifest publishes no skill bundles.

## 2026-09-23 - Discoverable skill install released to development

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

Commit `91da59b8a6e7a2226a650dad481ed2609cbcb62c` integrated the
install-target and search improvements into `development`. CI run
`35871598722` passed. Release Artifacts run `35871598742` published the
complete nine-asset `development-latest` prerelease at the same commit. A
local production build installed `workspace/wk-spec` and its `write-spec`
dependency as direct Codex skill links, and `skm check` passed.

## 2026-09-23 - Search distinguishes skills, collections, and bundles

- Type: fact
- Source: user and implementation
- Confidence: high
- Review: after development integration
- Supersedes: 2026-09-23 - Skill target discovery and search metadata contract (search presentation only)

Content:

SKM search labels each skill's name and description and displays its exact
declared dependencies. Search reads schema-1 or schema-2 namespace manifests to
show published skill collections such as Workspace, while only explicit valid
schema-2 bundle declarations appear as bundles with member identities. A
namespace collection is browseable and does not itself provide group install.
The current Workspace registry manifest is schema 1 and lists 19 skills but no
installable bundle. JSON retains existing fields and adds dependencies,
collections, and bundle package names.
