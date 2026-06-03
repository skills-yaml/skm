# Project Guidelines (skills-yaml / skm)

These instructions apply to AI coding agents and contributors working in this repository.

## Core Goal

Ship focused, safe, and test-backed CLI improvements in Rust that follow clean code design patterns and pass all quality checks.

## Authoritative References (Read Before Editing)

- [Task](./docs/tech/task.md): task runner rules and task patterns.
- [SDLC](./docs/tech/sdlc.md): branch strategy, PR process, code quality.
- [CI](./docs/tech/ci.md): pipeline configuration and check execution.
- [Project Structure](./docs/tech/project_structure.md): repository layout and file ownership.
- [CLI Design Guide](./docs/specs/foundation/ui_brand_and_style.md): styling and formatting of CLI output.

## Architecture Boundaries

- `src/main.rs`: Command line parsing, subcommands mapping, execution orchestration.
- `src/config.rs`: YAML serialization, deserialization, default settings schema for `skills.yaml`.
- `src/linker.rs`: Path resolving and symlinking logic to target agent directories.
- `tests/`: Integration tests.

Respect boundaries. Keep modules modular and avoid mixing concerns.

## Build and Test Commands

Use Taskfile entrypoints only.

- Root validation: `task check` (enforces formatting, warnings-free Clippy build, and compilation check)
- Root tests: `task test`
- Build command: `task build`

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
- Do not check in active personal configurations/links.

## Testing Rules

- Add or update tests for every logic change (e.g. config parsing validation, linking logic).
- Mock directories or use temporary directories (`std::env::temp_dir()`) to test file system and linking actions to prevent breaking real user directories.
- Keep tests deterministic.

## Must / Must Not

- MUST keep changes focused and minimal.
- MUST follow established Rust coding styles.
- MUST NOT introduce new runtime dependencies to `Cargo.toml` without clear explanation/need.
- MUST NOT bypass git VCS rules.
