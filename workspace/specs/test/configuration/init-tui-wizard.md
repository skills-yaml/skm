# Specification: Init Configuration TUI

## Status

State: test

Integrated into `development` by PR #7 on 2026-09-09 at
`db7d02f44dda9a97def90633bab6d83d07c5e27d`, after CI passed. Production release
through `main` is pending.

## Scope

Replace the `skm init` text prompts with a terminal wizard for creating and
editing the current directory's `skills.yaml`. Guide users through project
metadata, agents, registries, skills, optional toolkit/workspace settings, and
review, with a continuously visible YAML preview and backward navigation.

## Acceptance Criteria

- Load an existing manifest, retaining optional and unknown configuration
  fields, or start a new editable configuration when no file exists.
- Show the loaded file initially and the draft as edits are made. Support
  scrolling the preview and adapting to smaller terminals.
- Allow editing metadata, toggling all supported agents, adding/editing/removing
  registries and registry/local skills, and configuring toolkit/workspace fields.
- Validate input and save only from the final review step. Cancellation leaves
  the manifest unchanged; invalid YAML and unsafe file targets are not replaced.
- Detect concurrent file edits before saving and write via an atomic replacement.
- Preserve `--non-interactive` creation and existing init flags; command-line
  selections appear in the wizard before review. Existing files remain protected
  from non-interactive replacement. `--advanced` opens the same complete wizard.
- Restore terminal state on normal exit, cancellation, errors, and panic; reject
  interactive use without a terminal with an actionable message.
- Keep init configuration-only: no registry downloads or skill installation.

## Affected Areas

- `src/main.rs`, `src/wizard.rs`, and wizard UI/support modules
- `src/linker.rs` (reuse the registry-name validator)
- `Cargo.toml`, `Cargo.lock`, `README.md`
- `Taskfile.yml`, `scripts/test_init_tui.py`
- `workspace/specs/README.md`, `workspace/agents/memory/`

## Release Plan

Release as SKM 0.3.0 through reviewed PRs into `development`, followed by
`main`. Record confirmed integration and publication before lifecycle moves.

## Validation Gates

- `task check`
- `task test`, including draft editing, preservation, save/cancel/conflict,
  validation, terminal rendering, and CLI compatibility tests
- `task build` and `task test:tui` (isolated Linux terminal smoke test)
- `git diff --check`

## Design Decisions

- Use Ratatui for layout, widgets, and test rendering, Crossterm for portable
  keyboard events and terminal lifecycle, and tempfile for atomic file replacement.
  These dependencies avoid implementing terminal protocols and save mechanics.
- Preserve existing YAML bytes on a no-op save. Editing preserves YAML values
  (including extension fields); comments and formatting may be normalized.
- `--global` retains the existing current-directory manifest location and only
  changes the displayed installation scope; toolkit setup remains project-scoped.

## Memory Impact

Status: updated

Rationale: Recorded existing-file editing, preview and persistence guarantees,
compatibility flags, and terminal test entrypoint in
`workspace/agents/memory/facts.md` and `workspace/agents/memory/changelog.md`.

## Validation Result

- `task check`: passed formatting, warnings-free Clippy, compilation, and
  workspace documentation gates.
- `task test`: passed 88 Rust tests and 6 workspace-validator tests. The 22 new
  tests cover draft preservation, invalid input and unsafe targets, conflicts,
  Unicode editing, navigation, scrolling, responsive rendering, and CLI flags.
- `task build`: passed the locked release build.
- `task test:tui`: passed six isolated Linux PTY scenarios covering creation,
  editing, cancellation, unchanged saves, concurrent edits, script mode, stdout
  separation, and terminal restoration.
- `git diff --check`: passed.
- Interactive terminal behavior on macOS and Windows was not exercised locally.

## Integration Evidence

- PR: https://github.com/skills-yaml/skm/pull/7
- Commit: `db7d02f44dda9a97def90633bab6d83d07c5e27d`
- CI: https://github.com/skills-yaml/skm/actions/runs/34410715962 (passed)
