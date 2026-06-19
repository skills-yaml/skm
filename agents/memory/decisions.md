# Decisions

## 2026-06-17 - Adopt workspace-docs@1.0.0

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

skm adopts the workspace documentation standard at `workspace-docs@1.0.0`. Adoption is additive: preserve project-specific guidance and legacy specs, and keep generated agent context inside `AGENT-CONTEXT` markers.

## 2026-06-19 - Standardize Diagnostic Logs to Stderr

- Type: decision
- Source: review
- Confidence: high
- Review: none
- Supersedes: none

Content:

Align skm CLI command outputs with the UI brand and style guide. Diagnostic outputs, installation logs, cache warnings, and interactive confirmations are sent to standard error (stderr). Standard output (stdout) is reserved strictly for successful query outputs meant for piping, such as listing details in `skm list`.

## 2026-06-19 - Implement Skill Removal Feature

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented `skm remove <SKILL_NAME>` command to safely unlink skill directories from configured agent paths and programmatically update `./skills.yaml` config entries.
