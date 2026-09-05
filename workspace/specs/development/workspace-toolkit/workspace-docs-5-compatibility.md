# Specification: Workspace Docs 5 Toolkit Compatibility

## Status

State: development

SKM compatibility support is implemented and locally validated on a feature
branch. It remains in development until integration through the repository's
normal branch workflow.

## Problem

SKM 0.2.0 accepts only toolkit manifests that declare
`workspace_docs_compatibility: 4.x`. Workspace toolkit 0.2.0 targets Workspace
Docs 5.x so it cannot be installed even when its package is otherwise valid.

## Scope

- Accept toolkit manifests targeting Workspace Docs 4.x or 5.x.
- Keep rejecting missing or unsupported compatibility declarations.
- Release the compatibility change as SKM 0.2.1.
- Update CLI documentation and examples to use the Workspace Docs 5.x toolkit.
- Document the toolkit's OpenTofu-only and CI/CD-only infrastructure mutation
  boundary without changing SKM's installation semantics.
- Document the toolkit's standing authority for routine delivery and its
  destructive-production-only human approval boundary.

## Acceptance Criteria

- Existing 4.x-compatible toolkit manifests remain valid.
- A valid 5.x-compatible toolkit manifest resolves successfully.
- An unsupported compatibility line fails before writes with a clear message.
- Workspace toolkit 0.2.0 can require SKM 0.2.1 and install normally.
- SKM documentation states that local infrastructure work is non-mutating and
  that OpenTofu mutation runs only through repository CI/CD.
- SKM documentation states that routine delivery proceeds under standing
  authority and that human approval is reserved for destructive production
  actions.
- Required checks, tests, and release build pass.

## Affected Areas

- `src/toolkit.rs`
- `src/main.rs`
- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `workspace/agents/memory/`

## Implementation Plan

1. Extend compatibility validation without weakening other manifest checks.
2. Add success and failure tests for the compatibility declaration.
3. Bump the patch version and update current configuration examples.
4. Run the Taskfile validation, test, and build gates.

## Validation Gates

- `task check`
- `task test`
- `task build`
- `git diff --check`

## Risks

- Accepting arbitrary values would hide unsupported lifecycle contracts. Keep
  the allowlist explicit.
- Removing 4.x support would break toolkit 0.1.0 consumers. Preserve it.

## Validation Result

- `task check`: passed formatting, Clippy, and compilation checks.
- `task test`: passed all 52 tests, including 5.x acceptance and unsupported
  compatibility rejection.
- `task build`: produced the locked release build.
- Workspace toolkit 0.2.0 installed successfully with no lock or output drift.
- SKM documents the toolkit's routine-delivery standing authority and scoped
  destructive-production approval boundary.
- `git diff --check`: passed.

## Memory Impact

Status: `updated`

Rationale: Recorded in workspace/agents/memory/decisions.md and workspace/agents/memory/changelog.md.
