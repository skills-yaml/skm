# skm

`skm` is a Rust CLI for managing AI agent skills from a declarative `skills.yaml` manifest.

It installs skills by creating symlinks into supported agent skill directories, so a project can declare the skills it needs once and keep Claude, Codex, Cursor, Copilot, Grok, and Hermes in sync.

[Install](#install) | [Quick Start](#quick-start) | [Commands](#commands) | [Configuration](#configuration)

## Install

Prerequisites:

- Rust toolchain with Cargo
- Git, when using remote registries
- A Unix-like environment with symlink support

Install from this checkout:

```sh
cargo install --path .
```

After the repository has a public Git remote, users can install directly from it:

```sh
cargo install --git <repository-url> skm
```

For local development without installing:

```sh
cargo run -- <command>
```

## Quick Start

Create a project manifest:

```sh
skm init
```

Install the skills declared in `skills.yaml` into project-local agent folders:

```sh
skm install
```

List link status:

```sh
skm list
```

Validate the manifest, sources, `SKILL.md` files, and symlink targets:

```sh
skm check
```

Use `--global` with `install`, `list`, or `check` to work against user-level agent directories instead of project-local directories.

## Commands

```txt
skm init [--name <name>]
skm install [--global]
skm add <skill-name> [--source <registry>] [--path <local-path>] [--global]
skm list [--global]
skm check [--global]
```

- `init`: creates a default `skills.yaml`.
- `install`: resolves configured skills and links them for each configured agent.
- `add`: adds one skill to `skills.yaml`, then links it.
- `list`: reports current link status, including missing sources and bad links.
- `check`: verifies source directories, `SKILL.md`, symlink existence, and symlink targets; intended for CI.

## Configuration

Example `skills.yaml`:

```yaml
name: my-project
version: 0.1.0
registries:
  default: git@github.com:skills-yaml/skills-registry.git
agents:
  - claude
  - codex
  - cursor
  - copilot
skills:
  - name: software-development/symphony-spec-writing
    version: latest
    source: default
```

Local offline skills can use `path` instead of a registry source:

```yaml
skills:
  - name: local/my-skill
    version: latest
    path: ./skills/local/my-skill
```

Each skill source directory must contain a `SKILL.md` file.

## Link Targets

Project-local mode links skills under the current project:

```txt
.claude/skills
.codex/skills
.cursor/skills
.github/skills
.grok/skills
.hermes/skills
```

Global mode links under the current user's home directory:

```txt
~/.claude/skills
~/.codex/skills
~/.cursor/skills
~/.copilot/skills
~/.grok/skills
~/.hermes/skills
```

## Safety

`skm` validates skill names and registry names before filesystem operations. It rejects absolute paths, `..`, empty path components, unsupported agents, and unsafe registry names.

When installing, `skm` refuses to replace existing real files or directories. It only replaces existing symlinks, and `skm check` verifies that symlinks point to the expected source.

## Development

Use the Taskfile entrypoints:

```sh
task check
task test
task build
```

`task check` runs formatting checks, Clippy with warnings denied, and `cargo check`.
