---
name: adopt-workspace-structure
description: Adopt, upgrade, repair, or assess the workspace-docs repository structure and its agent governance. Use when an agent must initialize workspace documentation in a project, migrate workspace-docs 1.x or 2.0 layouts to a newer version, update an existing pinned version, repair a partial or mixed adoption, check conformance and implement requested fixes, or complete and validate an in-progress workspace migration.
---

# Adopt Workspace Structure

Manage workspace adoption as a project-local, spec-driven migration. Preserve
existing policy and project behavior while bringing documentation, instructions,
specs, and durable memory into the selected `workspace-docs` contract.

For repository-controlled conflicts, use the applicable current source under
`workspace/instructions/` as the source of truth. Historical packages apply
only when pinned; report unresolved conflicts between current instruction files.

## Establish Authority and Scope

1. Determine whether the user requested assessment only or authorized changes.
   Keep assessment requests read-only; do not infer permission to migrate.
2. Treat a direct human request to adopt, upgrade, repair, or migrate the
   workspace as prospective instruction-change approval only for the governed
   instructions necessary to that named target. A general implementation or
   delivery request is not approval. Do not edit governed instructions first
   and request approval afterward.
3. Resolve the current repository root and operate only inside it. Do not
   inspect parent directories, sibling repositories, or machine-wide state.
4. Read the repository's existing `AGENTS.md`, `DESIGN.md`, `README.md`, task
   and SDLC guidance, `workspace/specs/README.md`, relevant specs, and durable
   memory before editing. Record required files that do not exist as adoption
   gaps.
5. Inspect the branch, status, and current tree. Preserve unrelated and
   pre-existing user changes.
6. Identify the current pinned `workspace-docs@X.Y.Z` version, if any, and the
   target version requested by the user.

If no target is specified, use the concrete version referenced by the selected
standard package's `default` pointer. Never silently select a downgrade.

## Resolve the Trusted Standard

Use a standard package from this order:

1. The current repository's `workspace/instructions/standards/workspace-docs/`.
2. A source explicitly provided by the user.
3. The documented canonical `skills-yaml/workspace` repository when network
   access and the requested operation authorize retrieval.

Pin a concrete version before copying or editing. Never discover a standard by
searching unrelated local projects. If no trusted package is available, stop
and report the required input.

Read these resources completely from the selected package:

- `AGENT_MIGRATION.md`
- `<target-version>/manifest.yaml`
- `<target-version>/agents-template.md`
- `<target-version>/audit-checklist.md`
- `<target-version>/specs-readme-template.md` when the target contract provides
  it
- `<target-version>/migration.md` for an update or repair

Read intermediate migration notes when crossing more than one released
version. Treat the target manifest as the structural source of truth.

## Classify the Work

Choose one mode and record it:

- **Fresh adoption**: no workspace documentation structure exists.
- **Version update**: an older version is pinned or an older canonical layout
  exists.
- **Repair**: the target version is declared but the structure is incomplete,
  duplicated, or inconsistent.
- **Assessment**: inspect conformance and report findings without changing
  files.

Treat mixed layouts as an update. Do not scaffold a second copy alongside
existing content.

## Create the Migration Contract

Before non-trivial changes, create or move a migration spec into
`workspace/specs/development/<primary-feature>/` according to local policy.
Use `workspace-governance` unless the repository defines a more accurate
primary feature. Add or update the spec's row in `workspace/specs/README.md` in
the same change, including its state and the reason work is active. Include:

- current and target versions;
- migration mode and scope;
- acceptance criteria;
- an explicit source, destination, and merge decision for every moved area;
- affected files and product/runtime boundaries;
- validation gates;
- collision, privacy, and rollback risks.
- a `Memory Impact` section initialized to `Status: pending`.

For a fresh adoption where the spec directory does not exist, create only the
minimum directory chain, root specs README from the target template, feature
directory, catalog row, and development spec first. Make no other migration
change until the contract is complete. If existing project policy uses another
spec system, follow it and document the target workspace spec transition.

## Apply the Selected Mode

### Fresh Adoption

1. Preserve or create the root governance files required by the manifest.
2. Create the required `workspace/` directories without claiming optional
   project content exists.
3. Add the pinned standard package and only the technical instructions
   applicable to the project.
4. Insert the generated context from `agents-template.md` between one balanced
   pair of `AGENT-CONTEXT` markers.
