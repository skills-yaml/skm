# Workspace Docs Versioning

Workspace documentation standards use semantic versioning:

```text
MAJOR.MINOR.PATCH
```

## Version Meaning

- `MAJOR`: breaking change to required docs, agent workflow, memory format, or spec state rules.
- `MINOR`: backward-compatible addition, such as a new optional doc type or checklist item.
- `PATCH`: clarification, typo fix, or non-behavioral wording change.

## Version Directories

Each version is immutable after release:

```text
workspace-docs/
  v1.0.0/
  v1.1.0/
  v1.2.0/
  v2.0.0/
  v2.1.0/
  v3.0.0/
  v4.0.0/
  v5.0.0/
  latest -> v5.0.0
  default -> v5.0.0
```

The `latest` symlink points to the highest released version. The `default` symlink points to the recommended stable version for project adoption.

## Project Pinning

Projects should record their adopted version in `AGENTS.md` or a generated agent context block:

```md
Workspace docs standard: workspace-docs@5.0.0
```

If a project cannot fully adopt the version, it should document deviations in
its own current-project documentation:

```text
standard_gap: CI guidance is not adopted because the repository has no CI workflow.
```

## Change Control

Changing a released version requires creating a new version directory.

Allowed without a new version:

- fixing broken links inside unreleased drafts
- correcting spelling before adoption
- updating `latest` or `default` symlinks

An exceptional correction to a released package is allowed only when an
explicit migration spec documents that the package contradicts its own declared
version or structure. The correction must restore the originally intended
contract, be recorded in durable memory, and pass standard-package validation.

Requires a new version:

- changing required files
- changing spec state rules
- changing memory storage schema
- changing generated block markers
- changing completion gates
