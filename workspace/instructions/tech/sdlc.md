# Software Development Lifecycle

## Development Workflow

The project follows a structured development workflow:

1. **Feature Specification**: Large feature additions must have specifications under `workspace/specs/<state>/<primary-feature>/` and a matching row in `workspace/specs/README.md`.
2. **Branch Strategy**:
   - `main`: Stable, production-ready code. Confirmed production release target for `done` specs.
   - `development`: Shared prerelease/test channel. Confirmed integration target for `test` specs.
   - Feature branches, when directly requested for the named task: `feat/<name>` or `feature/<name>`.
   - Bugfix branches, when directly requested for the named task: `fix/<name>` or `bugfix/<name>`.
3. **Pull Request Process**:
   - All changes must go through PR review.
   - CI checks must pass before merging.

## Workspace Docs 6 Coordination

When two or more agents may mutate this repository, each peer uses the
repository's `coordinate-multi-agent-development` skill and its portable
runtime to claim a task and create a dedicated detached linked worktree from
one reviewed base. The primary checkout remains coordination-only. Each peer
maintains a task index and its own WIP record under
`workspace/docs/work/multi-agent/<task-id>/`; shared file scopes are allowed
and conflicts are resolved during reviewed integration. A task or delivery
branch requires a direct user request for that named task. Existing linked
worktrees are preserved during adoption and are not automatically claimed.

The peer board, lock, managed worktrees, and recovery archives are local to the
Git common directory. They are not committed, copied into work records, or
treated as evidence of `development` integration or production release.

## Version Planning

Every non-legacy spec declares its per-component Version Impact before
implementation. `workspace/releases.json` records one logical owner and one
open target per component. SKM is pre-1.0: incompatible changes and compatible
additions use a minor bump; compatible fixes use a patch bump. An explicit
stable-contract decision would use a major bump to 1.0.0. Peers check the
shared reservation and published state before choosing a target, reuse an
already-applied shared release, and run `task versions:check` at handoff. The
project's native gate reads SKM's version from `[package].version` in
`Cargo.toml` and checks its `Cargo.lock` mirror.

Local implementation stays in `development` until confirmed integration into
the shared `development` test target. A version reservation or peer handoff is
not lifecycle evidence.

## CI/CD Pipeline

The CI/CD pipeline triggers on pull requests and merges to `main`. It follows the standard Rust validation flow:

1. **Check Phase**:
   - Run `task check` to verify formatting, Clippy, compilation, workspace
     structure, release reservations, and peer records.

2. **Test & Build Phase**:
   - Run `task test` to execute all cargo tests.
   - Run `task build` to build optimized release binaries.

## Code Quality Standards

- **Formatting**: Consistent formatting enforced by `cargo fmt`.
- **Linting**: No warnings or errors allowed by `cargo clippy` (enforced via `-D warnings` in checking).
- **Testing**: Unit and integration tests required under `tests/` or inline modules for logic changes.
- **Filesystem safety**: Path validation and non-destructive symlink behavior must be covered by tests when changed.

## Commits

- All commits must pass `task check` and `task test` before being pushed.
- All commit messages must follow the conventional commit format: `type(scope): description`

Examples:
- `feat(cli): add list command`
- `fix(linker): handle broken symlinks correctly`
- `chore(deps): bump clap to 4.4`
