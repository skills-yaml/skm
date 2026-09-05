#!/usr/bin/env python3
"""Offline workspace-docs@5.0.0 gates for this repository."""

from __future__ import annotations

import re
import sys
from pathlib import Path


class WorkspaceValidator:
    MODES = ("structure", "specs", "memory", "privacy")
    MEMORY_CATEGORY_FILES = (
        "workspace/agents/memory/decisions.md",
        "workspace/agents/memory/facts.md",
        "workspace/agents/memory/preferences.md",
        "workspace/agents/memory/open-questions.md",
    )
    MEMORY_CHANGELOG = "workspace/agents/memory/changelog.md"
    SPEC_STATES = ("backlog", "development", "test", "done")
    FEATURE_PATTERN = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")
    SKIP_PARTS = {".git", "target", "scratch"}

    def __init__(self, root: Path | str) -> None:
        self.root = Path(root).resolve()
        self.errors: list[str] = []

    def validate(self, modes: list[str] | tuple[str, ...] | None = None) -> list[str]:
        selected = modes or self.MODES
        for mode in selected:
            getattr(self, f"validate_{mode}")()
        return self.errors

    def add_error(self, relative: Path | str, line: int, message: str) -> None:
        self.errors.append(f"{relative}:{line}: {message}")

    def validate_structure(self) -> None:
        manifest_path = (
            self.root
            / "workspace/instructions/standards/workspace-docs/v5.0.0/manifest.yaml"
        )
        if not manifest_path.is_file():
            self.add_error(
                "workspace/instructions/standards/workspace-docs/v5.0.0/manifest.yaml",
                1,
                "required file is missing",
            )
            return
        manifest = self.parse_manifest(manifest_path)
        for relative in manifest.get("required_root_files", []):
            if not (self.root / relative).is_file():
                self.add_error(relative, 1, "required file is missing")
        for relative in manifest.get("required_workspace_directories", []):
            if not (self.root / relative).is_dir():
                self.add_error(relative, 1, "required directory is missing")
        for relative in manifest.get("required_spec_files", []):
            if not (self.root / relative).is_file():
                self.add_error(relative, 1, "required spec file is missing")
        for relative in (
            "Taskfile.yml",
            "scripts/validate_workspace.py",
            "scripts/test_validate_workspace.py",
        ):
            if not (self.root / relative).is_file():
                self.add_error(relative, 1, "repository validation entrypoint is missing")

        agents = self.root / "AGENTS.md"
        if agents.is_file():
            content = agents.read_text(encoding="utf-8")
            start = "<!-- AGENT-CONTEXT:START workspace-docs@5.0.0 -->"
            end = "<!-- AGENT-CONTEXT:END -->"
            if content.count(start) != 1 or content.count(end) != 1:
                self.add_error(
                    "AGENTS.md",
                    1,
                    "generated context must be one balanced block pinned to workspace-docs@5.0.0",
                )
            elif content.index(end) < content.index(start):
                self.add_error("AGENTS.md", 1, "generated context markers are out of order")

        standard_root = self.root / "workspace/instructions/standards/workspace-docs"
        for name in ("default", "latest"):
            link = standard_root / name
            try:
                valid = link.is_symlink() and link.resolve(strict=True).name == "v5.0.0"
            except (OSError, RuntimeError):
                valid = False
            if not valid:
                self.add_error(
                    link.relative_to(self.root),
                    1,
                    "must resolve to v5.0.0",
                )

    def validate_specs(self) -> None:
        spec_root = self.root / "workspace/specs"
        readme = spec_root / "README.md"
        if not readme.is_file():
            self.add_error(readme.relative_to(self.root), 1, "spec catalog is missing")
            return

        content = readme.read_text(encoding="utf-8")
        start_marker = "<!-- SPEC-CATALOG:START -->"
        end_marker = "<!-- SPEC-CATALOG:END -->"
        if content.count(start_marker) != 1 or content.count(end_marker) != 1:
            self.add_error(
                readme.relative_to(self.root),
                1,
                "spec catalog must contain one balanced marker pair",
            )
            return

        start = content.index(start_marker) + len(start_marker)
        end = content.index(end_marker)
        if end < start:
            self.add_error(
                readme.relative_to(self.root),
                1,
                "spec catalog markers are out of order",
            )
            return

        feature_pattern = re.compile(
            r"^\|\s*`(?P<feature>[^`]+)`\s*\|\s*(?P<purpose>\S.*?)\s*\|\s*$",
            re.MULTILINE,
        )
        defined_features = {
            match.group("feature") for match in feature_pattern.finditer(content[:start])
        }

        row_pattern = re.compile(
            r"^\|\s*\[[^\]]+\]\((?P<path>[^)]+)\)\s*"
            r"\|\s*`(?P<feature>[^`]+)`\s*"
            r"\|\s*`(?P<state>[^`]+)`\s*"
            r"\|\s*(?P<rationale>.*?)\s*\|\s*$"
        )
        catalog: dict[str, tuple[str, str, str, int]] = {}
        catalog_body = content[start:end]
        first_catalog_line = self.line_number(content, start)
        for offset, line in enumerate(catalog_body.splitlines(), start=0):
            line_number = first_catalog_line + offset
            if not line.strip() or line.startswith("| Spec |") or line.startswith("| ---"):
                continue
            match = row_pattern.fullmatch(line)
            if not match:
                self.add_error(
                    readme.relative_to(self.root),
                    line_number,
                    "malformed spec catalog row",
                )
                continue

            relative_path = match.group("path")
            feature = match.group("feature")
            state = match.group("state")
            rationale = match.group("rationale").strip()
            if relative_path in catalog:
                self.add_error(
                    readme.relative_to(self.root),
                    line_number,
                    f"duplicate spec catalog entry: {relative_path}",
                )
                continue
            if not rationale:
                self.add_error(
                    readme.relative_to(self.root),
                    line_number,
                    f"spec catalog rationale is required: {relative_path}",
                )
            catalog[relative_path] = (feature, state, rationale, line_number)

        actual: dict[str, tuple[str, str, Path]] = {}
        for state in self.SPEC_STATES:
            state_root = spec_root / state
            if not state_root.is_dir():
                self.add_error(
                    state_root.relative_to(self.root),
                    1,
                    f"required spec state directory is missing: {state}",
                )
                continue
            for path in sorted(state_root.rglob("*.md")):
                if path.name == "README.md":
                    continue
                relative_within_state = path.relative_to(state_root)
                relative = path.relative_to(spec_root).as_posix()
                if len(relative_within_state.parts) != 2:
                    self.add_error(
                        path.relative_to(self.root),
                        1,
                        "spec must be grouped under exactly one primary-feature directory",
                    )
                    continue

                feature = relative_within_state.parts[0]
                if not self.FEATURE_PATTERN.fullmatch(feature):
                    self.add_error(
                        path.relative_to(self.root),
                        1,
                        "primary feature must use lowercase hyphen-case",
                    )
                if feature not in defined_features:
                    self.add_error(
                        path.relative_to(self.root),
                        1,
                        f"primary feature is not defined in specs README: {feature}",
                    )

                spec_content = path.read_text(encoding="utf-8")
                declared_state = re.search(
                    r"^State:\s*`?(backlog|development|test|done)`?\s*$",
                    spec_content,
                    re.MULTILINE,
                )
                if not declared_state:
                    self.add_error(
                        path.relative_to(self.root),
                        1,
                        "spec must declare State as backlog, development, test, or done",
                    )
                elif declared_state.group(1) != state:
                    self.add_error(
                        path.relative_to(self.root),
                        self.line_number(spec_content, declared_state.start()),
                        f"declared spec state must match path state {state}",
                    )
                actual[relative] = (feature, state, path)

        for relative, (feature, state, path) in actual.items():
            if relative not in catalog:
                self.add_error(
                    path.relative_to(self.root),
                    1,
                    "spec is missing from the root status catalog",
                )
                continue
            catalog_feature, catalog_state, _, line_number = catalog[relative]
            if catalog_feature != feature:
                self.add_error(
                    readme.relative_to(self.root),
                    line_number,
                    f"catalog feature must match spec path: {relative}",
                )
            if catalog_state != state:
                self.add_error(
                    readme.relative_to(self.root),
                    line_number,
                    f"catalog state must match spec path: {relative}",
                )

        for relative, (_, _, _, line_number) in catalog.items():
            if relative not in actual:
                self.add_error(
                    readme.relative_to(self.root),
                    line_number,
                    f"stale spec catalog entry: {relative}",
                )

    def validate_memory(self) -> None:
        manifest_path = (
            self.root
            / "workspace/instructions/standards/workspace-docs/v5.0.0/manifest.yaml"
        )
        if manifest_path.is_file():
            manifest = self.parse_manifest(manifest_path)
            for relative in manifest.get("required_memory_files", []):
                if not (self.root / relative).is_file():
                    self.add_error(relative, 1, "required memory file is missing")

        spec_root = self.root / "workspace/specs"
        for state in ("development", "test", "done"):
            state_root = spec_root / state
            if not state_root.is_dir():
                continue
            for path in sorted(state_root.rglob("*.md")):
                if path.name == "README.md":
                    continue
                self.validate_spec_memory_impact(path, state)

    def validate_spec_memory_impact(self, path: Path, state: str) -> None:
        content = path.read_text(encoding="utf-8")
        section_match = re.search(
            r"^## Memory Impact\s*$\n(?P<body>.*?)(?=^##\s|\Z)",
            content,
            re.MULTILINE | re.DOTALL,
        )
        relative = path.relative_to(self.root)
        if not section_match:
            self.add_error(relative, 1, "Memory Impact section is required")
            return

        body = section_match.group("body")
        status_match = re.search(
            r"^Status:\s*`?(pending|updated|none)`?\s*$", body, re.MULTILINE
        )
        if not status_match:
            self.add_error(
                relative,
                self.line_number(content, section_match.start()),
                "memory impact status must be pending, updated, or none",
            )
            return

        status = status_match.group(1)
        status_line = self.line_number(
            content, section_match.start("body") + status_match.start()
        )
        if state == "done" and status == "pending":
            self.add_error(relative, status_line, "done spec memory impact is still pending")

        rationale_match = re.search(r"^Rationale:\s*(\S.*)$", body, re.MULTILINE)
        if not rationale_match:
            self.add_error(relative, status_line, "memory impact rationale is required")

        if status != "updated":
            return

        referenced_categories = [
            memory_file for memory_file in self.MEMORY_CATEGORY_FILES if memory_file in body
        ]
        if not referenced_categories:
            self.add_error(
                relative,
                status_line,
                "updated memory impact must reference a category memory file",
            )
        if self.MEMORY_CHANGELOG not in body:
            self.add_error(
                relative,
                status_line,
                "updated memory impact must reference changelog.md",
            )

        for memory_file in (*referenced_categories, self.MEMORY_CHANGELOG):
            if memory_file in body and not (self.root / memory_file).is_file():
                self.add_error(
                    relative,
                    status_line,
                    f"referenced memory file is missing: {memory_file}",
                )

    def validate_privacy(self) -> None:
        forbidden_patterns = (
            re.compile("/" + "home/"),
            re.compile("/" + "Users/"),
            re.compile(r"[A-Za-z]:[\\/]Users[\\/]"),
        )
        for path in sorted(self.root.rglob("*")):
            if not path.is_file() or any(part in self.SKIP_PARTS for part in path.parts):
                continue
            try:
                lines = path.read_text(encoding="utf-8").splitlines()
            except UnicodeDecodeError:
                continue
            for line_number, line in enumerate(lines, start=1):
                if any(pattern.search(line) for pattern in forbidden_patterns):
                    self.add_error(
                        path.relative_to(self.root),
                        line_number,
                        "machine-local absolute path is forbidden",
                    )

        inventory_root = self.root / "workspace/docs/projects"
        if inventory_root.exists():
            self.add_error(
                inventory_root.relative_to(self.root),
                1,
                "central cross-project inventories are forbidden",
            )

    @staticmethod
    def parse_manifest(path: Path) -> dict[str, str | list[str]]:
        result: dict[str, str | list[str]] = {}
        current_list: str | None = None
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            top_level = re.fullmatch(r"([A-Za-z0-9_]+):(?:\s*(.*))?", line)
            if top_level:
                key, raw_value = top_level.groups()
                value = (raw_value or "").strip().strip("\"'")
                if value:
                    result[key] = value
                    current_list = None
                else:
                    result[key] = []
                    current_list = key
                continue
            list_item = re.fullmatch(r"  -\s+(.+)", line)
            if list_item and current_list:
                value = list_item.group(1).strip().strip("\"'")
                current_value = result[current_list]
                if isinstance(current_value, list):
                    current_value.append(value)
            elif line and not line.startswith("  -"):
                current_list = None
        return result

    @staticmethod
    def line_number(content: str, index: int) -> int:
        return content.count("\n", 0, index) + 1


def main(argv: list[str] | None = None) -> int:
    args = argv if argv is not None else sys.argv[1:]
    root = Path.cwd()
    modes = args or list(WorkspaceValidator.MODES)
    unknown = [mode for mode in modes if mode not in WorkspaceValidator.MODES]
    if unknown:
        print(f"unknown validation mode: {', '.join(unknown)}", file=sys.stderr)
        return 2
    validator = WorkspaceValidator(root)
    errors = validator.validate(modes)
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        print(f"{len(errors)} workspace validation error(s)", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
