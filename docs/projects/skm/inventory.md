# skm Workspace Docs Inventory

Metadata:

- Adopted standard: workspace-docs@1.0.0
- Status: adoption inventory
- Owner: project
- Last reviewed: 2026-06-17

## Adopted Files

- `AGENTS.md`
- `README.md`
- `docs/tech/task.md`
- `docs/tech/sdlc.md`
- `docs/tech/project_structure.md`
- `docs/tech/ci.md`
- `docs/specs/README.md`
- `docs/specs/backlog/`
- `docs/specs/development/`
- `docs/specs/done/`
- `docs/standards/workspace-docs/README.md`
- `agents/memory/README.md`
- `agents/memory/decisions.md`
- `agents/memory/facts.md`
- `agents/memory/preferences.md`
- `agents/memory/open-questions.md`
- `agents/memory/changelog.md`

## Missing / Known Gaps

- None recorded.

## Legacy Spec Paths

- `docs/specs/foundation/`

## Quality Gates Available

- `task --list`
- `task check`
- `task test`

## Validation Run

- `git status --short` before edits: run; worktree clean.
- `task --list`: passed.
- `task check`: passed.
- `task test`: passed; 4 tests passed.

## Notes

- Existing project-specific manual instructions remain outside generated `AGENT-CONTEXT` markers.
- Legacy specs were preserved in place.
- No secrets or environment-specific credential values were added.
