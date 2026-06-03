# skills-yaml — Project Document

## Project description

`skills-yaml` (implemented as the `skm` CLI utility) is a fast, dependency-free Rust-based command-line tool for managing AI agent skills. It uses a declarative `skills.yaml` file to determine which skills are needed for a repository, and automatically caches and symlinks them to the appropriate local configuration folders of active coding agents (such as Claude, Codex, Cursor, Grok, Hermes, and Copilot).

## Project vision

Provide the standard package management and verification layer for AI agent capabilities, making skill packaging, versioning, distribution, and execution uniform across all development environments.

## Project mission

Provide a lightweight, cross-platform local client that simplifies skill onboarding for developers, reduces prompt token overhead through progressive disclosure, and ensures deterministic environment states via CI validation checks.

## Main features of the MVP

* **Environment Scaffolding (`skm init`)**: Generates a default `skills.yaml` manifest detailing active agents and standard skill registries.
* **Skill Linkage (`skm install`)**: Resolves remote or local skill source paths and installs symlinks into target folders for all active agents.
* **Status Listing (`skm list`)**: Displays a clean status summary of active links for each agent.
* **CI Validation (`skm check`)**: Scans all skill folders and verifies symlink layout, exiting with error code `1` if any links are broken or missing.