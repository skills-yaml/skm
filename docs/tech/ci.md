# CI Rules

## Scope

This document defines CI/CD pipeline rules for the `skills-yaml` project. These rules apply to any automated workflows (like GitHub Actions) that lint, format, check, test, and package the `skm` tool.

## Pipeline Integration

CI workflows must align with our local task automation setup:

1. **Invoke Taskfiles**: The CI pipeline must call tasks defined in `Taskfile.yml` rather than invoking tools (like rustfmt, clippy, cargo) directly.
2. **Authoritative Check**: `task check` must be the entrypoint for formatting and static analysis checks. Any failure must block the build.
3. **Deterministic Verification**: `task test` must run to execute unit and integration tests.

## Recommended GitHub Actions Workflow

```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Install Task
        uses: arduino/setup-task@v2
        with:
          repo-token: ${{ secrets.GITHUB_TOKEN }}

      - name: Run checks
        run: task check

      - name: Run tests
        run: task test
```

## Do

* **Use Taskfiles**: CI workflow steps must use the `task` execution interface.
* **Pin Actions**: All third-party actions in GitHub workflows must use immutable refs (e.g. `@v4`).
* **Pin Toolchains**: The Rust compiler toolchain should be pinned (e.g. `stable`).

## Don't

* **Bypass Taskfiles**: Do not invoke `cargo fmt` or `cargo clippy` directly in workflow steps.
* **Proceed on Failures**: Never build or release if the check or test phases fail.
