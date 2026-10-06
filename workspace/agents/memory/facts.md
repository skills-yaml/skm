# Facts

## 2026-09-30 - Workspace Docs 6.0.0 integrated into development

- Type: fact
- Source: pull request and CI
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

PR #72 merged the Workspace Docs 6.0.0 candidate adoption into `development`
at `c49f1cc47a477d92c2923677d49108ff222e9b01` on 2026-09-30 after
Validate run `36682939489` passed. The migration spec is in `test`; the draft
standard is not yet a production release.

## 2026-09-29 - SKM 0.9.0 candidate reserved for active work

- Type: fact
- Source: repository and user-directed migration
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

The `prod-latest` manifest reported SKM 0.8.0 at `07b4bba` on 2026-09-29;
`development-latest` still reported 0.8.0 at `72eaf7d`. The repository
reserves one applied 0.9.0 candidate for its active test-channel CLI and
documentation specifications and the Workspace Docs 6.0.0 adoption. The bump
is recorded once in `Cargo.toml` and `Cargo.lock`; it is not evidence of a
production release.

## 2026-09-29 - Registry release notification integrated into development

- Type: fact
- Source: pull request and CI
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

PR #67 merged the production-only `notify-registry` release job into
`development` at `a8ad8d795ccaeadf64291a693de785c2ec520d2b` on 2026-09-29 after Validate run `36548080616` passed. The
job is skipped until the `REGISTRY_APP_CLIENT_ID` variable and
`REGISTRY_APP_PRIVATE_KEY` secret are configured; the Registry's hourly poll
covers releases meanwhile.

## 2026-09-28 - Command contracts integrated into development

- Type: fact
- Source: pull request and CI
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

PR #69 merged the command contract reconciliation into `development` at
`46d70fadc5a3c92bd4013e89e578ce03bb587733` on 2026-09-28 after
Validate run `36476786848` passed. The specification is in `test`;
production release remains pending.

## 2026-09-26 - Legacy updater identity handshake integrated into development

- Type: fact
- Source: pull request and CI
- Confidence: high
- Review: after development release qualification
- Supersedes: none

Content:

PR #66 merged the environment-gated legacy updater identity probe into
`development` at `ed5b0a0987964b2c4aa0f7e92c948f937d56a9ee` on
2026-09-26 after Validate run `36270555944` passed. Development Release
Artifacts run `36271054364` published the nine-asset release. A preserved
older 0.8.0 Linux binary at `07b4bba` self-updated to `ed5b0a0`; the updated
binary reported its expected identity and a repeat upgrade made no change.
The specification is in `test`; cross-platform qualification and production
promotion remain pending.

## 2026-09-26 - Legacy updater probes caused the development update failure

- Type: fact
- Source: reproduced older binary and updater source
- Confidence: high
- Review: after cross-platform release qualification
- Supersedes: 2026-09-26 - Breaking CLI requires a bridge updater release

Content:

The 0.8.0 production updater invokes `version` with `SKM_NO_UPDATE_CHECK=1`
to verify a staged release and invokes the same probe after replacement on
Windows. The reorganized development binary removed that command, causing
`skm update` to fail identity verification. A new candidate that answers only
this internal probe lets older managed binaries self-update directly; a bridge
updater release is not a prerequisite. Cross-platform release qualification is
still required before production promotion.

## 2026-09-26 - Workspace documentation separation integrated into development

- Type: fact
- Source: pull request and CI
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

PR #60 merged the separation of Workspace toolkit documentation from the
README into `development` at `506d47a358d406e2759c8ef3dd47eb0863bf52ae` on 2026-09-26 after Validate run `36213304307`
passed. The specification is in `test`; production release is pending.

## 2026-09-26 - CLI command reorganization integrated into development

- Type: fact
- Source: pull request and CI
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

PR #63 merged the cache, skill version, and self-update command reorganization
into `development` at `2c852c248b54e6982a92584c956dd2bc036ab8fc` on
2026-09-26 after Validate run `36212734539` passed. The specification is in
`test`; production promotion still requires a bridge updater release.

## 2026-09-26 - Breaking CLI requires a bridge updater release

