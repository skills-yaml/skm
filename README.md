# skm

`skm` is a Rust CLI for managing AI agent skills and Workspace development
toolkits from a declarative `skills.yaml` manifest.

It installs skills by creating symlinks into supported agent skill directories,
so a project can declare the skills it needs once and keep all sixteen
supported agent clients in sync. Toolkit projects can also select
versioned bundles and portable role profiles, render agent-specific projections,
and commit a deterministic `skills.lock.yaml`.

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
`skm update` reads the selected channel's release manifest over HTTPS, verifies
the exact platform archive against both its checksum file and manifest digest,
extracts one expected binary, verifies its embedded identity, then replaces the
installed regular file using a guarded update transaction. Local builds are not
self-update managed; reinstall them with Cargo or the official installer.

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

`--check` retains compatibility for scripts and reports whether the managed
channel has a newer build. `--yes` is accepted for compatibility with older
automation. Use `skm version` to print the embedded version, channel, and
commit. Set `SKM_NO_UPDATE_CHECK=1` to suppress best-effort terminal startup
notices.

## Quick Start

Create a project manifest:

```sh
skm init
```

`init` loads `skills.yaml` if present, or starts a new draft, and then walks
through ordinary line-by-line prompts for project details, agents, registries,
skills, optional toolkit/workspace settings, and final review.

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

Search configured project and inherited global registries by skill name:

```sh
skm search spec
```

Each result includes its registry, available version, and a command that adds
and links it. Use the dedicated add command from a result:

```sh
skm add software-development/spec --source default
```

Use `--json` for machine-readable results and `--limit <number>` to bound the
result list. Search is read-only; use `skm add` when you want to update the
manifest and link a skill.

Install the skills declared in `skills.yaml` into project-local agent folders:

```sh
skm install
```

For a configured toolkit, preview every write and then apply non-interactively:

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
skm init [--name <name>] [--global] [--non-interactive] [--advanced] [--toolkit-manifest <path>] [--toolkit-version <version>] [--bundle <id>] [--profile <id>] [--workspace-standard <id>] [--workspace-source <path-or-git-url>] [--workspace-revision <commit>] [--workspace-integrity <sha256>] [--trusted-source <source>]
skm install [--global] [--dry-run] [--json] [--yes]
skm add <skill-name> [--source <registry>] [--path <local-path>] [--global]
skm search <query> [--registry <registry>] [--json] [--limit <limit>]
skm list [--global]
skm check [--global]
skm update [--channel prod|development] [--check] [--yes]
skm versions <skill-name> [--registry <registry>] [--json] [--stable-only] [--pre] [--limit <limit>]
skm use <skill-name>@<version> [--global] [--yes] [--dry-run]
skm update-skill <skill-name> [--global] [--yes] [--dry-run] [--pre]
skm dev link <path> [--name <name>] [--source <source>] [--global] [--all-agents] [--agent <agents>] [--force] [--verbose]
skm dev unlink <skill-name> [--global] [--yes] [--verbose]
skm dev list [--global] [--all] [--json] [--paths]
skm dev show <skill-name> [--global] [--json]
skm dev mode [on|off|status] [--global]
skm workspace audit [--target <version>] [--source <path-or-git-url>] [--revision <commit>] [--integrity <sha256>] [--json]
skm workspace adopt|upgrade|repair [--target <version>] [--source <path-or-git-url>] [--revision <commit>] [--integrity <sha256>] [--apply] [--yes] [--json]
```

- `init`: creates or edits `skills.yaml` through sequential prompts; use
  `--non-interactive` to create a default manifest for scripts.
- `install`: resolves configured skills and toolkit bundles once, preflights
  every target, transactionally materializes each adapter, and writes the
  lockfile last.
- `add`: adds one skill to `skills.yaml`, then links it.
- `search`: searches configured registries by skill name and prints a dedicated
  `skm add` command for each result without changing project state.
- `list`: reports current link status, including missing sources and bad links.
- `check`: verifies source directories, `SKILL.md`, symlink existence, and symlink targets; intended for CI.
- `update`: checks the selected release channel and installs the latest release artifact.
- `versions`: lists all available versions for a skill from a registry.
- `use`: switches a skill to a specific version (e.g. `skill@v1.2.0`) in `skills.yaml` and re-links it.
- `update-skill`: updates a skill to its latest version in `skills.yaml` and re-links it.
- `dev`: manages local development skills (linking local paths as symlinks directly, toggling dev mode).
- `workspace`: audits trusted `workspace-docs` packages and creates a verified,
  resumable handoff for adoption, upgrade, or repair. It does not interpret
  migration prose as executable code.

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
  skm-dependencies: "workspace/write-spec@0.2.0, workspace/review-changes@0.2.0"
```

