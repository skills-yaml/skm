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
