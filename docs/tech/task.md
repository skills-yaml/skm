# Task Management

## Scope and Usage

This document defines the standard approach for task automation across the project using Taskfiles (gotask/Task). Use this guide when:

- Creating or modifying the root Taskfile
- Adding new project sections later, if the repository grows beyond the current Rust CLI crate
- Defining CI/CD workflows that need to invoke build, test, or deployment operations
- Ensuring consistency in task naming, structure, and behavior across the codebase

**When NOT to use**: CI workflows must not bypass Taskfiles. Local and setup scripts may invoke tools directly when appropriate.

## Overview

The project task management must be handled by gotask (task) [Task](https://github.com/go-task/task).

- The project MUST have a root Taskfile at the project root for project-wide management tasks
- The current root `Taskfile.yml` MUST include:
  - `check`    Run formatting checks, clippy with warnings denied, and compiler checks
  - `fix`      Auto-format code and apply available automatic clippy fixes
  - `test`     Run cargo tests
  - `build`    Build the optimized release binary
- Additional section Taskfiles are only required when new independently managed components are added.

## Taskfile Rules

### Do

- CI workflows MUST use Taskfiles as the execution interface for formatting, checks, builds, publishing, and releases.
- `task check` MUST be authoritative and MUST aggregate all validations; any failure MUST block progress.
- Task semantics MUST remain consistent as new components are added.

### Don’t (Taskfile Rules)

- Taskfiles MUST NOT be bypassed by invoking tools directly.
- Tasks MUST NOT be ad hoc if they diverge from the standard task set.
- Validation, build, publish, and release MUST NOT be combined into a single task.

## Taskfile path resolution

- Derive paths from explicit vars (e.g. `COMPONENT_ROOT`) passed from the root include.
- Do not rely on `.TASKFILE_DIR` or relative `dir` alone; namespaced invocations resolve differently.
- Prefer paths relative to the current Taskfile
