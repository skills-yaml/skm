# Separate Workspace Documentation From The SKM README

## Status

State: `development`

Rationale: The user asked on 2026-09-25 to separate `skills.yaml` and SKM from
Workspace in documentation and the website only. Implementation is active on
`docs/separate-workspace-docs`; no code change is in scope.

## Problem and Users

The README presents SKM as a tool "for managing AI agent skills and Workspace
development toolkits". Workspace toolkit configuration, `wk-adopt` adoption
guidance, Workspace-only `init` flags, and the behavior of installed Workspace
workflows (OpenTofu policy, delivery authority) are mixed into the Quick Start,
Commands, Configuration, and Safety sections. A user who only wants to manage
skills from a registry reads Workspace material throughout and may conclude
SKM requires Workspace.

## Goals

- Present SKM and `skills.yaml` as a general skill manager that works with any
  registry.
- Move every Workspace-specific passage into one dedicated document,
  `workspace/docs/workspace-toolkit.md`, linked once from the README.
- Use neutral registry examples in the README instead of `workspace/*`
  packages.

## Non-goals

- Changing any SKM behavior, flag, manifest field, or lockfile content.
- Removing the toolkit or workspace code paths.
- Changing the repository's own workspace-docs governance or the Development
  section's description of its gates.

## Design

Rewrite the README introduction without the toolkit sentence. In Quick Start,
replace `workspace/wk-spec` and `workspace/all-workspace-skills` examples with
`software-development/spec` and `skills-yaml/authoring-toolkit`, and move the
`wk-adopt` guidance, the legacy nested-link note, and the toolkit install
preview to the new document. In Commands, list only general `init` flags and
point to the new document for toolkit flags; describe `install` without toolkit
bundles. In Configuration, replace the `workspace/*` dependency example with
neutral names and move "Workspace toolkit configuration" out. In Safety, keep
general guarantees and move toolkit integrity and installed Workspace workflow
policy out. The new document carries all moved content unchanged in meaning.

## Affected Areas

- `README.md`
- `workspace/docs/workspace-toolkit.md` (new)
- `workspace/docs/README.md`
- `workspace/specs/README.md`
- `workspace/agents/memory/decisions.md`, `workspace/agents/memory/changelog.md`

## Compatibility and Risks

Documentation only. External links to removed README anchors
(`#workspace-toolkit-configuration`) break; the README keeps a short pointer
section so readers still find the new document. No manifest, CLI, or lockfile
change.

## Acceptance Criteria

1. The README describes SKM without Workspace except one pointer section that
   links to `workspace/docs/workspace-toolkit.md`.
2. Every moved statement (toolkit configuration, toolkit `init` flags, install
   preview, `wk-adopt` adoption, legacy link cleanup, toolkit integrity, and
   installed Workspace workflow policy) appears in the new document.
3. README examples use no `workspace/*` package.
4. The `init` synopsis in the README still matches `skm init --help` for the
   general flags, and the new document lists the toolkit flags.

## Validation Gates

- `task check`, `task test`, and `git diff --check` pass.
- `grep -in workspace README.md` returns only the pointer section and the
  repository's own Development gates.

## Memory Impact

Status: `updated`

Rationale: The user's decision that SKM documentation stays independent of
Workspace is recorded in `workspace/agents/memory/decisions.md` with a matching
`workspace/agents/memory/changelog.md` entry.
