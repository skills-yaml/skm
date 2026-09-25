# Confirm Skill and Bundle Adds

## Status

State: `done`

Rationale: PR #56 integrated the implementation into `development`; PR #58 promoted it to `main`, and PR #59 prepared SKM 0.8.0 at `07b4bba9c1d5f3de3964f398ff8296afb617db6e`. Production CI and four-platform update qualification passed. The reviewed `release-prod` workflow published all nine `prod-latest` assets on 2026-09-25; the Linux checksum, release manifest, and binary identity matched the production commit.

## Problem and Users

`skm add` currently requires `--yes` for published bundles but adds an individual skill immediately without confirmation. Interactive users need to see the selected item, resolved dependencies, configuration change, and agent link impact before approving either operation.

## Goals

- Show an accurate plan for a skill or published bundle before any project configuration or agent-link writes.
- Prompt for explicit confirmation when `--yes` is absent, with No as the default. Declining or receiving EOF leaves the project unchanged.
- Let `--yes` apply the same planned change without prompting, including ordinary registry, local-path, and global skill adds.
- Keep bundle `--dry-run` and `--json` as read-only previews and preserve the bundle transaction and compatibility command.
- Reject an unconfirmed mutating add in a non-interactive context with an actionable `--yes` message.

## Non-goals

- Changing bundle membership, skill dependency rules, or the project configuration schema.
- Adding `--dry-run` or `--json` to single-skill adds.
- Changing `skm install`, `skm remove`, search output, or other command prompts.

## Design

For a bundle, reuse its validated plan. Print the bundle, registry, every resolved exact skill pin with member/dependency and add/keep status, and the number of agent links to create or repair. Then ask `Install <count> skills and create or repair <count> links? [y/N]` unless `--yes` is set. The existing atomic bundle apply runs only after a positive answer.

For a single skill, resolve the requested skill and dependency closure before asking. Print the requested `skills.yaml` entry, the resolved skills that will be linked, and the link impact for the chosen project or global scope. Existing configured skills may be checked and linked again, so the plan must describe the actual install path. Apply the existing skill-add operation only after confirmation. The prompt accepts `y` or `yes` case-insensitively; Enter, `n`, and EOF decline. A non-terminal input or output with no `--yes` fails before project writes. The preview paths may populate registry cache, but never write project configuration or agent links.

## Affected Areas

- `src/main.rs`: skill-add routing, plan, and `--yes` handling.
- `src/bundle.rs`: bundle confirmation after planning and before apply.
- A shared CLI confirmation helper and focused tests.
- `README.md`: describe interactive and non-interactive usage.
- `workspace/specs/README.md`: register this specification.

## Compatibility, Safety, and Rollback

Unconfirmed single-skill adds become interactive, so scripts must pass `--yes`. Bundle previews remain read-only, and declined prompts never apply the plan. Preflight failures occur before prompting where possible. Bundle application retains its rollback transaction. Skill addition retains its existing config and linking behavior after confirmation; this change does not claim an atomic migration of that path. No new runtime dependency or configuration migration is needed.

## Acceptance Criteria

1. Running `skm add <bundle>` with a terminal and no `--yes` prints every planned skill and link count, then applies only after an explicit Yes.
2. Running `skm add <skill>` with a terminal and no `--yes` prints the requested config entry, resolved skill/dependency list, scope, and link impact, then applies only after an explicit Yes.
3. No, Enter, EOF, and non-terminal use without `--yes` leave `skills.yaml` and agent links unchanged; non-terminal use reports how to proceed with `--yes`.
4. `--yes` skips prompting for both item kinds. Bundle `--dry-run`, `--json`, and `skm bundle add` preserve their existing preview and application semantics.
5. Invalid flags, collisions, and dependency or target failures still fail before any project writes; existing valid skill and bundle adds continue to work.

## Validation Gates

- Deterministic tests for confirmation acceptance, decline, EOF, non-terminal handling, skill routing, and bundle plan/apply paths.
- `task check`, `task test`, and `git diff --check` pass.
- Preserve the user's existing `skills.yaml` and `.nib/` work outside the change.

## Memory Impact

Status: `updated`

Rationale: The user's approved add-confirmation behavior is recorded in `workspace/agents/memory/decisions.md` with a matching `workspace/agents/memory/changelog.md` entry.
