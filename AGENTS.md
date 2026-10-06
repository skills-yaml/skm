# Project Guidelines (skm)

<!-- AGENT-CONTEXT:START workspace-docs@7.0.0 -->
## Workspace Documentation Standard

This project follows `workspace-docs@7.0.0`.

### Required Reading

- `AGENTS.md`
- `DESIGN.md`
- `README.md`
- `workspace/instructions/tech/task.md`
- `workspace/instructions/tech/sdlc.md`
- `workspace/instructions/tech/project_structure.md`
- `workspace/specs/README.md`
- `workspace/instructions/standards/workspace-docs/AGENT_MIGRATION.md` for
  workspace adoption, updates, or structural repair
- Relevant conditional docs under `workspace/instructions/tech/`
- Relevant specs under `workspace/specs/`
- Project memory under `workspace/agents/memory/` when present

### Spec-Driven Development

Every non-trivial change must be tied to a spec in exactly one state:

- `workspace/specs/backlog/`
- `workspace/specs/development/`
- `workspace/specs/test/`
- `workspace/specs/done/`
- `workspace/specs/blocked/`

Every non-legacy spec must be grouped by its single primary feature:

```text
workspace/specs/<state>/<primary-feature>/<spec>.md
```

The root `workspace/specs/README.md` defines feature categories and is the
canonical status catalog. Update its link, feature, state, and status rationale
whenever a spec is created, moves state, changes feature, resumes from blocked, or
otherwise changes why it is in its state.

Allowed transitions:

- `backlog -> development`
- `development -> test`
- `test -> done`
- Documented direct routes permit `development -> done` with the same gates.
- Backlog, development, and test may enter blocked and resume at their recorded stage.

`development` means implementation is active. Move to `test` only after the
implementation is integrated into the configured test branch or environment,
conventionally `develop`. Move to `done` only after verified merge into `main` and reconciliation of
acceptance, documentation, catalog, versions, and memory. Publication is tracked
separately. Blocked specs record Previous State, Block Kind, Block Reason, and
Resume Condition; renew stale evidence on resumption. Branch
names are defaults; documented repository-local equivalents are allowed.

Do not infer a transition from the checked-out branch name alone. Record the
confirmed integration or main merge event in the spec and catalog rationale.

Do not start implementation until the development spec has scope, acceptance
criteria, affected areas, validation gates, and a `Memory Impact` section with
`Status: pending`.

### Review and Modular Quality Gates

Small low-risk changes with focused tests may use author self-review; other
non-trivial changes require independent agent or human exact-candidate review.
Security, data integrity, public interfaces, governed instructions, and release
controls always require independent review. Resolve every finding through a
fix or reviewer-agreed documented rejection before final verification.

Design gate modules from repository inputs, dependencies, consumers, Taskfile
commands, resources, pass conditions, and freshness rules. Run affected modules
during coherent changes and review fixes; unknown scope uses aggregate fallback.
After review, fixes, generated artifacts, and documentation stabilize, freeze
the candidate and run task check, task test, and applicable project gates.
Reuse only proven-fresh evidence; verify actual combined integration/main
revisions. Each acceptance criterion has supporting evidence. Subjective
product decisions go to the user. Repair failed gates safely; never bypass them.

The task request authorizes ordinary safe delivery, including task branches,
integration, main merge, and non-destructive release. Governed instructions
and destructive production actions retain prospective scoped human approval.
Preserve done history; create linked follow-up specs for later changes.

### Spec Versioning

Every non-legacy spec must include a `Version Impact` table with Component,
Impact (`major`, `minor`, `patch`, or justified `none`), Release, and Rationale.
Follow the pinned standard's `versioning.md` and `workspace/releases.json`.
Reserve a target before implementation; one owner applies its bump once at
`development-start` (default) or `merge`, before integration artifacts are built.
Each agent checks the latest shared reservations before choosing a version and
again at handoff. Reuse an already-applied shared release; reconcile an occupied
independent target through the shared reservation transaction. Run `task versions:check`.

### Multi-Agent Development

No coordinator agent is required. On one machine sharing a Git common directory,
peers use the installed coordination skill's bundled runtime to discover and
claim unique tasks and create detached linked worktrees. A Task wrapper is
optional. Keep the primary checkout coordination-only during concurrent edits.
Ordinary task branches are authorized by the task request, subject to protections.
Read-only agents need no worktree until they mutate repository files.

All peers may edit the same file in separate worktrees. Declare bounded scopes
and maintain your own record under
`workspace/docs/work/multi-agent/<task-id>/<agent-id>.md`; the joining peer owns
the task index. Use the atomic JSON board in the Git common directory for live
progress and version reservations. Independent peers review exact revisions;
any peer may land approved work through serialized integration and explicitly
configured native validation gates.

