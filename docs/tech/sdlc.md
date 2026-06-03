# Software Development Lifecycle

## Development Workflow

The project follows a structured development workflow:

1. **Feature Specification**: Large feature additions must have specifications in `docs/specs/feature`
2. **Branch Strategy**:
   - `main`: Stable, production-ready code.
   - Feature branches: `feat/<name>` or `feature/<name>`
   - Bugfix branches: `fix/<name>` or `bugfix/<name>`
3. **Pull Request Process**:
   - All changes must go through PR review.
   - CI checks must pass before merging.

## CI/CD Pipeline

The CI/CD pipeline triggers on pull requests and merges to `main`. It follows the standard Rust validation flow:

1. **Check Phase**:
   - Run `task check` to verify formatting (`cargo fmt`), clippy warnings/errors (`cargo clippy`), and compiler checks (`cargo check`).

2. **Test & Build Phase**:
   - Run `task test` to execute all cargo tests.
   - Run `task build` to build optimized release binaries.

## Code Quality Standards

- **Formatting**: Consistent formatting enforced by `cargo fmt`.
- **Linting**: No warnings or errors allowed by `cargo clippy` (enforced via `-D warnings` in checking).
- **Testing**: Unit and integration tests required under `tests/` or inline modules for logic changes.

## Commits

- All commits must pass `task check` and `task test` before being pushed.
- All commit messages must follow the conventional commit format: `type(scope): description`

Examples:
- `feat(cli): add list command`
- `fix(linker): handle broken symlinks correctly`
- `chore(deps): bump clap to 4.4`
