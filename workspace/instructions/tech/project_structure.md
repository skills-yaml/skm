# Project Structure

The `skm` project is organized as a standard Rust binary crate.

```txt
Cargo.toml       # Dependency declarations and build configurations
Cargo.lock       # Pinning of compiled dependencies
Taskfile.yml     # Local check, fix, test, and build task entrypoints
AGENTS.md        # Contributor and agent workflow rules
DESIGN.md        # Root design tokens and UI policy
skills.yaml      # Workspace-docs pin for this repository; not a skill install set

workspace/
  instructions/  # Task, SDLC, CI, structure, and versioned standards
  specs/         # Lifecycle-managed feature specs and the root catalog
  docs/          # Architecture and work notes
  agents/memory/ # Durable project memory
  company/       # Reserved business context

src/
  main.rs        # Command line arguments routing and subcommand logic
  config.rs      # Data structs, YAML serialization/deserialization for skills.yaml
  linker.rs      # Path validation, target resolution, symlink checks, and linking
```

This repository's `skills.yaml` pins `workspace-docs@5.0.0` and does not install
skills. Target projects still create their own manifests with `skm init`.

Generated build artifacts live under `target/` and scratch work lives under `scratch/`; neither is part of the source ownership model.
