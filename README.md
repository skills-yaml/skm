# skm

`skm` is a Rust CLI for managing AI agent skills from a declarative
`skills.yaml` manifest.

It installs skills from any registry by creating symlinks into supported agent
skill directories, so a project can declare the skills it needs once and keep
all sixteen supported agent clients in sync. `skm install` commits a
deterministic `skills.lock.yaml`.

[Install](#install) | [Release Channels](#release-channels) | [Quick Start](#quick-start) | [Commands](#commands) | [Configuration](#configuration) | [Workspace Toolkits](#workspace-toolkits)

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

GitHub Actions builds release binaries for Linux, macOS, and Windows. The Linux release is built for musl so it does not require a specific host glibc version. Each build is packaged as a GitHub Release asset:

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

Both channels publish the four packaged binaries, their SHA-256 files, and a
strict `skm-release.json` manifest. The manifest binds the channel, tag,
version, commit, archive names, archive sizes, and checksums. Publication is a
recoverable transaction, so a moving tag never intentionally exposes a partial
release asset set. Production installers use `prod-latest` by default; pass
`development` to install from `development-latest`.

## Updates

Release builds embed the Git commit and release channel they were built from.
`skm self upgrade` reads the selected channel's release manifest over HTTPS, verifies
the exact platform archive against both its checksum file and manifest digest,
extracts one expected binary, verifies its embedded identity, then replaces the
installed regular file using a guarded update transaction. Local builds are not
self-update managed; reinstall them with Cargo or the official installer.

Check for a production update:

```sh
skm self check
```

Install the latest production build:

```sh
skm self upgrade --yes
```

Check or install the development channel:

```sh
skm self check --channel development
skm self upgrade --channel development --yes
```

`skm self check` reports whether the managed channel has a newer build. Use
`skm self version` to print the embedded version, channel, and
commit. Set `SKM_NO_UPDATE_CHECK=1` to suppress best-effort terminal startup
notices. Interactive help requests show the same available-update notice
before the help text.

This command change requires a bridge release before production promotion:
older SKM updaters verify downloaded binaries by invoking the removed top-level
`version` command. Until a bridge updater is released and installed, use the
official installer to move from an older binary to the new CLI. Release update
qualification rejects an older bootstrap binary rather than claiming that its
self-update path works.

## Quick Start

Create a project manifest:

```sh
skm init
```

`init` loads `skills.yaml` if present, or starts a new draft, and then walks
through ordinary line-by-line prompts for project details, agents, registries,
skills, optional settings, and final review.

- Press **Enter** to keep the value shown in a prompt.
- Enter **-** to clear an optional text value.
- Enter **:q** at any prompt to cancel without writing.
- Select agents by number or name, or enter **all** or **none**.
- Registry and skill sections use short action commands such as **a** to add,
  **e 2** to edit item 2, and **d 2** to remove item 2 with confirmation.

The skill section loads configured project and inherited global registries only
when you enter **s** to search or **r** to refresh and search. Enter a query,
then toggle numbered results. Matches are case-insensitive and show the skill's
registry and available version. Existing pins and extension fields stay intact;
a same-name skill from another source must be edited explicitly. Use **a** for
manual registry or local-path entries.

Local registries and matching cached Git registries load directly. Uncached Git
registries are inspected in temporary storage without a checkout or
installation. A refreshed search fetches current remote contents. One failing
registry is reported without hiding results from the others.

After the optional settings question, `init` validates and prints the complete
YAML before asking whether to save. Cancellation and end-of-input leave the
manifest unchanged. An unchanged save preserves its original bytes; edited
saves preserve configuration values, including extension fields, but may
normalize YAML formatting and comments. The prompt flow refuses to replace
malformed files, symlinks, or files changed externally while it is open.

For scripts, create a manifest with the existing defaults:

```sh
skm init --non-interactive --name my-project
```

Non-interactive initialization refuses to replace an existing manifest.
`--advanced` is a compatibility alias for the same complete prompt flow.
`--global` prepares for global installation; `skills.yaml` remains in the
current directory.

Search configured project and inherited global registries for skills and
published bundles by name:

```sh
skm search spec
```

Text results use two columns. The first shows each name, with the version
immediately after a skill name and a `skill` or `bundle` label. The second shows
the description and registry on separate lines. Skill dependencies appear
below the registry when present; published schema-2 bundles show a summary of
their member skills. For example:

```text
NAME (TYPE)                         DESCRIPTION
──────────────────────────────────  ─────────────────────────────────────────────
software/spec@1.2.0 (skill)         Write clear, reviewable software specs.
                                    Registry: default

software/starter (bundle)           Includes 2 skills: software/spec, software/review.
                                    Registry: default
```

Namespace collections remain browse-only; they are not installable bundles.
Add a selected skill with:

```sh
skm add software-development/spec --source default --kind skill
```

`skm add` shows the planned configuration, resolved skills, and agent link
changes, then asks for confirmation. The prompt defaults to No. Pass
`--yes` to apply without a prompt, including from a script; a non-interactive
add without `--yes` fails before changing the project. This applies to both
single skills and published bundles.

Search commands include `--kind skill` or `--kind bundle` so they remain
unambiguous if a registry publishes both with the same ID. For a unique ID,
`skm add` chooses the item automatically.

Use `--json` for machine-readable search results, including result kinds,
dependencies, collections, and bundle members, and `--limit <number>` to bound
the combined result list. Search is read-only; use `skm add` to install a skill
or every member of a published bundle. Preview or add a bundle with:

```sh
skm add skills-yaml/authoring-toolkit --source default --kind bundle --dry-run
skm add skills-yaml/authoring-toolkit --source default --kind bundle
skm add skills-yaml/authoring-toolkit --source default --kind bundle --yes
```

For bundles, `--dry-run` and `--json` preview without asking or changing the
project; `--json` emits a structured plan. These preview flags do not apply to
single skills. Bundle add requires a project `skills.yaml` with an effective
agent target. It expands the bundle and its exact same-registry dependencies
into ordinary pinned `skills`
entries, then links them as one rollback-protected project change. Repeating
the same command succeeds without changing the project. Conflicting pins and
real-file targets stop the whole operation. A published schema-2 bundle is
required.

Install the skills declared in `skills.yaml` into project-local agent folders:

```sh
skm install
```

Select at least one agent with a project skill directory in `skills.yaml`
before a skills-only install. For example, `agents: [codex]` links
`software-development/spec` as `.agents/skills/spec` so Codex can discover it.
`agents: []` and project-only Hermes configurations now report an error for
skills-only installs. Hermes uses `skm install --global` for its global target.

Preview every write and then apply non-interactively:

```sh
skm install --dry-run
skm install --yes
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
skm init [--name <name>] [--global] [--non-interactive] [--advanced]
skm install [--global] [--dry-run] [--json] [--yes]
skm add <skill-or-bundle> [--source <registry>] [--kind skill|bundle] [--path <local-path>] [--global] [--dry-run | --json | --yes]
skm bundle add <namespace/bundle> [--source <registry>] [--dry-run | --json | --yes]
skm search <query> [--registry <registry>] [--json] [--limit <limit>]
skm list [--global]
skm check [--global]
skm cache refresh [registry]
skm cache status [registry]
skm cache prune <registry>|--all [--keep <number>] [--dry-run] [--yes]
skm cache clear <registry>|--all [--dry-run] [--yes]
skm skill versions <skill-name> [--registry <registry>] [--json] [--stable-only] [--pre] [--limit <limit>]
skm skill use <skill-name>@<version> [--global] [--yes] [--dry-run]
skm skill outdated [skill-name] [--refresh] [--pre]
skm skill upgrade <skill-name> [--global] [--yes] [--dry-run] [--pre]
skm self version
skm self check [--channel prod|development]
skm self upgrade [--channel prod|development] [--yes]
skm dev link <path> [--name <name>] [--source <source>] [--global] [--all-agents] [--agent <agents>] [--force] [--verbose]
skm dev unlink <skill-name> [--global] [--yes] [--verbose]
skm dev list [--global] [--all] [--json] [--paths]
skm dev show <skill-name> [--global] [--json]
skm dev mode [on|off|status] [--global]
```

- `init`: creates or edits `skills.yaml` through sequential prompts; use
  `--non-interactive` to create a default manifest for scripts. Toolkit flags
  are described in [Workspace Toolkits](#workspace-toolkits).
- `install`: resolves configured skills once, preflights every target,
  transactionally links each one, and writes the lockfile last.
- `add`: adds and links one skill, or expands a published registry bundle and
  exact dependencies into project skill pins. Both paths show a plan and prompt
  unless `--yes` is set. `--dry-run` and `--json` preview bundles without
  applying; bundle application remains transactional.
- `bundle add`: compatibility command with the same bundle confirmation flow.
- `search`: searches configured registries for skills and published bundles in
  one result list, shows a matching `skm add` command for each, and lists
  browse-only namespace collections without changing project state.
- `list`: reports current link status, including missing sources and bad links.
- `check`: verifies source directories, `SKILL.md`, symlink existence, and symlink targets; intended for CI.
- `cache refresh`: updates one registry cache or all configured registry caches when no name is given.
- `cache status`, `prune`, and `clear`: inspect cached registries, remove unprotected older versions, or clear selected caches.
- `skill versions`: lists available versions from a cached registry.
- `skill use`: switches a skill to a specific version (e.g. `skill@v1.2.0`) in `skills.yaml` and re-links it.
- `skill outdated`: compares pinned registry skills with cached versions; `--refresh` fetches relevant registries first. Local and `latest`-tracking skills are skipped.
- `skill upgrade`: updates a skill pin to its latest cached version and re-links it.
- `self check`, `self upgrade`, and `self version`: inspect and update the SKM binary.
- `dev`: manages local development skills (linking local paths as symlinks directly, toggling dev mode).

## Configuration

Example `skills.yaml`:

```yaml
name: my-project
version: 0.1.0
registries:
  default: git@github.com:skills-yaml/registry.git
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

### Registry skill dependencies

SKM 0.4 reads optional exact same-registry dependencies from the Agent Skills
string metadata map:

```yaml
metadata:
  skm-version: "0.1.0"
  skm-dependencies: "my-namespace/write-spec@0.2.0, my-namespace/review-changes@0.2.0"
```

Dependencies must use `namespace/name@MAJOR.MINOR.PATCH`. Resolution inherits
the selected trusted registry, expands the complete closure before writes, and
rejects missing packages, malformed or duplicate declarations, cycles, and
conflicting exact versions. Local-path skills cannot declare registry
dependencies. When a `latest` or `default` package declares `skm-version`, the
lockfile records that immutable version instead of the moving alias.

## Link Targets

| Agent | Project path | Global path |
| --- | --- | --- |
| Claude | `.claude/skills` | `~/.claude/skills` |
| Codex | `.agents/skills` | `~/.agents/skills` |
| Copilot | `.github/skills` | `~/.copilot/skills` |
| Cursor | `.cursor/skills` | `~/.cursor/skills` |
| Antigravity | `.agents/skills` | `~/.gemini/config/skills` |
| Pi | `.pi/skills` | `~/.pi/agent/skills` |
| OpenCode | `.opencode/skills` | `~/.config/opencode/skills` |
| Cline | `.cline/skills` | `~/.cline/skills` |
| Kilo Code | `.kilo/skills` | `~/.kilo/skills` |
| Gemini CLI | `.gemini/skills` | `~/.gemini/skills` |
| Goose | `.agents/skills` | `~/.agents/skills` |
| Crush | `.crush/skills` | `~/.config/crush/skills` |
| OpenHands | `.agents/skills` | `~/.openhands/skills` |
| Grok | `.grok/skills` | `~/.grok/skills` |
| Qwen Code | `.qwen/skills` | `~/.qwen/skills` |
| Hermes | *(none)* | `~/.hermes/skills` |

When multiple selected agents use the same directory, SKM writes and checks it
once and records every claimant in `skills.lock.yaml`. Hermes does not scan a
project-local skills directory; users who want project content available to
Hermes can add a shared directory through `external_dirs` in
`~/.hermes/config.yaml`.

The table records SKM link targets, not a guarantee that every agent has been
tested end to end with every published skill. Project skill discovery may also
require agent-specific setup: for example, Pi and Gemini CLI require a trusted
workspace before they load project skills. SKM places skills directly under
each target directory using the `SKILL.md` name; packages with the same final
name cannot be installed together into one project.

## Safety

`skm` validates skill names and registry names before filesystem operations. It rejects absolute paths, `..`, empty path components, unsupported agents, and unsafe registry names.

Linking never overwrites a real file or directory; SKM only replaces existing
symlinks, and `skm check` verifies that each link points to its expected source.

## Workspace Toolkits

SKM can also install Workspace development toolkits: versioned bundles, role
profiles, and a pinned workspace-docs standard declared with optional
`toolkit`, `bundles`, `profiles`, and `workspace` fields in `skills.yaml`.
Projects that only manage skills never need these fields. Configuration,
`init` flags, adoption with the `wk-adopt` skill, integrity guarantees, and the
policy of installed Workspace workflows are documented in
[workspace/docs/workspace-toolkit.md](workspace/docs/workspace-toolkit.md).

## Development

Use the Taskfile entrypoints:

```sh
task check
task test
task build
```

`task check` runs formatting checks, Clippy with warnings denied, `cargo check`,
and the workspace-docs structure, spec-catalog, memory-impact, and privacy gates.

After `task build`, run `task test:init` for Linux terminal smoke coverage of
the sequential init prompts, including registry search, saving, cancellation,
external edits, and cooked-terminal behavior.
