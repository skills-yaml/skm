# skm

`skm` is a Rust CLI for managing AI agent skills from a declarative
`skills.yaml` manifest.

It installs skills from any registry by creating symlinks into supported agent
skill directories, so a project can declare the skills it needs once and keep
all sixteen supported agent clients in sync. Project-local `skm install`
writes a deterministic `skills.lock.yaml` after linking succeeds.

[Install](#install) | [Release Channels](#release-channels) | [Quick Start](#quick-start) | [Commands](#commands) | [Configuration](#configuration) | [Installation scopes](workspace/docs/installation-scopes.md) | [Toolkits](#toolkits)

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
notices. Interactive help requests, including bare `skm`, show the same
available-update notice before the help text.

Older managed SKM binaries use `skm update` to install a new release. The new
binary accepts their internal identity check during that upgrade, then uses
`skm self upgrade` for later updates. If an already-published build rejects the
upgrade, install the latest build with the official installer.

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
The older `--interactive` and `--advanced` flags remain accepted but are
hidden from help; interactive prompts are already the default.
`--global` edits the user manifest at `~/.skm/skills.yaml`. Select the
agents that should receive global skills; `~/.skm/config.yaml` contains base
settings, not the global skill selection.

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
skm add software-development/spec --registry default --kind skill
```

`skm add` previews the manifest entry and asks for confirmation. The prompt
defaults to No; pass `--yes` for non-interactive use. It writes the entry only;
run `skm install --yes` afterwards to link skills and update the lockfile.
Existing scripts that expect `add --yes` to link must append that install command.
Run `skm install` before `skm check` after adding an entry.

The item kind is detected automatically. When a skill and bundle share an ID,
select `--kind skill` or `--kind bundle` explicitly. Published bundles cannot be
added with `--global`. Re-adding an existing entry with the same kind and source
is a no-op.

Use `--json` for machine-readable search results, including result kinds,
dependencies, collections, and bundle members, and `--limit <number>` to bound
the combined result list. Search is read-only. Preview or add a bundle with:

```sh
skm add skills-yaml/authoring-toolkit --registry default --dry-run
skm add skills-yaml/authoring-toolkit --registry default --json
skm add skills-yaml/authoring-toolkit --registry default --yes
skm install --yes
```

`--dry-run` and `--json` preview skill or bundle entries without writing. A
preview of an uncached remote registry requires `skm cache refresh <registry>`
first; normal add can inspect that remote without a persistent cache. Bundle
manifests must satisfy the published schema-2 contract.

A bundle entry in `skills.yaml` stays as one declarative entry — members are
resolved at install time, not unpacked:

```yaml
skills:
  - name: workspace/all-workspace-skills
    kind: bundle
    version: latest
    source: default
```

`skm install` resolves the bundle to its members and writes both the bundle
and its resolved members into `skills.lock.yaml`. `skm check` detects when the
published bundle changes members or a member's published version changes.
`skm skill outdated` reports these changes and `skm skill upgrade --all`
updates the lock and links, removing obsolete members that no other entry needs.
When publishers provide bundle release history, `latest` and `default` select the
newest numeric release. Pin a published release with
`skm skill use workspace/all-workspace-skills@1.2.0 --yes`; this updates the
manifest, links and lock together. `outdated` reports newer releases, while
`upgrade` keeps exact bundle pins unchanged; use `skill use` to change one.
Legacy bundles without history support aliases but cannot resolve exact pins.
See [registry formats](workspace/docs/registry-format.md) for publisher details.

Install the skills declared in `skills.yaml` into project-local agent folders:

```sh
skm install
```

Select at least one agent with a project skill directory in `skills.yaml`
before a skills-only install. For example, `agents: [codex]` links
`software-development/spec` as `.agents/skills/spec` so Codex can discover it.
`agents: []` and project-only Hermes configurations now report an error for
skills-only installs. Hermes uses `skm install --global` for its global target.

Preview link and lockfile changes, then apply:

```sh
skm install --dry-run
skm install --yes
```

`--json` emits the same read-only plan as structured data. Project installs
write `skills.lock.yaml` last and restore prior links if an operation fails.
Global installs link into user-level agent directories and do not write a
project lockfile. When a project already has a skills-only lockfile, `add`,
`remove`, and named version changes refresh it after a successful manifest
change. Run `skm install` after direct edits to `skills.yaml`.

List link status:

```sh
skm list
```

Validate the manifest, sources, `SKILL.md` files, and symlink targets:

```sh
skm check
```

Use `--global` with skill commands to select the user manifest and user-level agent directories from any working directory.

## Project-local or global?

Use project-local skills for repository-specific, team-reviewed workflows. Use
global skills for a user's baseline tools across projects. Both can coexist;
`--global` selects `~/.skm/skills.yaml` and the selected agents’ user-level destinations. See [installation scopes](workspace/docs/installation-scopes.md) for
examples, tradeoffs, duplicate-name behavior, and daily update commands.

## Commands

```txt
skm init [--name <name>] [--global] [--non-interactive]
skm install [--global] [--dry-run] [--json] [--yes]
skm add <skill-or-bundle> [--registry <registry>] [--kind skill|bundle] [--path <local-path>] [--global] [--dry-run | --json | --yes]
skm search <query> [--registry <registry>] [--json] [--limit <limit>]
skm remove <skill-name> [--global] [--dry-run] [--yes]
skm list [--global]
skm check [--global]
skm cache refresh [registry]
skm cache status [registry]
skm cache prune <registry>|--all [--keep <number>] [--dry-run] [--yes]
skm cache clear <registry>|--all [--dry-run] [--yes]
skm skill versions <skill-name> [--registry <registry>] [--json] [--stable-only] [--pre] [--limit <limit>]
skm skill use <skill-name>@<version> [--global] [--yes] [--dry-run]
skm skill outdated [skill-name] [--global] [--refresh] [--pre]
skm skill upgrade <skill-name>|--all [--global] [--yes] [--dry-run] [--pre]
skm self version
skm self check [--channel prod|development]
skm self upgrade [--channel prod|development] [--yes]
skm dev link <path> [--name <name>] [--source <source>] [--global] [--all-agents] [--agent <agents>] [--force] [--verbose]
skm dev unlink <skill-name> [--global] [--yes] [--verbose]
skm dev list [--global] [--all] [--json] [--paths]
skm dev show <skill-name> [--global] [--json]
skm setup
skm registry add|remove|list|set-default|info ...
skm config get|set|unset|show|reset|validate ...
skm clean symlinks|reset ...
```

- `init`: creates or edits `skills.yaml` through sequential prompts; use
  `--non-interactive` to create a default manifest for scripts. Toolkit flags
  are described in [Toolkits](#toolkits).
- `install`: resolves configured skills once, preflights every target,
  transactionally links each one, and writes the project lockfile last.
- `add`: writes a skill or published bundle entry to `skills.yaml`, prompting
  unless `--yes` is set. `--dry-run` and `--json` preview either kind without
  writing. Run `install` to resolve and link entries.
- `search`: searches configured registries for skills and published bundles in
  one result list, shows a matching `skm add` command for each, and lists
  browse-only namespace collections without changing project state.
- `list`: reports current link status, including missing sources and bad links.
- `check`: verifies source directories, `SKILL.md`, symlink existence, and symlink targets; intended for CI.
- `cache refresh`: updates one effective project or inherited global registry,
  or all effective registries when no name is given.
- `cache status`, `prune`, and `clear`: inspect cached registries, remove unprotected older versions, or clear selected caches.
- `skill versions`: lists available versions from a cached registry.
- `skill use`: switches a skill to a specific version (e.g. `skill@v1.2.0`) in `skills.yaml` and re-links it.
- `skill outdated`: compares pinned registry skills with cached versions; `--refresh` fetches relevant registries first. Local and `latest`-tracking skills are skipped.
- `skill upgrade`: updates one skill pin, or uses `--all` to preview and apply
  every outdated exact pin together from the local cache.
- `self check`, `self upgrade`, and `self version`: inspect and update the SKM binary.
- `dev`: links, unlinks, lists, and inspects local development skills.
- `setup`: ensures global base configuration and fetches the default registry.
- `registry set-default`: selects the global default for new additions and
  unqualified version lookups. Existing manifest entries without `source`
  still use the literal registry named `default`.
- `config reset`: writes default configuration; `clean reset` removes selected
  configuration, cache, or managed links after confirmation.

`skm add --source` and `skm registry default` remain accepted compatibility
spellings. `skm bundle`, `skm dev mode`, and `skm init-config` have been retired.

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

A published registry bundle can be declared as a single `kind: bundle` entry
without unpacking its members:

```yaml
skills:
  - name: workspace/all-workspace-skills
    kind: bundle
    version: latest
    source: default
```

`kind` is optional and defaults to `skill`. `kind: bundle` entries are
registry-only (no `path`), require an explicit registry `source`, and are
resolved to their published members at install time. The lockfile records both the bundle and its resolved members.

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

## Toolkits

SKM also installs local toolkits of versioned skills, bundles, and role profiles.
The optional `toolkit`, `bundles`, and `profiles` fields use SKM-owned formats;
publishers supply domain behavior through their skills. See
[toolkit configuration](workspace/docs/toolkits.md) and the
[registry format reference](workspace/docs/registry-format.md).

## Development

Use the Taskfile entrypoints:

```sh
task check
task test
task build
```

`task check` runs formatting checks, Clippy with warnings denied, `cargo check`,
and the Workspace Docs structure, spec catalog, memory impact, privacy,
version reservation, and peer record gates. The project currently pins the
`workspace-docs@7.0.0` standard; both standard aliases select released 7.0.0.
The governance pin lives in `AGENTS.md` and the Workspace standard aliases.
Workspace validation does not require governance metadata in `skills.yaml`.
Workspace Docs 7 separates verified main completion from publication and adds
blocked-spec resume metadata. See [validation modules](workspace/validation/README.md).

After `task build`, run `task test:init` for Linux terminal smoke coverage of
the sequential init prompts, including registry search, saving, cancellation,
external edits, and cooked-terminal behavior.
