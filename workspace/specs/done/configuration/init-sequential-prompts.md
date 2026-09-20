# Specification: Sequential Init Prompts

## Status

State: done

Released in SKM 0.6.0 through `main` at
`769f5a433c635a56b86e744513dd71d7a42a181c`. This specification supersedes
the full-screen interaction released in
`workspace/specs/done/configuration/init-tui-wizard.md` and adds an explicit,
on-demand interaction over the released standalone registry search support.

## Summary

Replace the full-screen `skm init` terminal UI with a portable, line-by-line
prompt flow that remains safe for new and existing `skills.yaml` files.

## Problem

The full-screen interface requires users to learn modal navigation, occupies
the alternate terminal screen, depends on terminal dimensions and rendering
support, and exposes advanced configuration alongside the common setup path.
Users need a simpler interaction that works as an ordinary terminal dialogue
without sacrificing manifest safety or registry skill discovery.

## Goals

- Guide users through project details, agents, registries, skills, optional
  toolkit/workspace settings, and final review using sequential text prompts.
- Preserve existing manifest values and extension fields while editing.
- Keep registry skill search, manual skill entry, and source/version selection.
- Write only after explicit review and confirmation, with the existing atomic
  save and concurrent-edit protections.
- Keep prompts and status messages on stderr so stdout remains safe for
  machine-readable command output.
- Preserve `--non-interactive`, `--global`, `--advanced`, and existing init
  configuration flags.

## Non-goals

- Retain full-screen panels, live YAML rendering, raw terminal mode, mouse
  input, or terminal-size-dependent layouts.
- Install skills or mutate registry caches during init.
- Redesign the standalone `skm search` command.
- Add a new runtime dependency.

## Proposed Design

### Prompt contract

- Each prompt shows the current value when one exists.
- Enter keeps the current value.
- `-` clears an optional value where clearing is valid.
- `:q` cancels from any prompt without writing the manifest.
- Invalid input explains the expected form and repeats the same prompt.
- Prompts use stdin and stderr in normal cooked-terminal mode; no alternate
  screen, raw mode, cursor control, or minimum terminal size is required.

### Flow

1. Project: prompt for name and optional version.
2. Agents: show supported agents and accept selected numbers or names, `all`,
   `none`, or Enter to keep the current selection.
3. Registries: show configured entries and offer add, edit, or named removal
   actions until the user continues.
4. Skills: show configured entries and offer registry search, refreshed search,
   manual add, edit, or named removal actions. Search results are numbered and
   include registry and available version.
5. Workspace: keep optional toolkit/workspace settings unchanged unless the
   user explicitly chooses to edit them.
6. Review: validate and print the complete YAML, then ask whether to save.

Registry discovery is synchronous and starts only after the user chooses search
or refresh. Loading and source-specific warnings are printed before the next
prompt. Repeated selection preserves same-name source protections.

### Compatibility and safety

- Existing malformed manifests, directories, and symlinks remain rejected.
- No-op saves retain the original bytes.
- Edited saves retain unknown YAML values but may normalize comments and
  formatting.
- A file changed outside the prompt flow is rejected at save time.
- EOF and `:q` are cancellation, not implicit confirmation.
- Non-interactive initialization remains create-only.

## Alternatives Considered

- Keep and simplify the full-screen TUI: rejected because the requested product
  direction is to remove the TUI entirely.
- Make init flags-only: rejected because the requested replacement is a simple
  sequential interaction.
- Restore the legacy wizard unchanged: rejected because it does not safely edit
  existing manifests or preserve the current registry catalog behavior.

## Risks and Tradeoffs

- Sequential editing takes more lines than a full-screen form. Concise section
  menus and keep-current defaults limit repetition.
- On-demand registry discovery can pause at a prompt. Print a loading message,
  retain bounded Git operations, and allow cancellation before starting it.
- Removing the live preview reduces immediate feedback. The final review prints
  the complete YAML and validation errors identify the relevant configuration.

