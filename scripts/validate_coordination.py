#!/usr/bin/env python3
"""Validate tracked Workspace Docs 6 peer records without reading local board state."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ID = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")
REVISION = re.compile(r"[0-9a-f]{40}")
STAMP = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z")
REQUIRED_HEADINGS = (
    "Assignment",
    "Actions",
    "Validation",
    "Blockers and Dependencies",
    "Next Step",
    "Handoff",
)


class CoordinationValidator:
    def __init__(self, root: Path | str) -> None:
        self.root = Path(root).resolve()
        self.errors: list[str] = []

    def error(self, path: Path, message: str) -> None:
        self.errors.append(f"{path.relative_to(self.root)}: {message}")

    @staticmethod
    def frontmatter(path: Path) -> tuple[dict[str, str], list[str], str] | None:
        try:
            content = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError):
            return None
        if not content.startswith("---\n"):
            return None
        parts = content.split("---\n", 2)
        if len(parts) != 3:
            return None
        fields: dict[str, str] = {}
        scope: list[str] = []
        in_scope = False
        for line in parts[1].splitlines():
            if line == "scope:":
                in_scope = True
                continue
            if in_scope and line.startswith("  - "):
                scope.append(line[4:])
                continue
            in_scope = False
            if ": " in line:
                key, value = line.split(": ", 1)
                if key in fields:
                    return None
                fields[key] = value
        return fields, scope, parts[2]

    @staticmethod
    def safe_scope(value: str) -> bool:
        if not value or len(value) > 256 or value != value.strip() or any(char in value for char in "\\:*?[\x00"):
            return False
        if value.startswith("workspace/docs/work/multi-agent"):
            return False
        return all(part not in ("", ".", "..", ".git") for part in value.split("/"))

    def validate(self) -> list[str]:
        config_path = self.root / "workspace/validation/peer-commands.json"
        try:
            config = json.loads(config_path.read_text(encoding="utf-8"))
        except (OSError, ValueError, UnicodeError):
            config = None
        if config != {"schema_version": 1, "commands": [["task", "check"], ["task", "test"]]}:
            self.error(config_path, "peer validation must use the complete Taskfile check and test gates")
        base = self.root / "workspace/docs/work/multi-agent"
        if not base.is_dir() or base.is_symlink():
            self.error(base, "record directory is missing or symlinked")
            return self.errors
        if not (base / "README.md").is_file():
            self.error(base / "README.md", "record directory README is required")
        for task in sorted(base.iterdir()):
            if task.name == "README.md":
                continue
            if task.is_symlink() or not task.is_dir() or not ID.fullmatch(task.name):
                self.error(task, "task directory needs a safe lowercase identifier")
                continue
            index = task / "README.md"
            if not index.is_file() or index.is_symlink():
                self.error(index, "task index is missing or symlinked")
                continue
            parsed = self.frontmatter(index)
            if parsed is None:
                self.error(index, "task index needs valid frontmatter")
                continue
            fields, _, body = parsed
            if fields.get("schema_version") != "1" or fields.get("coordination_id") != task.name:
                self.error(index, "task index schema or coordination ID mismatch")
            if fields.get("status") not in ("active", "complete"):
                self.error(index, "task index status must be active or complete")
            if not REVISION.fullmatch(fields.get("base_revision", "")):
                self.error(index, "task index needs a full base revision")
            index_base = fields.get("base_revision")
            if not STAMP.fullmatch(fields.get("updated_at", "")):
                self.error(index, "task index needs a UTC updated_at timestamp")
            if any(f"## {heading}" not in body for heading in ("Assignments", "Dependencies", "Integration")):
                self.error(index, "task index is missing handoff sections")
            agents = [path for path in task.glob("*.md") if path.name != "README.md"]
            if not agents:
                self.error(task, "task requires at least one agent record")
            for agent in agents:
                if agent.is_symlink() or not ID.fullmatch(agent.stem):
                    self.error(agent, "agent record filename is unsafe")
                    continue
                parsed = self.frontmatter(agent)
                if parsed is None:
                    self.error(agent, "agent record needs valid frontmatter")
                    continue
                fields, scope, body = parsed
                if fields.get("schema_version") != "1" or fields.get("coordination_id") != task.name or fields.get("agent_id") != agent.stem:
                    self.error(agent, "agent record schema or identity mismatch")
                if fields.get("role") not in ("implementer", "reviewer") or fields.get("status") not in ("active", "handoff", "complete"):
                    self.error(agent, "agent role or status is invalid")
                if fields.get("base_revision") != index_base or not REVISION.fullmatch(fields.get("base_revision", "")):
                    self.error(agent, "agent base revision must match the task index")
                if not STAMP.fullmatch(fields.get("updated_at", "")):
                    self.error(agent, "agent record needs a UTC updated_at timestamp")
                task_ref = fields.get("task_ref")
                authorization = fields.get("branch_authorization")
                if task_ref == "detached":
                    if authorization != "none":
                        self.error(agent, "detached record must have no branch authorization")
                elif not task_ref or not authorization or not re.match(r"^\d{4}-\d{2}-\d{2} direct user request\b", authorization):
                    self.error(agent, "attached task branch requires dated direct-user authorization")
                if not scope or any(not self.safe_scope(item) for item in scope):
                    self.error(agent, "agent scope needs safe repository-relative paths")
                if any(f"## {heading}" not in body for heading in REQUIRED_HEADINGS):
                    self.error(agent, "agent record is missing handoff sections")
                if re.search(r"/(?:home|Users)/|[A-Za-z]:[\\/]Users[\\/]", agent.read_text(encoding="utf-8")):
                    self.error(agent, "machine-local path is forbidden")
        return self.errors


def main() -> int:
    errors = CoordinationValidator(Path.cwd()).validate()
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        print(f"{len(errors)} coordination validation error(s)", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