- Type: fact
- Source: code and release qualification contract
- Confidence: high
- Review: after bridge release
- Supersedes: none

Content:

Older SKM updaters verify a staged binary by invoking its top-level `version`
command. The reorganized CLI removes that command, so old binaries cannot
self-update directly to it. A separately qualified bridge release with an
updater that verifies `skm self version` is required before production promotion;
the official installer can install the new binary directly.

## 2026-09-25 - SKM 0.8.0 search and add workflow released to production

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: 2026-09-25 - Add confirmation integrated into development; 2026-09-25 - Two-column search results integrated into development

Content:

PR #58 promoted two-column `skm search` results and planned `skm add`
confirmation to `main`. PR #59 prepared SKM 0.8.0 at
`07b4bba9c1d5f3de3964f398ff8296afb617db6e`. Production CI
`36197428227` and four-platform update qualification `36198106673` passed.
After required reviewer approval, Release Artifacts run `36197428257`
published the nine-asset `prod-latest` release. Its manifest, Linux archive
checksum, and released binary identity matched version 0.8.0 and the exact
production commit.

## 2026-09-25 - Add confirmation integrated into development

- Type: fact
- Source: command
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

PR #56 merged the plan-and-confirm `skm add` behavior for skills and bundles
into `development` at `ed645db0bd5c4f31aa15411ebb3aa9adaaf38a87` on
2026-09-25 after its Validate check passed. The specification is in `test`
pending a confirmed production release.

## 2026-09-25 - Two-column search results integrated into development

- Type: fact
- Source: command
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

PR #54 merged the two-column `skm search` text presentation into `development`
at `9033c29891a688deb053bda4b800bbe598e29dbb` on 2026-09-25 after its
Validate check passed. The corresponding specification is in `test` pending a
confirmed production release.

## 2026-09-24 - SKM 0.7.0 bundle workflow released to production

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: 2026-09-24 - Unified registry search and add integrated into development; 2026-09-24 - Registry bundle consumer integrated into development

Content:

SKM 0.7.0 at `44a502af733e1ef3e0a360fe6e00d72959be261d` was published
through the reviewed `release-prod` environment after production CI and
four-platform update qualification passed. Its nine release assets, four
archive checksums, release manifest, and production binary identity were
verified. It supports one `skm search` and `skm add` workflow for published
skills and schema-2 bundles. Before release, the 20-member
`workspace/all-workspace-skills` bundle published in Registry passed live
search, preview, add, `skm check`, and repeat-add with the development binary.

## 2026-09-24 - Unified registry search and add integrated into development

- Type: fact
- Source: command
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

PR #46 merged unified skill and published bundle results for `skm search`
and the shared `skm add` route into `development` at
`34d516ed1bf357fbc0d476de0c77c9485378fa5b` after its Validate check
passed. The specification moved to `test`; production release remains pending.

## 2026-09-24 - Registry bundle consumer integrated into development

- Type: fact
- Source: command
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

PR #44 merged the `skm bundle add` consumer into `development` at
`29ba7dc77315be65e6e77d4d6932fbbc9018627d` after its Validate check
passed. The SKM specification moved to `test`; released Registry schema-2
bundle qualification and production release remain pending.

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

## 2026-09-24 - Skill discovery and search released to production

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

PR #42 merged the discoverable skill-target and search improvements into
`main`. Commit `9c8153a23bb3c8f2c3148895962efceaa60a5825` passed production
CI `35962863500` and release-update qualification `35963192731` on Linux,
macOS Intel, macOS Apple Silicon, and Windows. After reviewer approval, Release
Artifacts `35962863697` published the complete nine-asset `prod-latest`
release. The Linux archive checksum matched; the manifest and binary reported
the exact production commit, and a released-binary search smoke showed labeled
skill details, dependencies, and the 19-skill Workspace collection.

## 2026-09-24 - Bundle-capable SKM 0.7.0 qualified on the development channel

- Type: fact
- Source: command
- Confidence: high
- Review: after Registry bundle publication
- Supersedes: none

Content:

