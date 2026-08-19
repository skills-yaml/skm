# Workspace Documentation Standard

This directory contains the versioned documentation and agent-process standard
for repositories that adopt this workspace structure.

The standard defines:

- required project documentation files & root container structure (`workspace/agents/`, `workspace/instructions/`, `workspace/specs/`, `workspace/docs/`, `workspace/company/`)
- root policy and token rules (`AGENTS.md`, `DESIGN.md`)
- spec-driven development states and transitions
- primary-feature grouping and a canonical spec status-rationale catalog
- generated versus static documentation boundaries
- durable agent memory storage rules
- audit criteria and migration guidance for agents
  ([`AGENT_MIGRATION.md`](./AGENT_MIGRATION.md))

## Current Versions

- `latest` points to `v5.0.0` (newest standard version).
- `default` points to `v5.0.0` (recommended stable version).
- `v4.0.0` / `v3.0.0` / `v2.1.0` / `v2.0.0` / `v1.2.0` /
  `v1.1.0` / `v1.0.0` legacy
  versions are preserved for backward compatibility.

Projects should pin to a concrete version when reproducibility matters:

```text
workspace-docs@5.0.0
```

Projects may use `workspace-docs@default` while adopting the shared process.

## Directory Layout

```text
workspace-docs/
  AGENT_MIGRATION.md
  README.md
  VERSIONING.md
  latest -> v5.0.0
  default -> v5.0.0
  v5.0.0/
    manifest.yaml
    agents-template.md
    docs-tech-template.md
    process.md
    sdlc.md
    memory.md
    specs-readme-template.md
    audit-checklist.md
    migration.md
```

## Adoption & Structural Separation Model

Generated or synchronized agent context must live inside clear markers:

```md
<!-- AGENT-CONTEXT:START workspace-docs@5.0.0 -->
<!-- Generated content. Manual edits may be overwritten. -->
<!-- AGENT-CONTEXT:END -->
```

Manual project rules must stay outside generated blocks.

Agents performing a first-time adoption, version update, or structural repair
must start with the [Agent Migration Guide](./AGENT_MIGRATION.md), then apply the
target version's migration notes and audit checklist.

Version 5.0 preserves feature grouping, the root status catalog, and memory
impact, and adds an explicit test state. Specifications move from development
to test only after shared test integration and from test to done only after
production release. Conventional branches are `develop` and `main`, but
repositories may document equivalents.

When the repository includes the
[`adopt-workspace-structure` skill](../../skills/adopt-workspace-structure/SKILL.md),
invoke it to manage this workflow from discovery through validated completion.
