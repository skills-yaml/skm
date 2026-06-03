# Project Structure

The `skm` project is organized as a standard Rust binary crate.

```txt
Cargo.toml       # Dependency declarations and build configurations
Cargo.lock       # Pinning of compiled dependencies
Taskfile.yml     # Local check, fix, test, and build task entrypoints
AGENTS.md        # Contributor and agent workflow rules

docs/
  specs/         # Functional requirements and guidelines
    foundation/  # High-level product and CLI specifications
  tech/          # Technical design documents

src/
  main.rs        # Command line arguments routing and subcommand logic
  config.rs      # Data structs, YAML serialization/deserialization for skills.yaml
  linker.rs      # Path validation, target resolution, symlink checks, and linking
```

There is no checked-in `skills.yaml` manifest by default. Developers can create one in a target project with `skm init`.

Generated build artifacts live under `target/` and scratch work lives under `scratch/`; neither is part of the source ownership model.