Release Artifacts run `35992853126` published the complete nine-asset SKM
0.7.0 `development-latest` prerelease from
`4c8ed3a663dfb029329ab348a7e5970ac1c9ec25`. The Linux archive checksum,
manifest version and commit, and released binary identity matched. The released
binary recognized the staged `workspace/all-workspace-skills` bundle as 20
existing skill pins with no changes on repeat, and `skm check` passed. The
Workspace Registry bundle and SKM 0.7.0 production release remain pending.

## 2026-10-01 - Support Workspace Docs 7 Compatibility

- Type: fact
- Source: repo
- Confidence: high
- Review: none
- Supersedes: Supported-major set ending at Workspace Docs 6

Content:

Registry bundle and local toolkit validation explicitly support Workspace Docs
4.x, 5.x, 6.x, and 7.x. Future majors remain rejected; minimum SKM version,
integrity, adapter, and transaction checks remain independently enforced.
Production and development release events must be verified separately.

## 2026-10-01 - Release Workspace Docs 7 Compatibility

- Type: fact
- Source: repo
- Confidence: high
- Review: none
- Supersedes: V7 bundle rejection in previously published SKM builds

Content:

PR #76 integrated v7 compatibility into development at 9068581; the published
0.9.0 development artifact passed checksum, identity, fresh twenty-member
bundle installation, skm check, and no-op repetition. PR #75 released the
focused 0.8.1 maintenance fix through main at 49468aa; production CI and all
four platforms passed, and the published Linux artifact repeated that live
qualification. Future compatibility majors remain explicitly rejected.

## 2026-10-06 - Integrate publisher-independent formats and Workspace Docs 7

- Type: fact
- Source: repo
- Confidence: high
- Review: after main merge
- Supersedes: none

Content:

PR #79 merged into development at 6e572b29ca35b4ecafd841d870d5cbc8e08b4a32 on 2026-10-06 after CI run 37450733442 passed. The reviewed combined candidate passed task check, all 202 tests, and task build. Main promotion and publication remain pending. The repository governance standard is 7.0.0; SKM runtime owns its registry format without publisher-specific policy. This supersedes the older supported-major runtime rule.

## 2026-10-06 - Complete Workspace Docs 7 and publisher-independent formats

- Type: fact
- Source: repo
- Confidence: high
- Review: after production publication
- Supersedes: none

Content:

PR #80 merged the confirmed development implementation into main at 79658625c5bb30cd84b0ad5185237e28e20038eb on 2026-10-06; actual main CI run 37451646628 passed task check, all 202 tests, and task build. Acceptance, documentation, catalog, applied SKM 0.9.0, and memory are reconciled. Production publication remains separately subject to qualification and human release-prod approval. The earlier supported-major allowlist and Workspace-specific runtime contract are superseded; Workspace skills own governance compatibility while SKM validates its installation format.

## 2026-10-06 - Integrate supplemental legacy updater qualification

- Type: fact
- Source: repo
- Confidence: high
- Review: after production qualification
- Supersedes: none

Content:

PR #82 merged into development at 1b6a6e0c501a05861e697399c6752343cd66c3a0 on 2026-10-06 after CI 37453601758 passed. Independent review and full local check, 202 tests, and build passed; main integration and native four-platform qualification remain pending before publication.

## 2026-10-06 - Publish protected SKM 0.9.0

- Type: fact
- Source: repo and release verification
- Confidence: high
- Review: none
- Supersedes: production publication pending for shared SKM 0.9.0

Content:

PR #83 merged into main at 7a289ab9edcb8799c295041b2fab63a79ba233fa; actual main CI 37454486476 and development CI 37454491771 passed. Qualification 37455327437 passed current-pair updates, actual legacy 0.8.0 updates, and modern-bootstrap rejection on Linux, macOS Intel/ARM, and Windows while production remained held. Release Artifacts 37454486538 then published verified SKM 0.9.0 through the normal eligible-reviewer approval path on 2026-10-06; all nine assets, manifest/source identity, checksums, Linux binary identity, and repeat-update stability were verified. Main/runtime uses publisher-independent SKM formats and repository governance 7.0.0. Registry dispatch remains pending because its optional credential variable is absent.
