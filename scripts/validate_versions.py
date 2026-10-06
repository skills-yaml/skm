#!/usr/bin/env python3
"""Validate Workspace Docs 7 spec impact and SKM release reservations."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ID = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")
SEMVER = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")
IMPACT = {"none": 0, "patch": 1, "minor": 2, "major": 3}
STATES = ("backlog", "development", "test", "done", "blocked")


class VersionValidator:
    def __init__(self, root: Path | str) -> None:
        self.root = Path(root).resolve()
        self.errors: list[str] = []

    def error(self, path: str, message: str) -> None:
        self.errors.append(f"{path}: {message}")

    def relative_file(self, value: object) -> Path | None:
        if not isinstance(value, str) or not value or "\\" in value:
            return None
        path = Path(value)
        if path.is_absolute() or any(part in ("", ".", "..", ".git") for part in value.split("/")):
            return None
        try:
            if not (self.root / path).resolve().is_relative_to(self.root):
                return None
        except (OSError, RuntimeError):
            return None
        return path

    def spec_rows(self, relative: str) -> list[tuple[str, str, str]]:
        content = (self.root / relative).read_text(encoding="utf-8")
        sections = re.findall(
            r"^## Version Impact\s*$\n(.*?)(?=^##\s|\Z)",
            content,
            re.MULTILINE | re.DOTALL,
        )
        if len(sections) != 1:
            self.error(relative, "exactly one Version Impact section is required")
            return []
        lines = [line.strip() for line in sections[0].splitlines() if line.strip()]
        expected = "| Component | Impact | Release | Rationale |"
        if len(lines) < 3 or lines[0] != expected or not re.fullmatch(r"\|(?:\s*:?-{3,}:?\s*\|){4}", lines[1]):
            self.error(relative, "Version Impact needs Component, Impact, Release, and Rationale table columns")
            return []
        rows: list[tuple[str, str, str]] = []
        for line in lines[2:]:
            fields = [field.strip() for field in line.strip("|").split("|")]
            if len(fields) != 4 or not all(fields):
                self.error(relative, "Version Impact row must have four non-empty cells")
                continue
            component, impact, release, rationale = fields
            if not ID.fullmatch(component):
                self.error(relative, f"invalid component: {component}")
            if impact not in IMPACT:
                self.error(relative, f"invalid impact: {impact}")
            if release not in ("none", "historical") and not ID.fullmatch(release):
                self.error(relative, f"invalid release: {release}")
            if len(rationale) < 12:
                self.error(relative, "Version Impact rationale is too short")
            rows.append((component, impact, release))
        if not rows:
            self.error(relative, "Version Impact requires at least one row")
        if len({component for component, _, _ in rows}) != len(rows):
            self.error(relative, "Version Impact repeats a component")
        return rows

    @staticmethod
    def bumped(baseline: tuple[int, int, int], impact: str) -> tuple[int, int, int]:
        major, minor, patch = baseline
        if impact == "major":
            return (major + 1, 0, 0)
        if impact == "minor":
            return (major, minor + 1, 0)
        return (major, minor, patch + 1)

    def source_version(self, relative: str) -> str | None:
        path = self.relative_file(relative)
        if path is None or not (self.root / path).is_file():
            self.error("workspace/releases.json", f"invalid version source: {relative}")
            return None
        if path.suffix != ".toml":
            self.error("workspace/releases.json", f"unsupported version source: {relative}")
            return None
        try:
            manifest = (self.root / path).read_text(encoding="utf-8")
            package = re.search(r"(?ms)^\[package\]\s*\n(.*?)(?=^\[|\Z)", manifest)
            match = re.search(r'(?m)^version\s*=\s*"([^"]+)"\s*$', package.group(1)) if package else None
            version = match.group(1) if match else None
        except (OSError, UnicodeError):
            version = None
        if not isinstance(version, str) or not SEMVER.fullmatch(version):
            self.error(relative, "package version must be a normal X.Y.Z value")
            return None
        if path == Path("Cargo.toml"):
            try:
                lock = (self.root / "Cargo.lock").read_text(encoding="utf-8")
                blocks = re.findall(r"(?ms)^\[\[package\]\]\s*\n(.*?)(?=^\[\[package\]\]|\Z)", lock)
                mirrors = []
                for block in blocks:
                    if re.search(r'(?m)^name\s*=\s*"skm"\s*$', block):
                        mirror = re.search(r'(?m)^version\s*=\s*"([^"]+)"\s*$', block)
                        mirrors.append(mirror.group(1) if mirror else None)
            except (OSError, UnicodeError):
                mirrors = []
            if mirrors != [version]:
                self.error("Cargo.lock", "SKM lockfile version must match Cargo.toml")
        return version

    def validate(self) -> list[str]:
        ledger_path = self.root / "workspace/releases.json"
        try:
            ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
        except (OSError, ValueError, UnicodeError):
            self.error("workspace/releases.json", "missing or invalid JSON release ledger")
            return self.errors
        if not isinstance(ledger, dict) or ledger.get("schema_version") != 1:
            self.error("workspace/releases.json", "schema_version must be 1")
            return self.errors
        historical = ledger.get("historical_specs")
        releases = ledger.get("releases")
        if not isinstance(historical, list) or not isinstance(releases, list):
            self.error("workspace/releases.json", "historical_specs and releases must be lists")
            return self.errors
        if len(historical) != len(set(map(str, historical))):
            self.error("workspace/releases.json", "duplicate historical spec path")
        historical_set = set(item for item in historical if isinstance(item, str))
        spec_rows: dict[str, list[tuple[str, str, str]]] = {}
        states: dict[str, str] = {}
        effective_states: dict[str, str] = {}
        for state in STATES:
            for path in sorted((self.root / "workspace/specs" / state).rglob("*.md")):
                if path.name == "README.md":
                    continue
                relative = path.relative_to(self.root).as_posix()
                states[relative] = state
                effective_states[relative] = state
                if state == "blocked":
                    values = re.findall(r"^Previous State:[ \t]*(.*)$", path.read_text(encoding="utf-8"), re.MULTILINE)
                    previous = values[0].strip().strip("`") if len(values) == 1 else ""
                    if previous not in ("backlog", "development", "test"):
                        self.error(relative, "blocked spec needs one valid Previous State")
                    else:
                        effective_states[relative] = previous
                spec_rows[relative] = self.spec_rows(relative)
                if state == "done":
                    if relative not in historical_set and any(release == "historical" for _, _, release in spec_rows[relative]):
                        self.error(relative, "historical release needs a frozen historical_specs entry")
                    if relative in historical_set and any(release != "historical" for _, _, release in spec_rows[relative]):
                        self.error(relative, "frozen historical spec must use release historical")
                elif relative in historical_set:
                    self.error(relative, "only pre-migration done specs may be historical")
                for _, impact, release in spec_rows[relative]:
                    if release == "none" and impact != "none":
                        self.error(relative, "non-none impact needs a release reservation")
                    if release != "none" and impact == "none":
                        self.error(relative, "none impact must use release none")
                    if release == "historical" and state != "done":
                        self.error(relative, "historical release is only allowed for done specs")
        for relative in historical_set:
            if states.get(relative) != "done":
                self.error("workspace/releases.json", f"historical path is not a current done spec: {relative}")

        seen_ids: set[str] = set()
        open_components: set[str] = set()
        targets: set[tuple[str, str]] = set()
        expected_members: dict[str, set[str]] = {}
        expected_impact: dict[str, int] = {}
        for relative, rows in spec_rows.items():
            for component, impact, release in rows:
                if release in ("none", "historical") or impact not in IMPACT:
                    continue
                expected_members.setdefault(release, set()).add(relative)
                expected_impact[release] = max(expected_impact.get(release, 0), IMPACT[impact])
        for row in releases:
            if not isinstance(row, dict):
                self.error("workspace/releases.json", "release entry must be an object")
                continue
            release = row.get("id")
            if not isinstance(release, str) or not ID.fullmatch(release) or release in seen_ids:
                self.error("workspace/releases.json", f"invalid or duplicate release id: {release}")
                continue
            seen_ids.add(release)
            component = row.get("component")
            owner = row.get("owner")
            if not isinstance(component, str) or not ID.fullmatch(component):
                self.error("workspace/releases.json", f"{release}: invalid component")
                continue
            if not isinstance(owner, str) or not ID.fullmatch(owner):
                self.error("workspace/releases.json", f"{release}: invalid logical owner")
            impact = row.get("impact")
            if impact not in ("major", "minor", "patch"):
                self.error("workspace/releases.json", f"{release}: invalid aggregate impact")
                continue
            baseline, target = row.get("baseline"), row.get("target")
            if not isinstance(baseline, str) or not SEMVER.fullmatch(baseline) or not isinstance(target, str) or not SEMVER.fullmatch(target):
                self.error("workspace/releases.json", f"{release}: baseline and target need normal X.Y.Z versions")
                continue
            baseline_parts = tuple(map(int, baseline.split(".")))
            computed = self.bumped(baseline_parts, impact)
            if target != ".".join(map(str, computed)):
                self.error("workspace/releases.json", f"{release}: target does not match {impact} bump from {baseline}")
            if (component, target) in targets:
                self.error("workspace/releases.json", f"{release}: component target is already reserved")
            targets.add((component, target))
            if row.get("timing") not in ("development-start", "merge") or row.get("status") not in ("planned", "applied", "released"):
                self.error("workspace/releases.json", f"{release}: invalid timing or status")
            if not isinstance(row.get("evidence"), str) or len(row["evidence"].strip()) < 20:
                self.error("workspace/releases.json", f"{release}: evidence is required")
            members = row.get("specs")
            if (not isinstance(members, list) or not members
                    or any(not isinstance(member, str) or self.relative_file(member) is None for member in members)
                    or len(members) != len(set(members))):
                self.error("workspace/releases.json", f"{release}: member specs must be unique and non-empty")
                continue
            member_set = set(members)
            if member_set != expected_members.get(release, set()):
                self.error("workspace/releases.json", f"{release}: member paths differ from Version Impact rows")
            for member in member_set:
                if states.get(member) not in STATES:
                    self.error("workspace/releases.json", f"{release}: unknown member spec: {member}")
                elif not any(comp == component and rid == release for comp, _, rid in spec_rows[member]):
                    self.error("workspace/releases.json", f"{release}: member component differs: {member}")
            if IMPACT[impact] != expected_impact.get(release, 0):
                self.error("workspace/releases.json", f"{release}: aggregate impact differs from member specs")
            if row.get("status") != "released":
                if component in open_components:
                    self.error("workspace/releases.json", f"{release}: multiple open releases for {component}")
                open_components.add(component)
            if any(effective_states.get(member) in ("test", "done") for member in member_set) and row.get("status") == "planned":
                self.error("workspace/releases.json", f"{release}: test/done specs need an applied version")
            if row.get("timing") == "development-start" and any(effective_states.get(member) == "development" for member in member_set) and row.get("status") == "planned":
                self.error("workspace/releases.json", f"{release}: development-start bump is not applied")
            source = row.get("version_source")
            if not isinstance(source, str):
                self.error("workspace/releases.json", f"{release}: version_source is required")
            else:
                current = self.source_version(source)
                if row.get("status") != "released":
                    expected = baseline if row.get("status") == "planned" else target
                    if current is not None and current != expected:
                        self.error(source, f"{release}: source version {current} differs from {expected}")
        for release in expected_members.keys() - seen_ids:
            self.error("workspace/releases.json", f"missing release reservation: {release}")
        return self.errors


def main() -> int:
    errors = VersionValidator(Path.cwd()).validate()
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        print(f"{len(errors)} version validation error(s)", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