## Affected Areas

- `src/wizard.rs`, `src/wizard/prompt.rs`, `src/search.rs`, and removal
  of `src/wizard/ui.rs`
- `Cargo.toml`, `Cargo.lock`
- `README.md`, `Taskfile.yml`, prompt smoke coverage under `scripts/`
- `workspace/specs/README.md`, related init specifications, and project memory

## Acceptance Criteria

- `skm init` uses only sequential line prompts and never enters raw or alternate
  terminal mode.
- Users can keep or edit project metadata, agents, registries, skills, and
  optional toolkit/workspace settings.
- Users can search or refresh registry skills, select numbered results, and add
  local or registry skills manually.
- Removal prompts identify the registry or skill being removed.
- `:q`, EOF, and a declined final save leave `skills.yaml` unchanged.
- Review prints valid YAML before save; invalid drafts are not written.
- Existing-file, no-op, extension-field, concurrent-edit, and atomic-save
  guarantees remain covered by tests.
- `--non-interactive` remains create-only and works without a terminal.
- Ratatui and Crossterm are removed from runtime dependencies.
- Documentation and Taskfile naming no longer describe init as a TUI.

## Validation Gates

- `task check`
- `task test`
- `task build`
- Sequential prompt smoke task from `Taskfile.yml`
- `git diff --check`

## Rollout

The prompt flow replaces the released TUI implementation directly.
`--advanced` remains a compatibility alias for the same complete prompt flow.
No manifest migration is required.

## Implementation and Validation

Completed locally on 2026-09-19. `skm init` now uses cooked-terminal prompts,
loads registry catalogs only after a search or refresh action, prints the full
YAML before confirmation, and retains the existing atomic-save and
concurrent-edit protections. Ratatui, Crossterm, the renderer state, and the
raw-terminal smoke harness were removed.

- `task check`: formatting, warnings-free Clippy, compilation, and workspace
  documentation gates passed.
- `task test`: 118 Rust tests and 6 documentation validator tests passed.
- `task build`: the locked optimized release binary built successfully.
- `task test:init`: five Linux PTY scenarios passed for sequential save,
  cancellation, on-demand search, concurrent edits, and script mode; the
  harness also rejects terminal control sequences and stdout prompt output.
- `git diff --check`: passed.

- PR #25 merged the implementation into `development` on 2026-09-19 at
  `4e6db2b6b2665cecb337f50d410ac3bdf7d06186` after its CI validation passed.
- PR #26 fixed the Windows-only updater dependency discovered by the release
  matrix and merged at
  `b25954d74445e51c797de102054c4f744a0ed1f7`.
- CI run `35471263033` passed on that development commit.
- Release Artifacts run `35471263031` built and published all four supported
  platform archives, checksums, and `skm-release.json` to
  `development-latest`.
- The downloaded Linux archive passed its published SHA-256 check and reported
  `skm 0.6.0 (development - b25954d74445e51c797de102054c4f744a0ed1f7)`;
  the manifest reported the same version, channel, and commit.

PR #32 promoted the exact qualified development commit to `main`. Production
CI run `35496461707` passed, qualification run `35496742318` exercised
notification, replacement identity, and the idempotent no-op path on Linux,
macOS Intel, macOS Apple Silicon, and Windows while publication remained held,
and Release Artifacts run `35496461689` published after reviewer approval. The
downloaded Linux archive passed its SHA-256 check and reported
`skm 0.6.0 (prod - 769f5a433c635a56b86e744513dd71d7a42a181c)`;
`skm-release.json` reported the same version, channel, and commit with all four
supported platform assets. The specification is complete.

## Open Questions

None. The user selected sequential prompts as the replacement interaction.

## Memory Impact

Status: updated

Rationale: Recorded the user-selected sequential init interaction in
`workspace/agents/memory/decisions.md`, recorded the on-demand registry
interaction in `workspace/agents/memory/facts.md`, and appended a corresponding
record to `workspace/agents/memory/changelog.md`.