Conflicts must be resolved before integration succeeds. Preserve interrupted
work, obtain fresh review for revised handoffs, and never force-remove dirty
or uncaptured worktrees. No automatic timeout steals ownership. Keep secrets,
absolute local paths, machine identities, and raw logs out of tracked records.
See the coordination skill for commands, recovery, and the local-only boundary.

### Documentation & Instruction Boundaries

- **Root Policy & Tokens**: `AGENTS.md` and `DESIGN.md` remain at the root.
- **System & Agent Instructions (`workspace/instructions/`)**: Static policies,
  architecture rules, standards, agent prompts, and skills. Do not modify them
  unless an authorized migration spec requires it.
- **Development Specs (`workspace/specs/`)**: Lifecycle-managed specs in
  `backlog`, `development`, `test`, `blocked`, `done`, or preserved `legacy` state.
- **Human & Project Documentation (`workspace/docs/`)**: Reference,
  architecture, and session work documentation.
- **Company Context (`workspace/company/`)**: Business, brand, design, domain,
  and strategy reference material.
- **Agent Memory (`workspace/agents/memory/`)**: Durable decisions, facts,
  preferences, open questions, and changelog.

Generated context stays between `AGENT-CONTEXT` markers. Manual project rules
stay outside generated blocks.

### Agent Memory

Classify every completed user-directed task or bounded work item before final
handoff. Internal commands and tool calls are not separate memory actions.

- Use `updated` for a new or changed durable decision, stable non-obvious fact,
  recurring preference, or consequential open question.
- Use `none` when the task creates no new durable context, with a rationale.
- For `updated`, append the durable entry to its category file and append a
  corresponding record to `changelog.md`.
- Development and test specs may retain `pending` only while the durable result
  is genuinely unresolved. Resolve to `updated` or `none` before `done`.
- State the classification and rationale in every final handoff.

Never store secrets, credentials, personal data, or transient scratch notes in
memory.
<!-- AGENT-CONTEXT:END -->

These instructions apply to AI coding agents and contributors working in this repository.

## Core Goal

Ship focused, safe, and test-backed CLI improvements in Rust that follow clean code design patterns and pass all quality checks.

## Authoritative References (Read Before Editing)

- [Task](./workspace/instructions/tech/task.md): task runner rules and task patterns.
- [SDLC](./workspace/instructions/tech/sdlc.md): branch strategy, PR process, code quality.
- [CI](./workspace/instructions/tech/ci.md): pipeline configuration and check execution.
- [Project Structure](./workspace/instructions/tech/project_structure.md): repository layout and file ownership.
- [CLI Design Guide](./DESIGN.md): styling and formatting of CLI output.

## Architecture Boundaries

- `src/main.rs`: Command line parsing, subcommands mapping, execution orchestration.
- `src/config.rs`: YAML serialization, deserialization, default settings schema for `skills.yaml`.
- `src/linker.rs`: Path validation, registry resolving, symlink validation, and linking logic.
- `workspace/`: Product, process, CI, project structure, specs, and memory documentation.
- `Taskfile.yml`: Local validation, formatting, testing, and build entrypoints.

Respect boundaries. Keep modules modular and avoid mixing concerns.

## Build and Test Commands

Use Taskfile entrypoints only.

- Root validation: `task check` (enforces formatting, warnings-free Clippy build, compilation check, and workspace-docs gates)
- Root tests: `task test`
- Build command: `task build`
- Auto-format and apply available clippy fixes: `task fix`

## Workflow

1. Clarify blockers before coding. Do not assume missing criteria.
2. Plan impacted files, tests, and linkage implications.
3. Implement incrementally with clean, focused commits.
4. Self-review for warnings, formatting, and edge cases.
5. Run verification gates (`task check`, `task test`) before marking task complete.

## Conventions That Matter

- Taskfiles are mandatory interfaces for local and CI operations. Do not bypass with direct cargo calls.
- Rust code must format correctly under `cargo fmt`.
- Clippy checks must pass without warnings (`-D warnings` is enforced).
- Command line arguments must be documented clearly in `Clap` derive parameters to auto-generate help.
- Skill names and registry names must be validated before filesystem operations.
- Linking must not overwrite real files or directories; replacing existing symlinks is allowed.
- `skm check` must verify symlinks point to the expected skill source.
- Do not check in active personal configurations/links.

## Testing Rules

- Add or update tests for every logic change (e.g. config parsing validation, path validation, linking logic).
- Use temporary directories (`std::env::temp_dir()`) to test file system and linking actions to prevent touching real user directories.
- Keep tests deterministic.

## Must / Must Not

- MUST keep changes focused and minimal.
- MUST follow established Rust coding styles.
- MUST NOT introduce new runtime dependencies to `Cargo.toml` without clear explanation/need.
- MUST NOT bypass git VCS rules.
