# Specification: Agent Skill Directory Integration

## Status

State: development

Implementation and local validation are complete on
`feat/backlog-specs-implementation`. The spec remains in `development` until
the branch is integrated into the configured `development` test target.

## Overview

SKM links skills into one directory per agent, derived from the agent's name.
Two of the six mappings do not match any path the agent documents, and the
model cannot express the paths several widely used agents actually read.

Meanwhile the ecosystem converged. `.agents/skills` is now read by Codex,
Copilot, Cursor, Antigravity, Pi, OpenCode, Kilo Code, Gemini CLI, Goose, Crush,
OpenHands and Roo Code. Several vendors describe their own directory as the
legacy one.

This specification corrects the incorrect targets, replaces the per-agent
directory-name model with explicit paths, and extends support from six agents to
sixteen.

## Problem Statement

`src/linker.rs` derives every target from a single directory name per agent:

```rust
"codex" => ".codex",              // get_global_agent_skills_dir
"codex" => ".codex/skills",       // get_project_agent_skills_dir
```

Three defects follow.

**P1. Codex never reads `.codex/skills`.** Codex documents `.agents/skills`
walked from the working directory to the repository root, `$HOME/.agents/skills`,
and `/etc/codex/skills`. `~/.codex/` holds `config.toml` only. Every Codex link
SKM writes today is inert.

This defect hides itself: Cursor reads `.codex/skills` and `~/.codex/skills` for
compatibility, so a user running both sees the skills load and concludes Codex
found them.

**P2. Hermes has no project-level skill directory.** Hermes documents
`~/.hermes/skills/` as "the primary directory and source of truth", plus
`external_dirs` in `~/.hermes/config.yaml`. The project-level `.hermes/skills`
SKM creates is never scanned. The global target is correct.

**P3. The global model cannot express real paths.** `get_global_agent_skills_dir`
builds `$HOME/<name>/skills` from one path segment. Codex needs
`~/.agents/skills`, OpenCode `~/.config/opencode/skills`, Pi
`~/.pi/agent/skills`, and Antigravity `~/.gemini/config/skills` — none of which
that shape can produce.

## Scope

- Replace the directory-name mapping with an explicit path list per agent per
  scope.
- Deduplicate targets, because many agents resolve to the same directory.
- Correct `codex`, remove project-level `hermes`.
- Add ten agents: `antigravity`, `pi`, `opencode`, `cline`, `kilo`,
  `gemini-cli`, `goose`, `crush`, `openhands`, `qwen`.
- Migrate installs holding links at paths this change abandons.

### Non-Goals

- Changing skill resolution, registry behavior, or the lockfile's purpose.
- Writing outside the repository in project scope, or outside the agent
  directories in global scope.
- Supporting agents with no skills concept. Continue uses ordered rules under
  `.continue/rules/` and Aider uses read-only conventions files; neither has a
  directory SKM can link into.
- Reading skill directory paths from a registry. Rejected under R7.

## Requirements

### R1: Explicit paths per agent and scope

Each supported agent declares a list of project-relative paths and a list of
home-relative paths. Both lists may hold more than one entry and either may be
empty. An agent with no project path, such as Hermes, is valid and must not
produce a project-scope target.

### R2: Target deduplication

Several agents resolve to the same directory. `.agents/skills` alone is claimed
by eleven. A directory claimed by more than one enabled agent must produce one
link, one preflight check, and one lockfile entry, recording every agent that
claimed it. `skm list` reports the directory once and names its claimants.

### R3: The agent table

The paths below are taken from each vendor's own documentation. Where an agent
reads another vendor's directory for compatibility, SKM does not use that path
as a write target: SKM writes each agent's own documented location and lets
compatibility reads find it.

| Agent | Project paths | Global paths |
| --- | --- | --- |
| `claude` | `.claude/skills` | `~/.claude/skills` |
| `codex` | `.agents/skills` | `~/.agents/skills` |
| `copilot` | `.github/skills` | `~/.copilot/skills` |
| `cursor` | `.cursor/skills` | `~/.cursor/skills` |
| `antigravity` | `.agents/skills` | `~/.gemini/config/skills` |
| `pi` | `.pi/skills` | `~/.pi/agent/skills` |
| `opencode` | `.opencode/skills` | `~/.config/opencode/skills` |
| `cline` | `.cline/skills` | `~/.cline/skills` |
| `kilo` | `.kilo/skills` | `~/.kilo/skills` |
| `gemini-cli` | `.gemini/skills` | `~/.gemini/skills` |
| `goose` | `.agents/skills` | `~/.agents/skills` |
| `crush` | `.crush/skills` | `~/.config/crush/skills` |
| `openhands` | `.agents/skills` | `~/.openhands/skills` |
| `grok` | `.grok/skills` | `~/.grok/skills` |
| `qwen` | `.qwen/skills` | `~/.qwen/skills` |
| `hermes` | *(none)* | `~/.hermes/skills` |

Notes carried from the source documentation:

- Antigravity's global path is `~/.gemini/config/skills`, which is not Gemini
  CLI's `~/.gemini/skills`. The two products share a vendor directory and differ
  inside it.
