# Hide Workspace Pin Prompts in Init

## Status

State: `development`

Rationale: The user confirmed on 2026-09-26 that interactive `skm init` should stop asking for Workspace standard, source, revision, and integrity while retaining toolkit, bundle, and profile prompts. Implementation is active; no integration into `development` is confirmed.

## Problem and Users

The optional settings step of `skm init` still asks general skill-manager users to edit four Workspace-specific pins. Those values belong to the separate toolkit configuration path. Users need a focused interactive flow without losing existing manifest data.

## Goals

- Remove the four Workspace pin prompts from the interactive `init` flow.
- Keep toolkit manifest/version, bundles, and profiles editable in the optional step.
- Preserve existing `workspace` values in the review and saved manifest.
- Keep explicit `skm init --workspace-*` flags and the manifest schema available for toolkit users.

## Non-goals

- Removing Workspace fields from `skills.yaml`, toolkit installation, or lockfiles.
- Changing the explicit Workspace CLI flags, non-interactive initialization, or other wizard steps.
- Migrating or clearing existing manifests.

## Design

Name the final optional section for toolkit settings. Its configured indicator reflects toolkit, bundle, and profile values. After a user chooses to edit, prompt for toolkit manifest/version, bundles, and profiles, then proceed directly to validation and full YAML review. Do not read or write the `workspace` subtree in that prompt step. Loading, editing, and saving an existing manifest continue to retain its Workspace pins and extension fields.

## Affected Areas

- `src/wizard/prompt.rs`: optional step presentation and focused regression tests.
- `README.md`: describe the interactive steps accurately and point toolkit users to explicit Workspace flags.
- `workspace/specs/README.md`: register this development spec.

## Compatibility and Risks

Existing manifests and scripts using `--workspace-*` continue to work. Interactive users can no longer create or change Workspace pins in the wizard; they can use the explicit flags or edit `skills.yaml`. The full review still displays preserved pins. A prompt-count change can affect scripted input that drives the interactive wizard, so test the revised sequence. No runtime dependency or data migration is needed.

## Acceptance Criteria

1. Interactive `skm init` shows toolkit, bundle, and profile prompts after choosing to edit optional settings, with no Workspace standard, source, revision, or integrity prompts.
2. The optional section's title and configured indicator refer only to toolkit selections.
3. Existing Workspace pins and extension fields survive an interactive edit and save, and remain visible in the complete YAML review.
4. Explicit `--workspace-*` flags and non-interactive initialization retain their behavior.

## Validation Gates

- Deterministic wizard tests cover the displayed prompt sequence and preservation of existing Workspace values.
- Existing CLI flag and draft-preservation tests pass.
- `task check`, `task test`, and `git diff --check` pass.
- The user's current checkout, `skills.yaml`, and `.nib/` remain untouched.

## Memory Impact

Status: updated

Rationale: The narrower interactive boundary is recorded in `workspace/agents/memory/decisions.md` with a matching `workspace/agents/memory/changelog.md` entry.
