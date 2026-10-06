# Local Toolkits

SKM can install repository-local bundles of skills and role profiles using its
[installation format](registry-format.md). Toolkit support is independent of
publisher identity and is optional for projects that only install registry skills.

```yaml
name: my-project
agents: [codex, cursor]
skills: []
toolkit:
  manifest: toolkit/manifest.yaml
  version: 0.3.0
bundles: [development-core]
profiles: [security-reviewer]
```

Toolkit source paths must be repository-relative and must not contain symlinks.
The lockfile records selected package/profile versions and integrity, adapter
versions/capabilities, and managed outputs. Codex profiles use native custom-agent
TOML under `.codex/agents/`; Cursor profiles use generated skill fallbacks under
`.cursor/skills/`.

Initialize a project with generic toolkit options:

```sh
skm init --non-interactive --toolkit-manifest toolkit/manifest.yaml --toolkit-version 0.3.0 --bundle development-core --profile security-reviewer
skm install --dry-run
skm install --yes
skm check
```

`--bundle` and `--profile` can be repeated and require `--toolkit-manifest`.
Toolkit initialization/installation is project-scoped. Installation computes
source integrity, preflights collisions, changes only owned outputs, rolls back
partial failures, and writes the lock last. Repeating an unchanged installation
preserves lock bytes.

Unknown root-level configuration and existing lock values are opaque metadata.
SKM does not expose domain-specific init flags or hash domain standard sources.
Legacy `workspace` and `trusted_sources` values remain readable and preserved;
installed skills manage their meaning. The retired `--workspace-standard`,
`--workspace-source`, `--workspace-revision`, and `--workspace-integrity` flags
are no longer accepted. New locks do not acquire domain metadata from config.
