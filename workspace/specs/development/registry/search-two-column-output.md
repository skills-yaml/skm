# Two-Column Search Results

## Status

State: `development`

Rationale: The user approved the example layout and requested implementation. Work has not been integrated into the configured `development` test channel as a reviewed change.

## Problem and Users

The human-readable `skm search` result prints one labeled field per line and includes add and preview commands. This makes a mixed list of skills and bundles hard to scan. CLI users need to compare names and descriptions quickly while still seeing which registry provides each result.

## Goals

- Show human-readable matches in two aligned columns: `NAME (TYPE)` and `DESCRIPTION`.
- Put a skill version immediately after its name, show `skill` or `bundle` beside each name, and put the registry on a second line in the description column.
- Show a skill's description when present and summarize a bundle from its declared members. Retain dependency details without overwhelming the first line.
- Remove add and preview instructions from human-readable search results.

## Non-goals

- Changing search, add, or bundle resolution behavior.
- Changing the `--json` response or removing its command fields.
- Adding a description field to the registry bundle schema.

## Design

Render the existing sorted, limited search matches as two text columns. The first column contains `<name>@<version> (skill)` or `<name> (bundle)`. The second column starts with the skill description or `Includes <count> skills: <members>` for a bundle, followed by `Registry: <source>` on the next line. When a skill has dependencies, list them beneath the registry. For a skill without a description, use a clear unavailable message. Size the first column to fit the visible names so no identifier is truncated. Preserve result counts, no-match messaging, warnings, and browse-only collections.

## Affected Areas

- `src/search.rs`: human-readable formatting and focused output tests.
- `README.md`: update the search presentation description and show an example.
- `workspace/specs/README.md`: register this development spec.

## Compatibility and Risks

This changes only human-readable output, so scripts parsing that text may need to adapt. JSON remains the stable machine-readable interface. Long names or descriptions may cause wide lines in narrow terminals; no data is silently truncated. No project configuration or migration changes are required. Search remains read-only.

## Acceptance Criteria

1. Mixed skill and bundle results appear under two column headers with aligned description text.
2. Skill versions follow skill names directly, kind labels distinguish skills and bundles, and each registry appears in the description column's second line.
3. Bundle member summaries, available skill descriptions, missing-description messages, and skill dependencies are readable without add or preview command lines.
4. Ordering, limit/count handling, warnings, collections, and JSON response shape remain unchanged.

## Validation Gates

- Focused deterministic renderer tests for mixed matches and missing descriptions.
- `task check`, `task test`, and `git diff --check` pass.
- Review the diff for unrelated changes and preserve the existing local `skills.yaml` modification.

## Memory Impact

Status: `updated`

Rationale: The user's approved presentation preference is recorded in `workspace/agents/memory/preferences.md` and the matching `workspace/agents/memory/changelog.md` entry.
