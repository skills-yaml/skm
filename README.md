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
notices. Interactive help requests show the same available-update notice
before the help text.

## Quick Start

Create a project manifest:

```sh
skm init
```

`init` loads `skills.yaml` if present, or starts a new draft, and then walks
through ordinary line-by-line prompts for project details, agents, registries,
skills, optional toolkit settings, and final review.

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
Toolkit bundles are a separate source-repository feature. Add a selected skill
with:

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
skm add workspace/all-workspace-skills --source default --kind bundle --dry-run
skm add workspace/all-workspace-skills --source default --kind bundle
skm add workspace/all-workspace-skills --source default --kind bundle --yes
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
`workspace/wk-spec` as `.agents/skills/wk-spec` so Codex can discover it.
`agents: []` and project-only Hermes configurations now report an error for
skills-only installs. Hermes uses `skm install --global` for its global target.
If an older SKM version created a nested `skills/workspace/wk-spec` link,
review and remove that old link after confirming it points to the same source.

For Workspace Docs assessment, adoption, upgrade, or repair, install the
published `wk-adopt` skill in a project with an effective agent target:

```sh
skm add workspace/wk-adopt --source default
skm check
```

SKM also links its exact `adopt-workspace-structure` dependency. Invoke
`wk-adopt` in your agent and state whether you want an assessment, adoption,
upgrade, or repair. The skill verifies the standard and performs authorized
repository work. Existing `.skm/workspace-plan.yaml` files from older SKM
versions remain available for review; SKM no longer creates or updates them.
Existing `trusted_sources` manifest values are preserved but no longer grant
source authorization in SKM.

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
skm init [--name <name>] [--global] [--non-interactive] [--advanced] [--toolkit-manifest <path>] [--toolkit-version <version>] [--bundle <id>] [--profile <id>] [--workspace-standard <id>] [--workspace-source <path-or-git-url>] [--workspace-revision <commit>] [--workspace-integrity <sha256>]
skm install [--global] [--dry-run] [--json] [--yes]
skm add <skill-or-bundle> [--source <registry>] [--kind skill|bundle] [--path <local-path>] [--global] [--dry-run | --json | --yes]
skm bundle add <namespace/bundle> [--source <registry>] [--dry-run | --json | --yes]
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
```

- `init`: creates or edits `skills.yaml` through sequential prompts; use
  `--non-interactive` to create a default manifest for scripts.
- `install`: resolves configured skills and toolkit bundles once, preflights
  every target, transactionally materializes each adapter, and writes the
  lockfile last.
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
- `update`: checks the selected release channel and installs the latest release artifact.
- `versions`: lists all available versions for a skill from a registry.
- `use`: switches a skill to a specific version (e.g. `skill@v1.2.0`) in `skills.yaml` and re-links it.
- `update-skill`: updates a skill to its latest version in `skills.yaml` and re-links it.
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

The table records SKM link targets, not a guarantee that every agent has been
tested end to end with every published skill. Project skill discovery may also
require agent-specific setup: for example, Pi and Gemini CLI require a trusted
workspace before they load project skills. SKM places skills directly under
each target directory using the `SKILL.md` name; packages with the same final
name cannot be installed together into one project.

## Safety

`skm` validates skill names and registry names before filesystem operations. It rejects absolute paths, `..`, empty path components, unsupported agents, and unsafe registry names.

Toolkit installation scans sources without following symlinks, computes SHA-256
integrity, and refuses every unmanaged collision before writing. SKM replaces or
removes only outputs owned by the previous lockfile. A repository-local journal
backs up managed paths during apply, rolls back a partial failure, and writes the
new lockfile last. An unchanged second install is byte-idempotent.

Toolkit Workspace pins remain project-scoped. SKM validates and hashes local
standard sources before recording their integrity in a lockfile, rejecting
unsafe source paths and symlinks. Remote Workspace source pins keep their
configured revision and integrity. Adoption and migration are handled by the
installed Workspace skills. Toolkit installation writes only inside the current
repository; skill commands use their documented project or `--global` targets.

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