Dependencies must use `namespace/name@MAJOR.MINOR.PATCH`. Resolution inherits
the selected trusted registry, expands the complete closure before writes, and
rejects missing packages, malformed or duplicate declarations, cycles, and
conflicting exact versions. Local-path skills cannot declare registry
dependencies. When a `latest` or `default` package declares `skm-version`, the
lockfile records that immutable version instead of the moving alias.

### Workspace toolkit configuration

Toolkit fields are optional, so existing skills-only manifests remain valid:

```yaml
name: my-project
version: 1.0.0
agents:
  - codex
  - cursor
skills: []
toolkit:
  manifest: workspace/instructions/toolkit/manifest.yaml
  version: 0.3.0
bundles:
  - development-core
profiles:
  - security-reviewer
workspace:
  standard: workspace-docs@5.0.0
  source: workspace/instructions/standards/workspace-docs
trusted_sources:
  - workspace/instructions/standards/workspace-docs
```

Toolkit and local workspace source paths must be repository-relative and may not
contain symlinks. The committed `skills.lock.yaml` records toolkit and workspace
versions and integrity, resolved skill and profile versions and integrity,
adapter versions and capabilities, and every managed output.

SKM accepts toolkit packages targeting Workspace Docs 4.x, 5.x, or 6.x and
toolkit skill entries with dependency IDs. Selected bundles expand the complete
toolkit dependency closure. The current Workspace toolkit uses the 5.x
`backlog -> development -> test -> done`
lifecycle; `develop` and `main` are conventional targets that repositories may
replace with explicitly documented equivalents.

The initial profile adapters are deliberately different: Codex receives native
project custom-agent TOML under `.codex/agents/`; Cursor receives an explicitly
labeled generated skill fallback under `.cursor/skills/`.

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

## Safety

`skm` validates skill names and registry names before filesystem operations. It rejects absolute paths, `..`, empty path components, unsupported agents, and unsafe registry names.

Toolkit installation scans sources without following symlinks, computes SHA-256
integrity, and refuses every unmanaged collision before writing. SKM replaces or
removes only outputs owned by the previous lockfile. A repository-local journal
backs up managed paths during apply, rolls back a partial failure, and writes the
new lockfile last. An unchanged second install is byte-idempotent.

Workspace source authorization is project-scoped. A partial local standard
package is a read-only blocker; an explicit or committed complete source can
resume the same plan. A Git source requires a full 40-character commit revision
and expected `sha256:` package integrity. SKM fetches only that revision,
materializes blobs without checkout filters, rejects unsafe paths and symlinks,
verifies the package, and caches it inside the project for offline re-application.
Committed Git sources must also appear in `trusted_sources`; an explicit
`--source` authorizes only the current command. No toolkit or workspace command
writes outside the current repository unless an existing non-toolkit command is
explicitly invoked with its established `--global` option.

Installed Workspace workflows enforce OpenTofu as the only infrastructure
mutation mechanism and repository CI/CD as the only mutation environment.
Local work is limited to OpenTofu source changes and non-mutating validation or
review. Applies, destroys, imports, and state mutations run only in CI/CD.
Provider CLIs and consoles are diagnostic-only even in CI/CD; alternative IaC
engines and imperative provisioning are not valid fallbacks.

Installed mutable Workspace workflows also treat the user's delivery request
as standing authority for routine repository work: implementation, tests,
feature-branch commits and pushes, pull-request updates, bounded CI repair,
test integration, and non-destructive releases continue without repeated
approval prompts. Read-only review roles remain read-only. Human approval is
reserved for a specifically identified destructive production action; SKM does
not bypass repository, branch, environment, or CI/CD protections.

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
