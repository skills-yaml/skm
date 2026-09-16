# Specification: Registry Skill Search

## Status

State: done

PR #15 passed validation and merged into `development` at
`bd8dc5228d2bacb8d4db6b47c71bfe4f746671fc` on 2026-09-15. Development CI
run `35031426208` passed, and release run `35031426172` published verified
SKM 0.5.0 artifacts to `development-latest`. PR #18 released the feature
through `main` at `488860f8fa078729879680c70f1075fdd9a3d3f2` on 2026-09-16.
Production CI run `35151714476` and release run `35151714469` passed.

## Scope

Add a standalone command for finding skills by name across configured project
and inherited global registries, with an option to add and link an unambiguous
result.

## Acceptance Criteria

- `skm search <query>` searches skill names case-insensitively across configured
  project and inherited global registries.
- `--registry <name>` limits results to one configured registry, `--limit`
  bounds output, and `--json` provides deterministic machine-readable results.
- Results include the canonical skill name, available version, registry, and a
  copyable command that adds the result.
- `--add` adds and links a single exact or otherwise unique result using the
  discovered registry and version. Ambiguous matches fail without changing the
  manifest or agent links.
- Search-only execution does not change the project manifest or agent links.
  Registry failures are reported while usable results remain available.
- Existing registry dependency resolution applies when a found skill is added.

## Affected Areas

- `src/main.rs`, `src/search.rs`
- `Cargo.toml`, `Cargo.lock`, `README.md`
- `workspace/specs/README.md`, `workspace/agents/memory/`

## Validation Gates

- `task check`
- `task test`, including matching, filtering, ambiguity, JSON, and an isolated
  end-to-end search/add/link flow
- `task build`
- `git diff --check`

## Memory Impact

Status: updated

Rationale: Recorded the standalone search syntax, registry discovery behavior,
machine-readable output, dependency-aware add flow, and ambiguity rule in
`workspace/agents/memory/facts.md` and `workspace/agents/memory/changelog.md`.

## Implementation and Validation

Completed locally on 2026-09-15. Search merges project and global registry
configuration, safely scans local or matching cached registries, and inspects
uncached Git registries in bounded temporary clones. Results are filtered by
canonical skill name and include the discovered version and source. `--add`
uses the existing dependency-aware add path after resolving one result.

- Local `task check`: passed formatting, warnings-free Clippy, compilation,
  and workspace documentation gates.
- Local `task test`: passed 105 Rust tests and 6 documentation validator tests,
  including isolated search-only and search/add/link coverage.
- Local `task build`: produced the locked optimized SKM 0.5.0 binary.
- PR #15 and post-merge `development` CI: passed.
- `development-latest`: published Linux, macOS Intel/Apple Silicon, and Windows
  packages with checksums. The downloaded Linux checksum matched and its binary
  reports `skm 0.5.0` with the expected `search` help.
- PR #18 and production `main` CI: passed.
- `prod-latest`: points to the production merge and contains Linux, macOS
  Intel/Apple Silicon, and Windows packages with checksums. The downloaded
  Linux package matched checksum
  `e7c28b111b91df17e7d1c42dcf6668d768de4336c74006a7b3d6a24bdc91325c`;
  its binary reports `skm 0.5.0` and exposes the expected `search` help.

The feature is released through `main`, so this spec is complete.
