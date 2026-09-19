# NIB release and self-update parity

State: test

## Status

- Primary feature: `updates`
- Owner: SKM maintainers

## Problem

SKM currently discovers updates from a mutable Git tag and delegates installation
to a remotely fetched script. A failed or interrupted release can expose an
incomplete asset set, and the updater has no release manifest binding the
channel, commit, platform asset, size, and checksum together.

## Goals and scope

Port NIB's release/update system to SKM, adapted for the `skills-yaml/skm`
repository and `skm` binary:

- Embed SKM build channel and commit metadata, and display it in `skm version`.
- Replace tag-only update discovery with strict `skm-release.json` validation.
- Verify the manifest, checksum asset, archive size, archive digest, archive
  shape, staged binary identity, and target file before replacement.
- Publish the four supported archives and their checksums transactionally,
  together with a complete release manifest.
- Serialize release publication and add the development-to-production
  qualification workflow used by NIB.
- Preserve SKM's `skm update --check` and `--yes` flags while making their
  checks and replacements use the new updater.

## Non-goals

- Publishing a release as part of this change.
- Creating or approving GitHub environments. Repository administrators must
  configure `release-prod` before production publication is enabled.
- Supporting self-update for locally built, unsigned, or unsupported-platform
  binaries.

## Design

`build.rs` records the build commit and channel. `src/version.rs` is the single
source of the detailed version display used by both the CLI and staged-binary
verification.

`src/updater.rs` follows NIB's bounded HTTPS transport and strict manifest
schema. It accepts only the official repository, selected moving tag, expected
four-archive set, validated version/SHA values, a matching checksum asset, and
the selected platform archive. It stages the extracted regular binary beside
the installed binary, validates its managed build identity, guards replacement
with a lock and file identity checks, and performs NIB's Windows hand-off and
cleanup transaction where applicable. Startup checks remain best-effort and
may be disabled with `SKM_NO_UPDATE_CHECK=1`.

The release workflow uses pinned actions, channel/branch validation, serialized
publication, and `release-prod` / `release-development` environments. The
publisher creates `skm-release.json` from the exact archive set and performs
the staged release-tag transaction. Qualification workflows exercise a real
development self-update from one immutable release to the next while a matching
production release remains held for approval.

## Alternatives considered

- Keep the existing tag comparison and remote installer execution. It cannot
  establish that the binary belongs to the selected release or that all release
  assets were published coherently.
- Use release checksums without a manifest. Checksums do not bind a channel,
  commit, version, and complete platform asset set.

## Affected areas

- `build.rs`, `src/version.rs`, `src/main.rs`, and `src/updater.rs`
- `Cargo.toml` and `Cargo.lock`
- `.github/workflows/release.yml` and release-update qualification workflow
- `scripts/install.*`, `scripts/publish-release.sh`, and qualification scripts
- `Taskfile.yml`, README, tests, specs, and agent memory

## Acceptance criteria

- A managed release binary reports its version, channel, and 40-hex commit.
- The updater rejects malformed, incomplete, mismatched, oversized, or
  non-official release metadata and archive inputs.
- The updater never replaces a symlink or a changed executable, and it verifies
  a staged replacement before publication.
- Releases publish the exact four archives, matching checksum files, and a
  `skm-release.json` manifest as one recoverable transaction.
- Development release qualification proves notification, update, identity
  replacement, and idempotent no-op behavior on every supported platform.
- `task check` and `task test` pass.

## Validation gates

- Unit and integration tests for manifest parsing, checksums, channel identity,
  archive validation, and publisher/installer contracts.
- Shell and PowerShell syntax checks through Taskfile entrypoints.
- `task check`
- `task test`

## Risks and rollout

The new `release-prod` GitHub environment intentionally holds production
publication until repository reviewers approve it. Configure both release
environments before merging this change, run a development release pair and
qualification workflow, then approve the held production run. Existing release
assets remain installable until a newly published manifest is used.

## Integration evidence

Merged into the configured `development` test branch through PR #23 on
2026-09-19 at `7abc8435049ba55923b19df3f3fa0ef340150a16`. The PR Validate
workflow passed before merge. PR #26 supplied the Windows-only UUID dependency
used by the replacement path after the first native Windows release build
exposed the missing declaration. CI run `35471263033` and Release Artifacts run
`35471263031` then passed at
`b25954d74445e51c797de102054c4f744a0ed1f7`; the published
`development-latest` archive checksum, binary identity, and manifest were
verified. Cross-release self-update qualification and production publication
remain pending.

## Memory Impact

Status: updated
Rationale: SKM's release trust and publication contract is now a durable
project decision.
Memory: `workspace/agents/memory/facts.md` and
`workspace/agents/memory/changelog.md`
