# Project Structure

The `skills-yaml` project is organized as a standard Rust binary crate.

```txt
Cargo.toml       # Dependency declarations and build configurations
Cargo.lock       # Pinning of compiled dependencies
skills.yaml      # Self-referential config for skm developers

docs/
  specs/         # Functional requirements and guidelines
    foundation/  # High-level product and CLI specifications
  tech/          # Technical design documents

src/
  main.rs        # Command line arguments routing and subcommand logic
  config.rs      # Data structs, YAML serialization/deserialization for skills.yaml
  linker.rs      # Core logic for resolving target locations and symlinking skills
```