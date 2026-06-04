# skm

`skm` is a Rust CLI for managing AI agent skills from a declarative `skills.yaml` manifest.

It installs skills by creating symlinks into supported agent skill directories, so a project can declare the skills it needs once and keep Claude, Codex, Cursor, Copilot, Grok, and Hermes in sync.

[Install](#install) | [Release Channels](#release-channels) | [Quick Start](#quick-start) | [Commands](#commands) | [Configuration](#configuration)

## Install

Prerequisites:

- Rust toolchain with Cargo
- Git, when using remote registries
- Symlink support. On Windows, creating symlinks may require Developer Mode or administrator privileges.

Install the latest production release on macOS or Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/skills-yaml/skm/main/scripts/install.sh | sh
```

Install the latest development release on macOS or Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/skills-yaml/skm/main/scripts/install.sh | sh -s -- development
```

Install the latest production release on Windows PowerShell:

```powershell
iwr https://raw.githubusercontent.com/skills-yaml/skm/main/scripts/install.ps1 -OutFile install.ps1
.\install.ps1 -AddToPath
```

Install the latest development release on Windows PowerShell:

```powershell
iwr https://raw.githubusercontent.com/skills-yaml/skm/main/scripts/install.ps1 -OutFile install.ps1
.\install.ps1 -Channel development -AddToPath
```

Install from this checkout:

```sh
cargo install --path .
```

For local development without installing:

```sh
cargo run -- <command>
```

## How Installation Works

GitHub Actions builds release binaries for Linux, macOS, and Windows. Each build is packaged as a GitHub Release asset:

```txt
skm-linux-x86_64.tar.gz
skm-macos-x86_64.tar.gz
skm-macos-aarch64.tar.gz
skm-windows-x86_64.zip
```

The installer script detects the operating system and CPU architecture, downloads the correct asset from GitHub Releases, extracts `skm`, and installs it into:

```txt
~/.local/bin/skm          # macOS/Linux default
%USERPROFILE%\.local\bin # Windows default
```

Override the install directory with `SKM_INSTALL_DIR` on macOS/Linux or `-InstallDir` on Windows.

## Release Channels

The release workflow publishes two moving release channels:

- `main` publishes production artifacts to the `prod-latest` GitHub Release.
- `development` publishes prerelease artifacts to the `development-latest` GitHub Release.

Both channels also upload the same packaged binaries as workflow artifacts for each run. Production installers use `prod-latest` by default; pass `development` to install from `development-latest`.

## Updates

Release builds embed the Git commit and release channel they were built from. `skm update` compares that commit with the current channel tag on GitHub.

Check for a production update:

```sh
skm update --check
```

Install the latest production build:

```sh
skm update --yes
```

Check or install the development channel:

```sh
skm update --channel development --check
skm update --channel development --yes
```

On macOS and Linux, `skm update` runs the shell installer directly. On Windows, it starts a separate PowerShell updater so the currently running `skm.exe` can exit before the binary is replaced.

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
skm update [--channel prod|development] [--check] [--yes]
```

- `init`: creates a default `skills.yaml`.
- `install`: resolves configured skills and links them for each configured agent.
- `add`: adds one skill to `skills.yaml`, then links it.
- `list`: reports current link status, including missing sources and bad links.
- `check`: verifies source directories, `SKILL.md`, symlink existence, and symlink targets; intended for CI.
- `update`: checks the selected release channel and installs the latest release artifact.

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
