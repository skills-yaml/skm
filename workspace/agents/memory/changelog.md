# Memory Changelog

## 2026-06-17 - Initialize Agent Memory

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Initialized `agents/memory/` for skm during additive adoption of `workspace-docs@1.0.0`.

## 2026-06-19 - Record diagnostic log stdout/stderr alignment

- Type: fact
- Source: review
- Confidence: high
- Review: none
- Supersedes: none

Content:

Aligned diagnostic logs and validation check outputs to stderr, keeping stdout clean for listing pipes. Updated Decisions memory.

## 2026-06-19 - Align Specifications with workspace-docs Standard

- Type: fact
- Source: review
- Confidence: high
- Review: none
- Supersedes: none

Content:

Moved implemented specifications (Auto-Update Notification, Config Management, Global Env Auto Config, Local Dev Mode, Registry Management, Skill Version Management) to `docs/specs/done/` and unimplemented specifications (Skill Removal, Cleanup Commands) to `docs/specs/backlog/` to adhere to the `workspace-docs@1.0.0` specification state directory standard.

## 2026-06-19 - Implement Skill Removal Feature

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented `skm remove` command to safely remove skill entries from `./skills.yaml` and delete symlinks from agent directories, backed by unit tests. Moved `skill-removal.md` spec to `docs/specs/done/`.

## 2026-06-19 - Implement Cleanup and Maintenance Commands (skm clean)

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented `skm clean` subcommands (`symlinks`, `cache`, `reset`) to find and remove broken or orphaned symlinks, manage cache size/retention, show cache statistics, and perform full/selective workspace resets with backup, backed by unit tests. Moved `cleanup-commands.md` spec to `docs/specs/done/`.

## 2026-06-19 - Implement Local Development Mode (skm dev)

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented `skm dev` commands (`link`, `unlink`, `list`, `show`, `mode`) to enable linking local directories as development skills, unlinking them, listing them, showing info, and toggling dev mode, backed by unit tests. Moved `local-dev-mode.md` spec to `docs/specs/done/`.

## 2026-06-19 - Implement Skill Version Management

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented skill versioning commands (`versions`, `use`, `update-skill`) to list registry versions semantically, switch config/links to a specific version, and update skills to the latest version, backed by integration tests. Moved `skill-version-management.md` spec to `docs/specs/done/`.

## 2026-08-19 - Record Workspace Docs 5 Toolkit Compatibility

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the SKM 0.2.1 compatibility decision for Workspace Docs 4.x and 5.x
toolkit manifests in `workspace/agents/memory/decisions.md`.

## 2026-08-19 - Adopt workspace-docs@5.0.0

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the `workspace-docs@5.0.0` adoption decision, branch mapping, and
canonical workspace paths in `workspace/agents/memory/decisions.md` and
`workspace/agents/memory/facts.md`.
