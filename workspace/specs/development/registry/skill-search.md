# Specification: Registry Skill Search

## Status

State: development

Implementation and validation are complete locally for SKM 0.5.0; integration
into `development` and release through `main` have not occurred.

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

- `task check`: formatting, warnings-free Clippy, compilation, and workspace
  documentation gates passed.
- `task test`: 105 Rust tests and 6 documentation validator tests passed,
  including isolated search-only and search/add/link coverage.
- `task build`: produced the locked optimized SKM 0.5.0 binary.
- `git diff --check`: passed.

The spec remains in development until confirmed integration into the shared
`development` branch.
