# Specification: Dedicated Add Command for Search Results

## Status

State: development

Implementation and local validation are complete. Integration into
`development` and release through `main` have not occurred.

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
