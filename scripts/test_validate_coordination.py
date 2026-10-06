#!/usr/bin/env python3
"""Success and failure cases for tracked peer record validation."""

from __future__ import annotations

import sys
import json
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from validate_coordination import CoordinationValidator

BASE = "a" * 40
STAMP = "2026-09-29T12:00:00Z"


class CoordinationValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.task = self.root / "workspace/docs/work/multi-agent/sample-task"
        self.task.mkdir(parents=True)
        config = self.root / "workspace/validation/peer-commands.json"
        config.parent.mkdir(parents=True)
        config.write_text(
            json.dumps({"schema_version": 1, "commands": [["task", "check"], ["task", "test"]]}),
            encoding="utf-8",
        )
        (self.task.parent / "README.md").write_text("# Work Records\n", encoding="utf-8")
        (self.task / "README.md").write_text(
            f"---\nschema_version: 1\ncoordination_id: sample-task\nstatus: active\n"
            f"base_revision: {BASE}\nupdated_at: {STAMP}\n---\n\n"
            "# Task\n\n## Assignments\n\nAgent.\n\n## Dependencies\n\nNone.\n\n"
            "## Integration\n\nPending.\n",
            encoding="utf-8",
        )
        self.agent = self.task / "worker-one.md"
        self.agent.write_text(
            f"---\nschema_version: 1\ncoordination_id: sample-task\n"
            f"agent_id: worker-one\nrole: implementer\nstatus: active\n"
            f"base_revision: {BASE}\ntask_ref: detached\nbranch_authorization: none\n"
            f"updated_at: {STAMP}\nscope:\n  - src/main.rs\n---\n\n"
            "# Peer Work Record\n\n## Assignment\n\nWork.\n\n## Actions\n\nStarted.\n\n"
            "## Validation\n\nPending.\n\n## Blockers and Dependencies\n\nNone.\n\n"
            "## Next Step\n\nImplement.\n\n## Handoff\n\nPending.\n",
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temp.cleanup()

    def errors(self) -> list[str]:
        return CoordinationValidator(self.root).validate()

    def test_valid_records_pass(self) -> None:
        self.assertEqual(self.errors(), [])

    def test_unsafe_scope_fails(self) -> None:
        self.agent.write_text(self.agent.read_text().replace("src/main.rs", "../other"))
        self.assertTrue(any("safe repository-relative paths" in item for item in self.errors()))

    def test_attached_branch_needs_task_authority(self) -> None:
        self.agent.write_text(self.agent.read_text().replace("task_ref: detached", "task_ref: feat/example"))
        self.assertTrue(any("task-request authorization" in item for item in self.errors()))

    def test_changed_base_revision_fails(self) -> None:
        self.agent.write_text(self.agent.read_text().replace(BASE, "b" * 40))
        self.assertTrue(any("base revision must match" in item for item in self.errors()))

    def test_missing_handoff_section_fails(self) -> None:
        self.agent.write_text(self.agent.read_text().replace("## Handoff", "## Ending"))
        self.assertTrue(any("missing handoff sections" in item for item in self.errors()))

    def test_peer_config_cannot_skip_aggregate_tests(self) -> None:
        config = self.root / "workspace/validation/peer-commands.json"
        config.write_text(json.dumps({"schema_version": 1, "commands": [["task", "check"]]}))
        self.assertTrue(any("complete Taskfile check and test" in item for item in self.errors()))

    def test_attached_branch_accepts_task_authority_and_older_direct_request(self) -> None:
        original = self.agent.read_text().replace("task_ref: detached", "task_ref: feat/example")
        for authority in ("2026-10-05 task request migrate workspace", "2026-09-29 direct user request named branch"):
            with self.subTest(authority=authority):
                self.agent.write_text(original.replace("branch_authorization: none", f"branch_authorization: {authority}"))
                self.assertEqual(self.errors(), [])



if __name__ == "__main__":
    unittest.main()
