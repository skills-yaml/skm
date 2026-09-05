# System and Agent Instructions

Static, read-only technical guidance, versioned standards, agent role files,
and skills for this repository.

For conflicts among repository-controlled files, `workspace/instructions/` is
the source of truth. Historical standard packages apply only when this
repository is pinned to that version.

## Subdirectories

- [`tech/`](./tech/): Task, SDLC, CI, and project-structure rules for this
  Rust CLI.
- [`standards/`](./standards/): Versioned workspace documentation standard
  (`workspace-docs@5.0.0`).
- [`agents/`](./agents/): Reserved for agent role instructions. Empty until
  project-specific roles exist.
- [`skills/`](./skills/): Repository skill instructions, including
  `adopt-workspace-structure`.

Do not edit these files during ordinary feature work. A direct human request
to adopt, upgrade, repair, or migrate workspace structure is the approval
for the governed instruction changes required by that named target.
