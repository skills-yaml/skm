# Separate Workspace Documentation From The SKM README

## Status

State: `done`

Rationale: PR #83 merged into main at 7a289ab9edcb8799c295041b2fab63a79ba233fa; actual main CI 37454486476 and development CI 37454491771 passed. Qualification 37455327437 passed current-pair updates, actual legacy 0.8.0 updates, and modern-bootstrap rejection on Linux, macOS Intel/ARM, and Windows while production remained held. Release Artifacts 37454486538 then published verified SKM 0.9.0 through the normal eligible-reviewer approval path on 2026-10-06; all nine assets, manifest/source identity, checksums, Linux binary identity, and repeat-update stability were verified.

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

## Version Impact

| Component | Impact | Release | Rationale |
| --- | --- | --- | --- |
| skm | patch | skm-next | Changed published project documentation without altering CLI behavior. |

## Memory Impact

Status: `updated`

Rationale: The user's decision that SKM documentation stays independent of
Workspace is recorded in `workspace/agents/memory/decisions.md` with a matching
`workspace/agents/memory/changelog.md` entry.

## Verified Production Completion

PR #83 merged into main at 7a289ab9edcb8799c295041b2fab63a79ba233fa; actual main CI 37454486476 and development CI 37454491771 passed. Qualification 37455327437 passed current-pair updates, actual legacy 0.8.0 updates, and modern-bootstrap rejection on Linux, macOS Intel/ARM, and Windows while production remained held. Release Artifacts 37454486538 then published verified SKM 0.9.0 through the normal eligible-reviewer approval path on 2026-10-06; all nine assets, manifest/source identity, checksums, Linux binary identity, and repeat-update stability were verified.

Historical reconciliation: PR #60 integrated the documentation separation. The later user-authorized [registry format ownership](../../done/registry/registry-format-ownership.md) superseded Workspace runtime flags/pins and the old README pointer. Current README links generic toolkit/registry documentation, and the former Workspace document remains an ordinary-publisher reference. Original scope/evidence are preserved; retired runtime policy is not claimed as current behavior.

Memory is updated in `workspace/agents/memory/facts.md` and
`workspace/agents/memory/changelog.md`. Applied version 0.9.0 is now released;
no duplicate bump is introduced. Earlier local/pending statements describe
historical snapshots. These completion records receive fresh independent review
and frozen aggregate verification before their record-only main closeout.
