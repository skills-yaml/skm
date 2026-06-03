# skm — Project Document

## Project description

`skm` is a fast Rust-based command-line tool for managing AI agent skills. It uses a declarative `skills.yaml` file to determine which skills are needed for a repository, and automatically caches and symlinks them to the appropriate local configuration folders of active coding agents (Claude, Codex, Cursor, Grok, Hermes, and Copilot).

## Project vision

Provide the standard package management and verification layer for AI agent capabilities, making skill packaging, versioning, distribution, and execution uniform across all development environments.

## Project mission

Provide a lightweight local client that simplifies skill onboarding for developers, reduces prompt token overhead through progressive disclosure, and ensures deterministic environment states via CI validation checks.

## Main features of the MVP

* **Environment Scaffolding (`skm init`)**: Generates a default `skills.yaml` manifest detailing active agents and standard skill registries.
* **Skill Linkage (`skm install`)**: Resolves remote or local skill source paths and installs symlinks into target folders for all active agents.
* **Skill Addition (`skm add`)**: Adds a skill to `skills.yaml`, then links it immediately.
* **Status Listing (`skm list`)**: Displays a clean status summary of active links for each agent, including missing sources and bad links.
* **CI Validation (`skm check`)**: Scans all skill folders and verifies source directories, `SKILL.md`, symlink layout, and symlink targets, exiting with error code `1` if validation fails.
* **Safety Guardrails**: Rejects unsafe skill names, rejects unsupported agents, and refuses to overwrite existing non-symlink files or directories.

## Platform assumptions

The current implementation uses Unix symlinks and targets Unix-like agent configuration directories. Windows support is not currently implemented.