5. Keep all manual `AGENTS.md` rules outside the generated block unchanged.
6. Initialize durable memory with stable current-project information only.
7. Add real Taskfile validation and test entrypoints; never add no-op gates.
8. Add the target version's memory-impact completion gate. Record the adoption
   decision in the appropriate category file and `changelog.md`.
9. Add the target version's spec-catalog gate and categorize any current specs
   by their primary feature without changing their lifecycle state.

### Version Update

1. Apply each required version boundary in order.
2. Create destinations before moving content.
3. Execute the spec's explicit migration map one area at a time.
4. Stop on an occupied destination or ambiguous duplicate. Inspect and merge
   deliberately; never overwrite or suppress the failure.
5. Replace only the generated `AGENT-CONTEXT` block with the target template.
6. Update repository-relative links, Taskfile paths, tooling references, and CI
   references affected by file moves without changing unrelated behavior.
7. Reconcile durable memory by preserving stable facts, superseding obsolete
   entries, and removing unsafe or unrelated local context.
8. Add or update the deterministic memory-impact gate required by the target
   standard version.
9. Categorize current specs, create or update the root specs catalog, and add
   the deterministic spec-catalog gate required by the target version.
10. Remove empty legacy paths only after verifying that no current-project
   content was lost.

### Repair

1. Diff the repository structure against the target manifest and audit
   checklist.
2. Repair only confirmed gaps, duplicates, stale markers, or broken links.
3. Preserve compliant content and avoid broad regeneration.
4. Record why the declared target and actual structure diverged.

### Assessment

Report findings by severity with repository-relative evidence, the detected
mode/version, required migrations, validation gaps, and a proposed explicit
map. Do not create a spec or change files unless the user also requested fixes.

## Enforce Migration Boundaries

- Keep all project-specific information limited to the current repository.
- Use repository-relative paths in files, commits, pull requests, issues, and
  handoffs.
- Never migrate secrets, credentials, environment values, caches, dependency
  trees, build artifacts, machine identities, absolute local paths, or
  cross-project inventories.
- Do not change application behavior, authentication, authorization, data
  flow, infrastructure, or CI/CD behavior unless explicitly in scope.
- Do not delete ambiguous content. Stop and request direction when ownership or
  merge precedence cannot be established safely.
- A requested migration implementation has standing workflow authority for
  commits, non-protected branch pushes, pull-request updates, bounded CI work,
  and non-destructive delivery. Do not request repeated approval for those
  steps or bypass repository protection. Pause for prospective human approval
  when a governed instruction is outside the directly approved migration scope,
  and for scoped approval before a destructive production action.

## Validate and Complete

1. Compare the final structure with the target manifest and audit checklist.
2. Confirm instructions, specs, docs, and memory each have one active canonical
   location.
3. Confirm one balanced generated context block pins the concrete target
   version and all manual policy remains intact.
4. Search the current repository for obsolete version markers, legacy paths,
   machine-local information, and unrelated-project context.
5. Validate current-document relative links.
6. Run Taskfile gates, normally `task check` and `task test`, plus every
   project-specific gate required by affected areas. Do not bypass Taskfile.
7. Review the full diff for unintended changes and verify the worktree contains
   only the migration scope.
8. Confirm every in-scope spec has a valid memory-impact section and the
   Taskfile gate rejects missing, unresolved, or incomplete classifications.
9. Confirm every non-legacy spec has one valid primary-feature path and one
   matching root catalog row with its current state and rationale.
10. Classify the migration itself as `updated` or `none`. An adoption or version
   change normally requires the durable decision and changelog to be updated.
11. Record exact results, the resolved memory impact, and justified deviations
   in the development spec.
12. Keep the spec in development after local migration work. Move it to
   `workspace/specs/test/<primary-feature>/` only after confirmed integration
   into the configured test target, conventionally `develop`, and update the
   root catalog with that evidence.
13. Move the spec from test to `workspace/specs/done/<primary-feature>/` only
   after confirmed production release, conventionally through `main`, and
   update the catalog with the release evidence. Do not infer either event from
   the checked-out branch name alone.
14. Record the adopted version and durable project decisions in agent memory.

Do not declare completion while required checks fail, collisions remain, or the
spec state disagrees with the implementation.

## Hand Off

Lead with the outcome and include:

- migration mode;
- previous and target versions;
- significant repository-relative moves or merges;
- confirmation that manual `AGENTS.md` policy was preserved;
- validation commands and results;
- memory-impact classification and rationale;
- deviations, blockers, or follow-up work;
- publication and lifecycle status.

Never include machine-local paths, identities, sibling-project details,
credentials, or environment-specific values in the handoff.
