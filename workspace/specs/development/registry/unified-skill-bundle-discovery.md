# Development Spec: Unified Registry Skill and Bundle Discovery

## Status

State: `development`

Rationale: The user requested one search interface and one add command for
published registry skills and manifest bundles. Implementation is active on a
feature branch; integration into `development` has not been confirmed.

## Problem and Goal

`skm search` currently matches only skills, then lists every published bundle
in a separate section regardless of the query. A user must switch from
`skm add` to `skm bundle add` to act on a bundle. This splits one registry
catalog into two workflows.

Search should match skill and bundle identifiers in one ordered result list,
label each kind, and show a working `skm add` command for either. `skm add`
should recognize a published bundle in the selected registry and use the
existing bundle planning and transaction behavior.

## Scope

- Match published schema-2 bundles by identifier alongside skills, with one
  shared result limit, count, and deterministic order. Keep namespace
  collections browse-only.
- Label result kind in text and JSON; show bundle members and the exact
  `skm add` preview and apply syntax.
- Route `skm add <namespace/name>` to skill or bundle behavior using the
  selected configured registry. A matching bundle uses the existing
  rollback-protected project transaction and requires `--yes` for application;
  `--dry-run` and `--json` remain read-only previews.
- Preserve ordinary skill add, local `--path`, and `--global` behavior. Reject
  bundle-only modes for a skill and global or local-path modes for a bundle
  with actionable errors.
- Preserve `skm bundle add` as a compatibility alias. Do not change the
  project configuration schema or how bundle members are pinned.
- If a registry publishes a skill and bundle with the same identifier, fail
  automatic selection with an ambiguity error. `--kind skill|bundle` selects
  either explicitly, and search commands include the matching kind.

## Affected Areas

- `src/search.rs`: combined matching and output, including JSON contract.
- `src/main.rs`: argument parsing and add routing.
- `src/bundle.rs`: reuse existing bundle apply path; no transaction rewrite.
- `README.md`: unified command documentation.
- Search and CLI tests: matching, modes, collisions, and a fixture add flow.

## Compatibility and Risks

The JSON `matches` array gains bundle entries and a `kind` field. Existing
skill fields remain available for skill entries. The top-level `bundles` list
is retained for existing consumers. Bundle routing must use the same selected
registry as the displayed search result and must never treat a browse-only
collection as a bundle. No new runtime dependency or migration is needed.

## Acceptance Criteria

1. A query matching one skill and one bundle shows both in the same sorted,
   limited results in text and JSON, with distinct kind labels and add syntax.
2. An unrelated query does not print all bundles as search results; collections
   remain clearly browse-only.
3. `skm add` of a published bundle previews without changing project files,
   applies exact pins and links with `--yes`, and keeps existing rollback and
   repeated-command behavior.
4. Ordinary registry and local-path skill additions still work. Invalid flag
   combinations and ambiguous automatic selection fail before writes;
   `--kind` can select either item when identifiers collide.
5. `skm bundle add` continues to work.

## Validation Gates

- Deterministic fixture tests for mixed search results and add routing.
- `task check` and `task test` pass.
- Review `git diff` for unrelated changes and preserve local `skills.yaml`.

## Memory Impact

Status: `updated`

Rationale: The user's durable preference for one registry search and add
interface is recorded in `workspace/agents/memory/decisions.md`, with a
matching `workspace/agents/memory/changelog.md` entry. Bundle application
semantics remain covered by the earlier bundle decision.
