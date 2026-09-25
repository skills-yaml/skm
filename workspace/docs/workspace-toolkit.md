# Workspace Toolkits

SKM manages skills from any registry without Workspace. This document covers
the optional Workspace integration: installing a Workspace development toolkit
(versioned bundles, role profiles, and a pinned workspace-docs standard) and
adopting the Workspace Docs structure through published skills. Everything
here is opt-in; a `skills.yaml` without these fields behaves exactly as the
[README](../../README.md) describes.

## Configuration

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

## Initializing A Toolkit Project

`skm init` accepts toolkit flags in addition to its general ones:

```txt
skm init [--name <name>] [--global] [--non-interactive] [--advanced] [--toolkit-manifest <path>] [--toolkit-version <version>] [--bundle <id>] [--profile <id>] [--workspace-standard <id>] [--workspace-source <path-or-git-url>] [--workspace-revision <commit>] [--workspace-integrity <sha256>]
```

- `--toolkit-manifest <path>` selects a repository-local Workspace toolkit
  manifest, and `--toolkit-version <version>` pins its version.
- `--bundle <id>` selects a toolkit bundle and `--profile <id>` an additional
  role profile; both can be repeated.
- `--workspace-standard <id>` pins a standard such as `workspace-docs@5.0.0`.
  `--workspace-source` selects its repository-local source, or a remote one
  pinned with `--workspace-revision <commit>` and
  `--workspace-integrity <sha256>`.

## Installing

`skm install` resolves configured skills and toolkit bundles once, preflights
every target, transactionally materializes each adapter, and writes the
lockfile last. For a configured toolkit, preview every write and then apply
non-interactively:

```sh
skm install --dry-run
skm install --yes
```

If an older SKM version created a nested `skills/workspace/wk-spec` link,
review and remove that old link after confirming it points to the same source.

## Adopting Workspace Docs

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

To add every published Workspace skill at once, use the registry bundle:

```sh
skm add workspace/all-workspace-skills --source default --kind bundle --dry-run
skm add workspace/all-workspace-skills --source default --kind bundle --yes
```

## Safety

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
