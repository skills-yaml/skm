#!/usr/bin/env python3
"""Success and failure tests for workspace-docs@7.0.0 gates."""

from __future__ import annotations

import re
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from validate_workspace import WorkspaceValidator


class WorkspaceValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self._seed_valid_tree()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _seed_valid_tree(self) -> None:
        (self.root / "AGENTS.md").write_text(
            "# Agents\n\n"
            "<!-- AGENT-CONTEXT:START workspace-docs@7.0.0 -->\n"
            "generated\n"
            "<!-- AGENT-CONTEXT:END -->\n",
            encoding="utf-8",
        )
        (self.root / "DESIGN.md").write_text("# Design\n", encoding="utf-8")
        (self.root / "README.md").write_text("# Readme\n", encoding="utf-8")
        (self.root / "Taskfile.yml").write_text("version: '3'\n", encoding="utf-8")
        (self.root / "skills.yaml").write_text(
            "workspace:\n  standard: workspace-docs@7.0.0\n",
            encoding="utf-8",
        )
        scripts = self.root / "scripts"
        scripts.mkdir()
        (scripts / "validate_workspace.py").write_text("# validator\n", encoding="utf-8")
        (scripts / "test_validate_workspace.py").write_text("# tests\n", encoding="utf-8")

        for relative in (
            "workspace/agents/memory",
            "workspace/instructions/tech",
            "workspace/instructions/standards/workspace-docs/v7.0.0",
            "workspace/instructions/agents",
            "workspace/instructions/skills",
            "workspace/specs/backlog",
            "workspace/specs/development/sample-feature",
            "workspace/specs/test",
            "workspace/specs/done",
            "workspace/specs/blocked",
            "workspace/specs/legacy",
            "workspace/docs/architecture",
            "workspace/docs/work",
            "workspace/docs/work/multi-agent",
            "workspace/company/documents",
            "workspace/company/design",
            "workspace/company/domain",
            "workspace/company/strategy",
        ):
            (self.root / relative).mkdir(parents=True)

        standard = self.root / "workspace/instructions/standards/workspace-docs"
        (standard / "v7.0.0" / "manifest.yaml").write_text(
            'id: workspace-docs\nversion: "7.0.0"\n'
            "required_root_files:\n"
            "  - AGENTS.md\n"
            "  - DESIGN.md\n"
            "  - README.md\n"
            "required_workspace_directories:\n"
            "  - workspace/agents/memory\n"
            "  - workspace/instructions/tech\n"
            "  - workspace/instructions/standards\n"
            "  - workspace/instructions/agents\n"
            "  - workspace/instructions/skills\n"
            "  - workspace/specs/backlog\n"
            "  - workspace/specs/development\n"
            "  - workspace/specs/test\n"
            "  - workspace/specs/done\n"
            "  - workspace/specs/blocked\n"
            "  - workspace/specs/legacy\n"
            "  - workspace/docs/architecture\n"
            "  - workspace/docs/work\n"
            "  - workspace/docs/work/multi-agent\n"
            "  - workspace/company/documents\n"
            "  - workspace/company/design\n"
            "  - workspace/company/domain\n"
            "  - workspace/company/strategy\n"
            "required_spec_files:\n"
            "  - workspace/specs/README.md\n"
            "  - workspace/releases.json\n"
            "required_memory_files:\n"
            "  - workspace/agents/memory/README.md\n"
            "  - workspace/agents/memory/decisions.md\n"
            "  - workspace/agents/memory/facts.md\n"
            "  - workspace/agents/memory/preferences.md\n"
            "  - workspace/agents/memory/open-questions.md\n"
            "  - workspace/agents/memory/changelog.md\n",
            encoding="utf-8",
        )
        (standard / "default").symlink_to("v7.0.0")
        (standard / "latest").symlink_to("v7.0.0")
        (standard / "v5.0.0").mkdir()
        (standard / "v7.0.0" / "agents-template.md").write_text(
            "<!-- AGENT-CONTEXT:START workspace-docs@7.0.0 -->\n"
            "generated\n"
            "<!-- AGENT-CONTEXT:END -->\n",
            encoding="utf-8",
        )
        for name in ("sdlc.md", "process.md", "audit-checklist.md",
                     "specs-readme-template.md", "migration.md", "memory.md",
                     "versioning.md", "docs-tech-template.md"):
            (standard / "v7.0.0" / name).write_text("# Standard fixture\n", encoding="utf-8")
        (self.root / "workspace/releases.json").write_text("{}\n", encoding="utf-8")

        memory = self.root / "workspace/agents/memory"
        for name in (
            "README.md",
            "decisions.md",
            "facts.md",
            "preferences.md",
            "open-questions.md",
            "changelog.md",
        ):
            (memory / name).write_text(f"# {name}\n", encoding="utf-8")

        (self.root / "workspace/specs/README.md").write_text(
            "# Specs\n\n"
            "## Feature Categories\n\n"
            "| Primary feature | Purpose |\n"
            "| --- | --- |\n"
            "| `sample-feature` | Sample. |\n\n"
            "## Status Catalog\n\n"
            "<!-- SPEC-CATALOG:START -->\n"
            "| Spec | Primary feature | State | Status rationale |\n"
            "| --- | --- | --- | --- |\n"
            "| [example.md](development/sample-feature/example.md) | `sample-feature` | `development` | Active local work. |\n"
            "<!-- SPEC-CATALOG:END -->\n",
            encoding="utf-8",
        )
        (self.root / "workspace/specs/development/sample-feature/example.md").write_text(
            "# Example\n\n"
            "State: development\n\n"
            "## Memory Impact\n\n"
            "Status: `pending`\n\n"
            "Rationale: Still in development.\n",
            encoding="utf-8",
        )

    def test_valid_tree_passes(self) -> None:
        errors = WorkspaceValidator(self.root).validate()
        self.assertEqual(errors, [])

    def test_missing_test_state_directory_fails(self) -> None:
        shutil.rmtree(self.root / "workspace/specs/test")
        errors = WorkspaceValidator(self.root).validate(["structure", "specs"])
        self.assertTrue(any("workspace/specs/test" in error for error in errors))

    def test_spec_without_feature_directory_fails(self) -> None:
        loose = self.root / "workspace/specs/development/loose.md"
        loose.write_text("# Loose\n\nState: development\n", encoding="utf-8")
        errors = WorkspaceValidator(self.root).validate(["specs"])
        self.assertTrue(
            any("exactly one primary-feature directory" in error for error in errors)
        )

    def test_done_spec_with_pending_memory_fails(self) -> None:
        done_dir = self.root / "workspace/specs/done/sample-feature"
        done_dir.mkdir()
        (done_dir / "released.md").write_text(
            "# Released\n\n"
            "State: done\n\n"
            "## Memory Impact\n\n"
            "Status: `pending`\n\n"
            "Rationale: unfinished.\n",
            encoding="utf-8",
        )
        catalog = self.root / "workspace/specs/README.md"
        text = catalog.read_text(encoding="utf-8")
        text = text.replace(
            "<!-- SPEC-CATALOG:END -->",
            "| [released.md](done/sample-feature/released.md) | `sample-feature` | `done` | Released through main. |\n"
            "<!-- SPEC-CATALOG:END -->",
        )
        catalog.write_text(text, encoding="utf-8")
        errors = WorkspaceValidator(self.root).validate(["specs", "memory"])
        self.assertTrue(any("still pending" in error for error in errors))

    def test_missing_catalog_row_fails(self) -> None:
        extra_dir = self.root / "workspace/specs/development/sample-feature"
        (extra_dir / "second.md").write_text(
            "# Second\n\n"
            "State: development\n\n"
            "## Memory Impact\n\n"
            "Status: `none`\n\n"
            "Rationale: No durable context.\n",
            encoding="utf-8",
        )
        errors = WorkspaceValidator(self.root).validate(["specs"])
        self.assertTrue(any("missing from the root status catalog" in error for error in errors))

    def test_machine_local_path_fails(self) -> None:
        (self.root / "README.md").write_text(
            "# Readme\n\nSee /" + "home/user/secret\n",
            encoding="utf-8",
        )
        errors = WorkspaceValidator(self.root).validate(["privacy"])
        self.assertTrue(any("machine-local absolute path is forbidden" in error for error in errors))

    def test_old_project_pin_fails(self) -> None:
        (self.root / "skills.yaml").write_text(
            "workspace:\n  standard: workspace-docs@5.0.0\n", encoding="utf-8"
        )
        errors = WorkspaceValidator(self.root).validate(["structure"])
        self.assertTrue(any("must pin workspace-docs@7.0.0" in error for error in errors))

    def block_example(self, previous: str = "development") -> Path:
        original = self.root / "workspace/specs/development/sample-feature/example.md"
        blocked = self.root / "workspace/specs/blocked/sample-feature/example.md"
        blocked.parent.mkdir(parents=True)
        original.rename(blocked)
        blocked.write_text(blocked.read_text().replace(
            "State: development", f"State: blocked\nPrevious State: {previous}\n"
            "Block Kind: impediment\nBlock Reason: Required dependency is unavailable.\n"
            "Resume Condition: Dependency is restored and evidence renewed."))
        catalog = self.root / "workspace/specs/README.md"
        catalog.write_text(catalog.read_text().replace(
            "development/sample-feature/example.md", "blocked/sample-feature/example.md"
        ).replace("| `development` |", "| `blocked` |"))
        return blocked

    def test_blocked_spec_with_resume_contract_passes(self) -> None:
        self.block_example("test")
        self.assertEqual(WorkspaceValidator(self.root).validate(), [])

    def test_blocked_fields_fail_when_missing_invalid_or_duplicated(self) -> None:
        path = self.block_example()
        valid = path.read_text()
        for field, replacement in (
            ("Previous State", "Previous State: done"),
            ("Block Kind", "Block Kind: unknown"),
            ("Block Reason", "Block Reason:"),
            ("Resume Condition", "Resume Condition:"),
        ):
            with self.subTest(field=field):
                path.write_text(re.sub(rf"^{field}:.*$", replacement, valid, flags=re.MULTILINE))
                errors = WorkspaceValidator(self.root).validate(["specs"])
                self.assertTrue(any(f"valid {field}" in error for error in errors))
        path.write_text(valid + "\nPrevious State: test\n")
        self.assertTrue(any("valid Previous State" in error for error in WorkspaceValidator(self.root).validate(["specs"])))
        path.write_text(valid.replace("Block Reason: Required dependency is unavailable.\n", ""))
        self.assertTrue(any("valid Block Reason" in error for error in WorkspaceValidator(self.root).validate(["specs"])))

    def test_blocked_active_spec_retains_memory_obligation(self) -> None:
        path = self.block_example("test")
        path.write_text(path.read_text().split("## Memory Impact")[0])
        self.assertTrue(any("Memory Impact section" in error for error in WorkspaceValidator(self.root).validate(["memory"])))

    def test_blocked_backlog_does_not_gain_active_memory_obligation(self) -> None:
        path = self.block_example("backlog")
        path.write_text(path.read_text().split("## Memory Impact")[0])
        self.assertEqual(WorkspaceValidator(self.root).validate(), [])

    def test_blocked_catalog_and_path_must_agree(self) -> None:
        self.block_example()
        catalog = self.root / "workspace/specs/README.md"
        catalog.write_text(catalog.read_text().replace("| `blocked` |", "| `test` |"))
        self.assertTrue(any("catalog state must match" in error for error in WorkspaceValidator(self.root).validate(["specs"])))

    def test_missing_blocked_directory_and_package_file_fail(self) -> None:
        shutil.rmtree(self.root / "workspace/specs/blocked")
        (self.root / "workspace/instructions/standards/workspace-docs/v7.0.0/sdlc.md").unlink()
        errors = WorkspaceValidator(self.root).validate(["structure"])
        self.assertTrue(any("workspace/specs/blocked" in error for error in errors))
        self.assertTrue(any("complete pinned standard" in error for error in errors))

    def test_stale_and_absolute_aliases_fail(self) -> None:
        standard = self.root / "workspace/instructions/standards/workspace-docs"
        for target in ("v5.0.0", str(standard / "v7.0.0")):
            with self.subTest(target=target):
                (standard / "default").unlink()
                (standard / "default").symlink_to(target)
                self.assertTrue(any("contained relative link" in error for error in WorkspaceValidator(self.root).validate(["structure"])))



if __name__ == "__main__":
    unittest.main()
