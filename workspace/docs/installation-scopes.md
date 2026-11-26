# Project-local and global skills

Use project-local skills for a repository's shared requirements and global skills
for a user's reusable tools across repositories. Both scopes can use the same
registry cache.

## Choose a scope

| Concern | Project-local | Global |
| --- | --- | --- |
| Install command | `skm install --yes` | `skm install --global --yes` |
| Links | Agent directories inside the project | Agent directories under the current user's home |
| Best fit | Team workflows and repository-specific skills | Personal baseline tools across repositories |
| Manifest | Project `skills.yaml` | `~/.skm/skills.yaml` |
| Lockfile | Writes project `skills.lock.yaml` after linking succeeds | Does not write a project lockfile |
| Versions | Prefer exact pins for reviewed team behavior | Exact pins or `latest`, according to update policy |

Global means one operating-system user, not every user on the machine. Run SKM
as the intended user; running as root targets root's home. Each machine needs its
own installation.

`--global` selects the user manifest and user-level agent directories from any
working directory. Base settings and registries live in `~/.skm/config.yaml`;
the user skill selection is a separate `~/.skm/skills.yaml`. The cache under
`~/.skm/cache` is shared with project installations.

## Project-local example

Create this `skills.yaml` at the repository root:

```yaml
name: example-project
registries:
  default: https://github.com/skills-yaml/registry.git
agents: [claude, codex, copilot, cursor]
skills:
  - name: workspace/write-spec
    version: 0.5.1
    source: default
```

From that directory:

```sh
skm cache refresh
skm install --dry-run
skm install --yes
skm check
```

Review and commit the manifest and generated lockfile according to repository
policy. Contributors install the declared skills in their own checkouts. Exact
pins keep new registry publications from changing a team's workflow without a
reviewed manifest change.

## Global example

As the intended user, run `skm init --global` to select agents and skills, or
edit `~/.skm/skills.yaml` directly. For example:

```yaml
name: Global skills
registries:
  default: https://github.com/skills-yaml/registry.git
agents: [claude, codex, copilot, cursor, antigravity, gemini-cli, grok, hermes]
skills:
  - name: workspace/write-spec
    version: latest
    source: default
```

Preview, install, and verify:

```sh
(cd "$HOME/.skm" && skm cache refresh)
skm install --global --dry-run
skm install --global --yes
skm check --global
```

Supported agents discover links in this user's global skill locations. Hermes has
only a global SKM target. See the README's [link targets](../../README.md#link-targets)
for each agent's directories and discovery requirements.

Published bundles currently support project installation only: `skm add <bundle>
--global` is rejected. For global use, declare the desired bundle members as
individual skills in the user manifest, then run `skm install --global`.
Installing Workspace skills makes them discoverable; it does not adopt a Workspace
documentation standard or rewrite repository instructions.

## Use both together

A useful combination is a global baseline for analysis and common maintenance,
plus project-local skills pinned to reviewed versions for team workflows. Avoid
same-name skills in both scopes when different versions would be confusing.

SKM manages links; each agent controls discovery and precedence. Do not assume a
universal project-over-global rule. Check the agent's behavior before relying on
the same skill name in both locations. SKM replaces existing symlinks but refuses
to overwrite real files or directories at its targets.

## Refresh and update

`skm cache refresh` uses the working directory manifest for registry overrides,
so run it in `~/.skm` when refreshing the global manifest’s registries. A cache
refresh fetches registry contents. It does not itself re-link installed
skills or change exact pins. For a global manifest following `latest`, run:

```sh
(cd "$HOME/.skm" && skm cache refresh) && skm install --global --yes && skm check --global
```

A daily scheduler can run that sequence as the intended user, with an explicit executable path. Registry credentials must work without an
interactive prompt. Following `latest` automatically changes available skill
instructions; use trusted registries and choose this policy deliberately.

For exact pins, preview version changes before applying:

```sh
(cd "$HOME/.skm" && skm cache refresh)
skm skill upgrade --all --global --dry-run
skm skill upgrade --all --global --yes
skm install --global --yes
skm check --global
```

For project skills, run these commands at the project root and omit `--global`.
Caches are shared within a user account, so clearing a registry can affect links
in both scopes. Keep both manifests when moving machines and regenerate links on
each destination.

## Migrating from older global commands

Older versions read `skills.yaml` from the working directory even with
`--global`. Review the old dedicated manifest, including its selected agents
and relative paths. Back up any existing `~/.skm/skills.yaml`, copy or merge
the reviewed selections into it, and adjust relative paths to be relative to
`~/.skm` (or use absolute paths). Then run `skm install --global --dry-run`,
`skm install --global --yes`, and `skm check --global`. Existing project
manifest files are not automatically imported.

Before downgrading, export `~/.skm/skills.yaml` to a dedicated directory,
adjust relative paths for that directory, and run the older binary's global
install from there. Downgrading does not synchronize later changes back.
Retained legacy settings and caches can aid recovery; cache clearing can break
links until installation is repeated. No release publication is needed for this
migration.