- Qwen Code diverged from the Gemini CLI it forked: it documents `.qwen/skills`
  and `~/.qwen/skills` and no `.agents/skills`.
- Codex, Pi and Grok walk parent directories up to the repository root. SKM
  writes at the project root, which those walks reach.
- Pi loads project skills only after the project is trusted. SKM creating the
  link is necessary but not sufficient; the user still trusts the project in Pi.
- Crush also reads XDG and Windows-specific locations and honors
  `$CRUSH_SKILLS_DIR`. SKM writes the documented default only.

### R4: Migration of abandoned targets

`skm install` must remove links this change abandons when the previous lockfile
records SKM as their owner: project and global `.codex/skills`, and project
`.hermes/skills`. Unmanaged files at those paths are left untouched and
reported. A user who has never run a version writing those paths sees nothing.

### R5: Configuration compatibility

Existing manifests must keep working. `agents: [claude, codex, cursor, copilot,
grok, hermes]` stays valid and changes only where a target moves. The `init`
wizard offers the full agent list. An unrecognized agent name is an error that
names the supported set, as today.

### R6: Documentation

`README.md`'s "Link Targets" section is replaced by the R3 table with both
scopes, a note that one directory can serve several agents, and the Hermes
`external_dirs` alternative for users who want a shared directory.

### R7: Skill directory paths stay in SKM

Paths are compiled into SKM, not read from a registry. A registry is remote
content; letting it name filesystem write targets would let a compromised
registry redirect writes, and SKM must link before any registry is available.
The registry describes skills; SKM decides where they land.

## Acceptance Criteria

- Each agent in R3 resolves to exactly its listed paths in both scopes.
- `hermes` produces no project target; `skm install` does not create
  `.hermes/skills`.
- With `codex`, `goose`, `openhands` and `antigravity` all enabled, project
  scope contains one `.agents/skills` link, the preflight checks it once, and
  the lockfile holds one entry naming all four agents.
- `skm install` removes SKM-owned links at the abandoned paths in R4 and leaves
  unmanaged files in place, reporting them.
- A second `skm install` with no manifest change is byte-idempotent, including
  after a migration.
- `skm check` passes against a project using all sixteen agents.
- `skm list` reports each deduplicated directory once with its claimants.
- Manifests naming only the original six agents install without edits.

## Affected Areas

- `src/linker.rs` — path tables, deduplication, both scope resolvers
- `src/cleaner.rs` — removal of abandoned targets
- `src/config.rs`, `src/wizard/` — agent list and validation
- `src/dev.rs` — development links use the same resolver
- lockfile writer — one entry per directory, claimants recorded
- `README.md` — Link Targets
- `workspace/specs/README.md` — catalog and the new `agents` feature category

## Validation Gates

- `task check`
- `task test`, including per-agent resolution, deduplication, migration
  behavior, idempotency, and rejection of unknown agents
- `task build`
- `task test:tui` for the agent selection step
- `git diff --check`

## Memory Impact

Status: updated

Rationale: `workspace/agents/memory/decisions.md` records that skill-directory
paths are compiled into SKM and shared targets are processed once with all
claimants retained. `workspace/agents/memory/facts.md` records the sixteen-agent
set and the corrected Codex and Hermes targets. Corresponding entries were
appended to `workspace/agents/memory/changelog.md`.

## Implementation Evidence

Completed locally on 2026-09-22 in `feat/backlog-specs-implementation`:

- Explicit project and global path tables cover all sixteen agents; Hermes has
  no project target.
- Link, unlink, list, check, cleanup, version preview, local-development, and
  toolkit flows use the shared deduplicating resolver.
- Toolkit lock outputs record every claimant for a shared target, while legacy
  singular lock outputs remain readable for safe migration.
- SKM-owned legacy Codex and Hermes project outputs migrate safely; unmanaged
  content at abandoned paths is preserved.
- `task check`, `task test`, `task build`, `task test:init`, and
  `git diff --check` passed. The Rust suite passed 129 tests, and workspace
  validation passed all six tests.

## Evidence

Paths in R3 were read from vendor documentation on 2026-09-21:

- Claude Code: https://code.claude.com/docs/en/skills
- Codex: https://learn.chatgpt.com/docs/build-skills.md
- Copilot: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills
- Cursor: https://cursor.com/docs/skills
- Antigravity: https://antigravity.google/docs/skills/
- Pi: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/skills.md
- OpenCode: https://opencode.ai/docs/skills/
- Cline: https://docs.cline.bot/customization/skills
- Kilo Code: https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-docs/pages/customize/skills.md
- Gemini CLI: https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/skills.md
- Goose: https://github.com/block/goose/blob/main/documentation/docs/guides/context-engineering/using-skills.md
- Crush: https://github.com/charmbracelet/crush#agent-skills
- OpenHands: https://github.com/OpenHands/docs/blob/main/overview/skills/adding.mdx
- Grok: https://docs.x.ai/build/features/skills-plugins-marketplaces
- Qwen Code: https://github.com/QwenLM/qwen-code/blob/main/docs/users/features/skills.md
- Hermes: https://github.com/NousResearch/hermes-agent/blob/main/website/docs/user-guide/features/skills.md
