# Help update notice

State: test
Primary feature: `updates`

## Integration evidence

Direct commit `2107f9e` integrated the implementation into this repository's
configured `development` test branch on 2026-09-22. `task check` and `task test`
passed before integration. Production release has not been confirmed.

## Problem

Clap renders `skm help` and `--help` by exiting during argument parsing.
The startup update check currently runs after parsing, so help requests never
show an available update notice.

## Scope

- Run the existing bounded, best-effort startup notice before Clap prints help.
- Cover root help, the `help` subcommand, and subcommand `--help`.
- Preserve Clap help text and exit behavior, and keep invalid arguments free of
  an extra update check.
- Preserve the existing interactive-stderr-only notice and opt-out behavior.

## Acceptance criteria

- A managed SKM release with an available update prints its normal notice when
  an interactive user requests help.
- Help still exits successfully and prints Clap's normal help text.
- Invalid arguments do not trigger a help-path update check.
- Local or unmanaged builds and disabled checks remain silent.

## Affected areas

- `src/main.rs`: parse-error handling for help requests.
- `tests/release_contract.rs` and CLI parsing tests: help and worker ordering.
- `README.md`: help notice behavior.

## Validation gates

- Targeted tests for Clap help and invalid-argument classification.
- `task check` and `task test`.

## Memory Impact

Status: updated
Rationale: Help-request notification is a durable CLI behavior decision.
Memory: `workspace/agents/memory/decisions.md` and
`workspace/agents/memory/changelog.md`.
