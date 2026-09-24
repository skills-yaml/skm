# Show skill dependencies and namespace collections in search

## Status

State: `test`

Rationale: Commit `4c766e693d58fb399fbb7bf25f1939254eb2a65b` integrated the change into `development`; CI `35924612401` and Release Artifacts `35924612416` passed. PR #42 merged the tested code into `main` at `be2581d98e81c79d0756c1ef36fb4ea584200fb1`, and production CI `35962268392` passed. The production release is held for qualification and reviewer approval.

## Problem

Search prints a skill name and an unlabeled description, omits its declared dependencies, and reports no bundles for the current Workspace schema-1 registry without showing its published skill collection. Users cannot quickly tell what they are viewing or whether a whole group is installable.

## Goals

- Label the skill name, description, version, registry, and exact declared dependencies in human results.
- Include dependencies in JSON results and show them in interactive init search.
- Show namespace collections published by registry manifests, including Workspace's schema-1 package set.
- Continue showing only explicit schema-2 bundles as installable group candidates, with member identities visible.
- Keep registry search read-only, bounded, deterministic, and compatible with existing JSON fields.

## Non-goals

- Installing a namespace collection or bundle; the coordinated bundle-install contract remains in backlog.
- Treating a schema-1 namespace as an installable bundle.
- Changing registry publication or package metadata formats.

## Design

Read bounded `SKILL.md` frontmatter for the selected version and parse the exact `skm-dependencies` string into a display list. Malformed metadata leaves the skill visible without invented dependencies. Read namespace manifest package keys for collections in schema 1 or 2. Read schema-2 bundle membership only when explicitly declared and valid. Label collection and bundle availability in human output. Preserve `members` counts and add package identities to bundle JSON.

## Affected areas

- `src/search.rs`, `src/wizard/prompt.rs`, CLI help, `README.md`, search tests, and the spec catalog.

## Acceptance criteria

- `skm search spec` displays distinctly labeled name and description plus the declared dependencies for matching skills.
- `skm search workspace` shows the Workspace collection from its schema-1 manifest without claiming an installable bundle exists.
- A valid schema-2 fixture shows explicit bundle membership; invalid bundle data remains hidden.
- JSON includes skill dependencies and collections while retaining existing fields.
- Local and remote discovery agree; malformed and control-character metadata cannot corrupt output.
- `task check`, `task test`, and `git diff --check` pass.

## Risks and rollout

Registry data is untrusted. Bound reads, sanitize displayed text, and validate identifiers before reporting membership. This is a display-only change; the existing bundle installation backlog controls future mutation behavior.

## Memory Impact

Status: `updated`

Rationale: Search's labeled skill details and namespace collection distinction are durable discovery behavior. Recorded in `workspace/agents/memory/facts.md` and `workspace/agents/memory/changelog.md`.
