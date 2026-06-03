# Task Management

## Scope and Usage

This document defines the standard approach for task automation across the entire project using Taskfiles (gotask/Task). Use this guide when:

- Setting up new project sections (backend, frontend, deployment)
- Creating or modifying Taskfiles for any component (libraries, services, lambdas)
- Defining CI/CD workflows that need to invoke build, test, or deployment operations
- Ensuring consistency in task naming, structure, and behavior across the codebase

**When NOT to use**: CI workflows must not bypass Taskfiles. Local and setup scripts may invoke tools directly when appropriate.

## Overview

The project task management must be handled by gotask (task) [Task](https://github.com/go-task/task)

- The project MUST have a root Taskfile at the project root for project-wide management tasks
- Each project section MUST have a dedicated Taskfile for each project section (backend, fe, deployment)
  - backend/Taskfile.yml
  - fe/Taskfile.yml
  - deployment/Taskfile.yml
- Each project section Taskfile MUST include:
  - `check`    Run linting, static analysis, formatting checks, and type checking; fail if any tool exits with an error
  - `fix`      Auto-format code and apply automatic lint fixes
  - `test`     Must be always present to test and validate the code
  - `build`    Must be always present to build the artifact locally (doesn't apply to tofu projects)
  - `publish`  Must be present for all the project sections that need to publish an artifact; when the deployment model needs to publish an artifact before deploying the project
  - `deploy`   MUST be present for any section that deploys to an environment. When present, deploy MUST depend on (or execute) build and publish as required.
  - Taskfiles MUST use list variables with the list of all the managed projects like (`libs/<lib_name>`, `srv/<service_name>`, `lambda/<lambda_name>`); the varaible MUST be called `PROJECTS`
  - Taskfiles MUST manage multiple projects with the pattern `task check PRJ=<path_to_project>` or `task check:all` check:all will loop all the projects and run the check task; fail if any step exits with an error

## Taskfile Rules

### Do

- CI workflows MUST use Taskfiles as the execution interface for formatting, checks, builds, publishing, and releases.
- `task check` MUST be authoritative and MUST aggregate all validations; any failure MUST block progress.
- Tasks MUST be generic and parametric, and MUST be reusable across libraries, services, and lambdas.
- Task semantics MUST be consistent across all components.

### Don’t (Taskfile Rules)

- Taskfiles MUST NOT be bypassed by invoking tools directly.
- Tasks MUST NOT be ad-hoc or component-specific if they diverge from the standard task set.
- Validation, build, publish, and release MUST NOT be combined into a single task.

## Taskfile path resolution

- Derive paths from explicit vars (e.g. `COMPONENT_ROOT`) passed from the root include.
- Do not rely on `.TASKFILE_DIR` or relative `dir` alone; namespaced invocations resolve differently.
- Prefer paths relative to the current Taskfile
