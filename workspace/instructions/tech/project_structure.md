# Project Structure

The `skm` project is organized as a standard Rust binary crate.

```txt
Cargo.toml       # Dependency declarations and build configurations
Cargo.lock       # Pinning of compiled dependencies
Taskfile.yml     # Local check, fix, test, and build task entrypoints
AGENTS.md        # Contributor and agent workflow rules
DESIGN.md        # Root design tokens and UI policy
skills.yaml      # SKM installation manifest; personal skill entries stay uncommitted
workspace/releases.json # Shared SKM release reservations and historical-spec boundary

workspace/
  instructions/  # Task, SDLC, CI, structure, and versioned standards
  specs/         # Lifecycle-managed feature specs and the root catalog
  docs/          # Architecture, work notes, and concurrent-peer records
  agents/memory/ # Durable project memory
  company/       # Reserved business context

src/
  main.rs        # Command line arguments routing and subcommand logic
  config.rs      # Data structs, YAML serialization/deserialization for skills.yaml
  linker.rs      # Path validation, target resolution, symlink checks, and linking
```

The contributor governance version is pinned in the generated `AGENTS.md`
context and the contained `default`/`latest` aliases under
`workspace/instructions/standards/workspace-docs/`. Workspace validation checks
those declarations independently of `skills.yaml`.

The committed SKM manifest is a minimal installation example and contains no
Workspace metadata or personal skill entries. Target projects create their own
installation manifests with `skm init`. Generic local toolkit/bundle/profile
configuration remains optional SKM installation functionality.

`scripts/validate_workspace.py`, `scripts/validate_versions.py`, and
`scripts/validate_coordination.py` are the repository's Workspace Docs gates.
The portable peer runtime is stored under
`workspace/instructions/skills/coordinate-multi-agent-development/scripts/`.
`task peers -- <command>` is its Taskfile wrapper.
`workspace/validation/peer-commands.json` selects the complete Taskfile check
and test gates for reviewed peer integration; focused-result reuse is disabled.

Generated build artifacts live under `target/` and scratch work lives under `scratch/`; neither is part of the source ownership model.
