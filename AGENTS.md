# Project Guidelines (skm)

<!-- AGENT-CONTEXT:START workspace-docs@5.0.0 -->
## Workspace Documentation Standard

This project follows `workspace-docs@5.0.0`.

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

Every non-legacy spec must be grouped by its single primary feature:

```text
workspace/specs/<state>/<primary-feature>/<spec>.md
```

The root `workspace/specs/README.md` defines feature categories and is the
canonical status catalog. Update its link, feature, state, and status rationale
whenever a spec is created, moves state, changes feature, is reopened, or
otherwise changes why it is in its state.

Allowed transitions:

- `backlog -> development`
- `development -> test`
- `test -> done`

`development` means implementation is active. Move to `test` only after the
implementation is integrated into the configured test branch or environment,
conventionally `develop`. Move to `done` only after release through the
configured production branch or environment, conventionally `main`. Branch
names are defaults; documented repository-local equivalents are allowed.

Do not infer a transition from the checked-out branch name alone. Record the
confirmed integration or release event in the spec and catalog rationale.

Do not start implementation until the development spec has scope, acceptance
criteria, affected areas, validation gates, and a `Memory Impact` section with
`Status: pending`.

### Documentation & Instruction Boundaries

- **Root Policy & Tokens**: `AGENTS.md` and `DESIGN.md` remain at the root.
- **System & Agent Instructions (`workspace/instructions/`)**: Static policies,
  architecture rules, standards, agent prompts, and skills. Do not modify them
  unless an authorized migration spec requires it.
- **Development Specs (`workspace/specs/`)**: Lifecycle-managed specs in
  `backlog`, `development`, `test`, `done`, or preserved `legacy` state.
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
