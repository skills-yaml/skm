# Project Structure

The `skm` project is organized as a standard Rust binary crate.

```txt
Cargo.toml       # Dependency declarations and build configurations
Cargo.lock       # Pinning of compiled dependencies
Taskfile.yml     # Local check, fix, test, and build task entrypoints
AGENTS.md        # Contributor and agent workflow rules
DESIGN.md        # Root design tokens and UI policy
skills.yaml      # Workspace-docs 6.0.0 project pin; personal skill entries stay uncommitted
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

The committed project manifest pins the `workspace-docs@6.0.0` candidate and
does not publish personal skill entries. Target projects still create their own
manifests with `skm init`.

`scripts/validate_workspace.py`, `scripts/validate_versions.py`, and
`scripts/validate_coordination.py` are the repository's Workspace Docs gates.
The portable peer runtime is stored under
`workspace/instructions/skills/coordinate-multi-agent-development/scripts/`.
`task peers -- <command>` is its Taskfile wrapper.
`workspace/validation/peer-commands.json` selects the complete Taskfile check
and test gates for reviewed peer integration; focused-result reuse is disabled.

Generated build artifacts live under `target/` and scratch work lives under `scratch/`; neither is part of the source ownership model.
