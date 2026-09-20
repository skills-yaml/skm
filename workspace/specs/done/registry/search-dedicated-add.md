# Specification: Dedicated Add Command for Search Results

## Status

State: done

PR #21 merged into `development` at
`a92cd4e49619a84f32c74e9772f37bb98da36621` on 2026-09-17. Development CI
run `35271724502` passed. Release run `35271724528` published all platform
artifacts successfully after retrying a transient GitHub asset-upload error.
PR #32 released the change in SKM 0.6.0 through `main` at
`769f5a433c635a56b86e744513dd71d7a42a181c`. Production CI run `35496461707`,
four-platform update qualification run `35496742318`, and Release Artifacts
run `35496461689` passed. The downloaded Linux production archive matched its
checksum and reported the expected version and commit.

## Scope

Keep `skm search` focused on registry discovery and direct users to the
dedicated `skm add` command when they want to install a result.

## Acceptance Criteria

- `skm search` no longer accepts `--add` or the add-only `--global` option.
- Search execution is always read-only and never changes `skills.yaml`, the
  registry cache, or agent links.
- Human-readable and JSON results retain a copyable `add_command`, formatted as
  `skm add <skill-name> --source <registry>`.
- `skm add` remains the only search-result installation path and keeps its
  existing project-local and `--global` behavior.
- CLI help and README examples document the read-only search contract and the
  separate add command.

## Affected Areas

- `src/main.rs`, `src/search.rs`
- `README.md`
- `workspace/specs/README.md`, `workspace/agents/memory/`

## Validation Gates

- `task check`
- `task test`, including search immutability, rejected mutation flags, and the
  dedicated add command in JSON output
- `task build`
- `git diff --check`

## Memory Impact

Status: updated

Rationale: Recorded the durable command boundary in
`workspace/agents/memory/decisions.md` and the corresponding memory change in
`workspace/agents/memory/changelog.md`.

## Implementation and Validation

Completed locally on 2026-09-17. The search command exposes only discovery
options, always follows its read-only path, and formats result suggestions with
the dedicated `skm add` command. The removed `--add` and `--global` search
options are rejected by argument parsing.

- `task check`: passed formatting, warnings-free Clippy, compilation, and
  workspace documentation validation.
- `task test`: passed 105 Rust tests and 6 workspace validator tests, including
  search immutability, rejected mutation flags, and JSON add-command coverage.
- `task build`: produced the locked optimized binary.
- Built CLI help lists only search options, and direct checks confirmed that
  `search --add` and `search --global` exit with argument errors.
- `git diff --check`: passed.
- PR #21 and post-merge development CI passed.
- `development-latest` points to the development merge and contains Linux,
  macOS Intel/Apple Silicon, and Windows packages with checksums. The downloaded
  Linux package matched checksum
  `d82961df5bed149f4ea0231f6a6c20ce8e2f6db2a084c830d3d477c0e2e06f45`;
  its binary exposes only the discovery options and rejects `search --add` and
  `search --global`.

The production manifest reports version `0.6.0`, channel `prod`, commit
`769f5a433c635a56b86e744513dd71d7a42a181c`, and all four supported platform
assets. The specification is complete.
