#!/usr/bin/env python3
"""Meaningful success and failure cases for the release reservation gate."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from validate_versions import VersionValidator


class VersionValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.spec = self.root / "workspace/specs/test/example/feature.md"
        self.spec.parent.mkdir(parents=True)
        self.spec.write_text(
            "# Feature\n\nState: test\n\n## Version Impact\n\n"
            "| Component | Impact | Release | Rationale |\n"
            "| --- | --- | --- | --- |\n"
            "| skm | minor | skm-next | Adds a compatible skill workflow. |\n",
            encoding="utf-8",
        )
        (self.root / "Cargo.toml").write_text(
            '[package]\nname = "skm"\nversion = "0.9.0"\n', encoding="utf-8"
        )
        (self.root / "Cargo.lock").write_text(
            '[[package]]\nname = "skm"\nversion = "0.9.0"\n', encoding="utf-8"
        )
        self.ledger_path = self.root / "workspace/releases.json"
        self.ledger = {
            "schema_version": 1,
            "historical_specs": [],
            "releases": [{
                "id": "skm-next", "component": "skm", "baseline": "0.8.0",
                "target": "0.9.0", "impact": "minor", "timing": "development-start",
                "status": "applied", "owner": "skm-release", "version_source": "Cargo.toml",
                "specs": ["workspace/specs/test/example/feature.md"],
                "evidence": "Candidate version is applied to the source manifest.",
            }],
        }
        self.write_ledger()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_ledger(self) -> None:
        self.ledger_path.write_text(json.dumps(self.ledger), encoding="utf-8")

    def errors(self) -> list[str]:
        return VersionValidator(self.root).validate()

    def test_matching_applied_reservation_passes(self) -> None:
        self.assertEqual(self.errors(), [])

    def test_missing_spec_impact_fails(self) -> None:
        self.spec.write_text("# Feature\n\nState: test\n", encoding="utf-8")
        self.assertTrue(any("Version Impact section" in item for item in self.errors()))

    def test_wrong_bump_and_source_drift_fail(self) -> None:
        self.ledger["releases"][0]["target"] = "0.8.1"
        self.write_ledger()
        errors = self.errors()
        self.assertTrue(any("target does not match" in item for item in errors))
        self.assertTrue(any("source version" in item for item in errors))

    def test_member_mismatch_fails(self) -> None:
        self.ledger["releases"][0]["specs"] = ["workspace/specs/test/example/other.md"]
        self.write_ledger()
        self.assertTrue(any("member paths differ" in item for item in self.errors()))

    def test_lockfile_version_drift_fails(self) -> None:
        (self.root / "Cargo.lock").write_text(
            '[[package]]\nname = "skm"\nversion = "0.8.0"\n', encoding="utf-8"
        )
        self.assertTrue(any("lockfile version" in item for item in self.errors()))

    def test_two_open_releases_for_one_component_fail(self) -> None:
        other = self.spec.with_name("other.md")
        other.write_text(self.spec.read_text().replace("skm-next", "other-release"), encoding="utf-8")
        second = dict(self.ledger["releases"][0])
        second["id"] = "other-release"
        second["specs"] = ["workspace/specs/test/example/other.md"]
        self.ledger["releases"].append(second)
        self.write_ledger()
        self.assertTrue(any("multiple open releases" in item for item in self.errors()))

    def test_historical_done_path_must_be_frozen(self) -> None:
        done = self.root / "workspace/specs/done/example/old-feature.md"
        done.parent.mkdir(parents=True)
        done.write_text(
            "# Old Feature\n\nState: done\n\n## Version Impact\n\n"
            "| Component | Impact | Release | Rationale |\n"
            "| --- | --- | --- | --- |\n"
            "| skm | minor | historical | Previously released functionality. |\n",
            encoding="utf-8",
        )
        self.assertTrue(any("frozen historical_specs" in item for item in self.errors()))
        self.ledger["historical_specs"] = ["workspace/specs/done/example/old-feature.md"]
        self.write_ledger()
        self.assertEqual(self.errors(), [])

    def test_malformed_member_data_fails_without_crashing(self) -> None:
        self.ledger["releases"][0]["specs"] = [{"unexpected": "object"}]
        self.write_ledger()
        self.assertTrue(any("member specs" in item for item in self.errors()))


if __name__ == "__main__":
    unittest.main()
