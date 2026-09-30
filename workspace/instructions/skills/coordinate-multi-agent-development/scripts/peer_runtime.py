#!/usr/bin/env python3
"""Coordinate local peer agents through an atomic repository-local JSON board."""
from __future__ import annotations

import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import uuid

ID = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")
SEMVER = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")
CRITERION = re.compile(r"AC-[1-9][0-9]*")
INTEGRATION = "refs/workspace-peers/integration"
MAX_ITEMS = 64
TASK_KINDS = {"implementation", "verification", "support"}


class PeerError(Exception):
    pass


def git(root: Path, *args: str, ok: bool = True) -> subprocess.CompletedProcess:
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    if ok and result.returncode:
        raise PeerError("Git operation failed; inspect preserved peer state before retrying")
    return result


def identifier(value: str) -> str:
    if not ID.fullmatch(value) or len(value) > 64:
        raise PeerError("identifiers must be lowercase hyphen-case, at most 64 characters")
    return value


def scope_path(value: str) -> str:
    value = value.rstrip("/")
    if (not value or len(value) > 256 or value != value.strip()
            or any(character in value for character in "\\:*?[\x00\r\n")
            or any(part in {"", ".", "..", ".git"} for part in value.split("/"))
            or value.startswith("workspace/docs/work/multi-agent")):
        raise PeerError("scope must be a safe repository-relative path outside WIP records")
    return value


def timestamp() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


class Runtime:
    def __init__(self, root: Path):
        self.root = Path(git(root, "rev-parse", "--show-toplevel").stdout.strip()).resolve()
        common = git(self.root, "rev-parse", "--path-format=absolute", "--git-common-dir").stdout.strip()
        self.common = Path(common).resolve()
        self.directory = self.common / "workspace-peers"
        if self.directory.is_symlink():
            raise PeerError("peer runtime directory must not be a symlink")
        self.board_path = self.directory / "board.json"
        self.lockfile = self.directory / "board.lock"

    @contextmanager
    def transaction(self, create: bool = False):
        if not create and not self.board_path.is_file():
            raise PeerError("peer board is not initialized; do not recreate lost ownership state")
        self.directory.mkdir(exist_ok=True)
        if self.board_path.is_symlink() or self.lockfile.is_symlink():
            raise PeerError("peer board files must not be symlinks")
        with self.lockfile.open("a+", encoding="utf-8") as lock:
            fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
            try:
                if self.board_path.exists():
                    try:
                        board = json.loads(self.board_path.read_text(encoding="utf-8"))
                    except (OSError, ValueError, UnicodeError):
                        raise PeerError("peer board is damaged; preserve it for explicit recovery")
                    if not isinstance(board, dict) or board.get("schema_version") != 2:
                        raise PeerError("peer board schema is unsupported; preserve it for migration")
                else:
                    board = {}
                yield board
                temporary = self.directory / f"board-{uuid.uuid4().hex}.tmp"
                try:
                    with temporary.open("x", encoding="utf-8") as output:
                        json.dump(board, output, indent=2, sort_keys=True)
                        output.write("\n")
                        output.flush()
                        os.fsync(output.fileno())
                    os.replace(temporary, self.board_path)
                    directory_fd = os.open(self.directory, os.O_RDONLY)
                    try:
                        os.fsync(directory_fd)
                    finally:
                        os.close(directory_fd)
                finally:
                    if temporary.exists():
                        temporary.unlink()
            finally:
                fcntl.flock(lock.fileno(), fcntl.LOCK_UN)

    def revision(self, value: str) -> str:
        result = git(self.root, "rev-parse", "--verify", "--end-of-options", value + "^{commit}", ok=False)
        if result.returncode:
            raise PeerError("revision must resolve to an existing commit")
        return result.stdout.strip()

    def current(self) -> str:
        return self.revision(INTEGRATION)

    def initialize(self, base: str):
        commit = self.revision(base)
        with self.transaction(create=True) as board:
            if board:
                if board.get("base") != commit:
                    raise PeerError("board already initialized from a different base")
                return {"base": board["base"], "integration": self.current()}
            if git(self.root, "show-ref", "--verify", "--quiet", INTEGRATION, ok=False).returncode == 0:
                raise PeerError("integration ref already exists; preserve and recover the board")
            if (self.directory / "worktrees").exists():
                raise PeerError("worktrees already exist; preserve and recover the board")
            default = {"schema_version": 1, "commands": [["task", "check"], ["task", "test"]]}
            canonical = json.dumps(default, sort_keys=True, separators=(",", ":"))
            board.update({"schema_version": 2, "base": commit,
                          "settings": {"plan_revision": 0, "validation_config": default,
                                       "validation_revision": digest(canonical.encode())},
                          "peers": {}, "tasks": {}, "reviews": {},
                          "reservations": {}, "acceptance": None})
            git(self.root, "update-ref", INTEGRATION, commit, "0" * 40)
        return {"base": commit, "integration": commit}

    @staticmethod
    def setting(board: dict, key: str, required: bool = True):
        value = board["settings"].get(key)
        if value is None and required:
            raise PeerError(f"peer board setting is missing: {key}")
        return value

    def integrated_file(self, relative: str) -> bytes:
        relative = scope_path(relative)
        result = git(self.root, "show", f"{self.current()}:{relative}", ok=False)
        if result.returncode:
            raise PeerError("registered file must exist in the current integrated revision")
        return result.stdout.encode()

    @staticmethod
    def validation_config(raw: bytes) -> tuple[dict, str]:
        try:
            value = json.loads(raw)
        except (UnicodeDecodeError, ValueError):
            raise PeerError("validation configuration must be valid JSON")
        commands = value.get("commands") if isinstance(value, dict) else None
        if value.get("schema_version") != 1 or not isinstance(commands, list) or not commands:
            raise PeerError("validation configuration requires schema_version 1 and commands")
        if len(commands) > MAX_ITEMS:
            raise PeerError("validation configuration exceeds 64 commands")
        for command in commands:
            if (not isinstance(command, list) or not command or len(command) > MAX_ITEMS
                    or any(not isinstance(argument, str) or not argument or "\x00" in argument
                           for argument in command)):
                raise PeerError("validation commands must be non-empty argument arrays")
        canonical = json.dumps(value, sort_keys=True, separators=(",", ":"))
        return value, digest(canonical.encode())

    def register_spec(self, spec_path: str, criteria: list[str], validation_path: str,
                      expected_revision: str | None = None):
        criteria = sorted(set(criteria))
        if (not criteria or len(criteria) > MAX_ITEMS
                or any(not CRITERION.fullmatch(item) for item in criteria)):
            raise PeerError("spec criteria must be unique AC-N identifiers")
        spec_path = scope_path(spec_path)
        contract_revision = digest(self.integrated_file(spec_path))
        config, validation_revision = self.validation_config(self.integrated_file(validation_path))
        with self.transaction() as board:
            prior = self.setting(board, "contract_revision", False)
            prior_validation = self.setting(board, "validation_revision", False)
            prior_criteria = self.setting(board, "criteria", False)
            if prior is not None and expected_revision != prior:
                raise PeerError("spec amendment requires the current expected contract revision")
            changed = prior is not None and (prior != contract_revision
                                             or prior_validation != validation_revision
                                             or prior_criteria != criteria)
            board["settings"].update(spec_path=spec_path, contract_revision=contract_revision,
                                     criteria=criteria, validation_config=config,
                                     validation_revision=validation_revision)
            if changed:
                for task in board["tasks"].values():
                    task["stale"] = True
                board["reviews"] = {}
                board["acceptance"] = None
            return {"spec": spec_path, "contract_revision": contract_revision,
                    "validation_revision": validation_revision, "criteria": criteria,
                    "amended": changed}

    def plan_task(self, task: str, kind: str, scopes: list[str], dependencies: list[str],
                  criteria: list[str], expected_plan_revision: int):
        task = identifier(task)
        if kind not in TASK_KINDS:
            raise PeerError("task kind must be implementation, verification or support")
        scopes = sorted(set(scope_path(item) for item in scopes))
        dependencies = sorted(set(identifier(item) for item in dependencies))
        criteria = sorted(set(criteria))
        if not scopes or max(len(scopes), len(dependencies), len(criteria)) > MAX_ITEMS:
            raise PeerError("task plan exceeds supported scope, dependency, or criterion limits")
        with self.transaction() as board:
            revision = self.setting(board, "plan_revision")
            if expected_plan_revision != revision:
                raise PeerError("task plan revision changed; reload before amending")
            accepted = set(self.setting(board, "criteria"))
            if any(not CRITERION.fullmatch(item) or item not in accepted for item in criteria):
                raise PeerError("task references an unknown acceptance criterion")
            if any(item not in board["tasks"] and item != task for item in dependencies):
                raise PeerError("task plan references an unknown dependency")
            prior = board["tasks"].get(task)
            if prior and prior["status"] in {"active", "handoff"}:
                raise PeerError("active task plans cannot be amended")
            history = list(prior.get("history", [])) if prior else []
            history.append({"status": "planned", "at": timestamp(),
                            "event": "amended" if prior else "created"})
            board["tasks"][task] = {"task": task, "kind": kind, "scope": scopes,
                                    "dependencies": dependencies, "criteria": criteria,
                                    "status": "planned", "claimant": None,
                                    "contract_revision": self.setting(board, "contract_revision"),
                                    "validation_revision": self.setting(board, "validation_revision"),
                                    "stale": False, "history": history}
            visiting, visited = set(), set()
            def visit(node: str):
                if node in visiting:
                    raise PeerError("task plan dependency cycle detected")
                if node in visited:
                    return
                visiting.add(node)
                for dependency in board["tasks"].get(node, {}).get("dependencies", []):
                    visit(dependency)
                visiting.remove(node)
                visited.add(node)
            for node in board["tasks"]:
                visit(node)
            board["settings"]["plan_revision"] = revision + 1
            board["acceptance"] = None
            return {"task": task, "plan_revision": revision + 1, "status": "planned"}

    def ready(self):
        with self.transaction() as board:
            integrated = {name for name, task in board["tasks"].items()
                          if task["status"] == "integrated" and not task["stale"]}
            ready = [task.copy() for task in board["tasks"].values()
                     if task["status"] == "planned" and not task["stale"]
                     and set(task["dependencies"]).issubset(integrated)]
            return {"plan_revision": self.setting(board, "plan_revision"),
                    "ready": sorted(ready, key=lambda item: item["task"])}

    def worktree(self, agent: str) -> Path:
        return self.directory / "worktrees" / identifier(agent)

    @staticmethod
    def registered(board: dict, agent: str) -> dict:
        peer = board["peers"].get(identifier(agent))
        if peer is None:
            raise PeerError("unknown peer")
        return peer

    def inspect_worktree(self, path: Path):
        if path.is_symlink() or not path.is_dir():
            raise PeerError("worktree is missing or unsafe; preserve ownership until recovered")
        common = git(path, "rev-parse", "--path-format=absolute", "--git-common-dir").stdout.strip()
        if Path(common).resolve() != self.common:
            raise PeerError("worktree belongs to a different repository")
        if git(path, "symbolic-ref", "-q", "HEAD", ok=False).returncode == 0:
            raise PeerError("peer worktree must remain detached")

    def clean(self, path: Path):
        self.inspect_worktree(path)
        if git(path, "status", "--porcelain", "--untracked-files=all").stdout:
            raise PeerError("worktree has uncommitted changes; preserve or commit them first")

    def actor(self, board: dict, agent: str) -> dict:
        peer = self.registered(board, agent)
        if self.root != self.worktree(agent).resolve():
            raise PeerError("run this operation from the peer's own registered worktree")
        self.inspect_worktree(self.root)
        return peer

    @staticmethod
    def record_paths(peer: dict) -> list[str]:
        directory = f"workspace/docs/work/multi-agent/{peer['task']}"
        return [directory + "/README.md", directory + f"/{peer['agent']}.md"]

    def initial_records(self, path: Path, agent: str, task: str, base: str,
                        scopes: list[str], dependencies: list[str]):
        directory = path / "workspace/docs/work/multi-agent" / task
        if directory.exists():
            raise PeerError("task record destination already exists; choose a new task ID")
        for ancestor in (directory, *directory.parents):
            if ancestor == path:
                break
            if ancestor.is_symlink():
                raise PeerError("WIP destination contains a symlink; preserve the new worktree")
        directory.mkdir(parents=True)
        (directory / "README.md").write_text(
            f"---\nschema_version: 1\ncoordination_id: {task}\nstatus: active\n"
            f"base_revision: {base}\nupdated_at: {timestamp()}\n---\n\n# Peer Task\n\n"
            f"## Assignments\n\n{agent} self-claimed this task through the shared peer board.\n\n"
            f"## Dependencies\n\n{', '.join(dependencies) or 'None'}.\n\n"
            "## Integration\n\nIndependent peer review and transactional integration are required.\n",
            encoding="utf-8")
        (directory / f"{agent}.md").write_text(
            f"---\nschema_version: 1\ncoordination_id: {task}\nagent_id: {agent}\n"
            f"role: implementer\nstatus: active\nbase_revision: {base}\ntask_ref: detached\n"
            f"branch_authorization: none\nupdated_at: {timestamp()}\nscope:\n"
            + "".join(f"  - {scope}\n" for scope in scopes)
            + "---\n\n# Peer Work Record\n\n## Assignment\n\nImplement the claimed scope.\n\n"
            "## Actions\n\nCreated an isolated worktree through an atomic peer claim.\n\n"
            "## Validation\n\nPending.\n\n## Blockers and Dependencies\n\nNone.\n\n"
            "## Next Step\n\nImplement and validate the task.\n\n## Handoff\n\nPending.\n",
            encoding="utf-8")

    def create_claim(self, board: dict, agent: str, task: str,
                     scopes: list[str], dependencies: list[str]):
        identifier(agent), identifier(task)
        scopes = sorted(set(scope_path(item) for item in scopes))
        dependencies = sorted(set(identifier(item) for item in dependencies))
        if not scopes or len(scopes) > MAX_ITEMS or len(dependencies) > MAX_ITEMS:
            raise PeerError("claim requires 1-64 scopes and at most 64 dependencies")
        prior = board["peers"].get(agent)
        if prior:
            if prior["task"] != task or prior["scope"] != scopes or prior["dependencies"] != dependencies:
                raise PeerError("agent identity already owns a different claim")
            self.inspect_worktree(self.worktree(agent))
            return self.public(prior)
        if any(peer["task"] == task for peer in board["peers"].values()):
            raise PeerError("task already claimed")
        for dependency in dependencies:
            planned = board["tasks"].get(dependency)
            legacy = next((peer for peer in board["peers"].values() if peer["task"] == dependency), None)
            if not ((planned and planned["status"] == "integrated" and not planned["stale"])
                    or (legacy and legacy["status"] == "complete")):
                raise PeerError(f"dependency is not integrated: {dependency}")
        base = self.current()
        path = self.worktree(agent)
        path.parent.mkdir(exist_ok=True)
        if path.exists():
            raise PeerError("unregistered worktree exists; preserve it for explicit recovery")
        git(self.root, "worktree", "add", "--detach", str(path), base)
        self.initial_records(path, agent, task, base, scopes, dependencies)
        peer = {"agent": agent, "task": task, "scope": scopes, "dependencies": dependencies,
                "base": base, "status": "active", "head": None, "updated": timestamp(),
                "history": [{"status": "active", "at": timestamp()}]}
        board["peers"][agent] = peer
        if task in board["tasks"]:
            board["tasks"][task].update(status="active", claimant=agent)
            board["tasks"][task].setdefault("history", []).append(
                {"status": "active", "agent": agent, "at": timestamp()})
        return self.public(peer)

    def claim(self, agent: str, task: str):
        with self.transaction() as board:
            planned = board["tasks"].get(identifier(task))
            if planned is None:
                raise PeerError("unknown task; publish the task plan before claiming")
            if planned["stale"]:
                raise PeerError("task is stale after a spec or validation amendment")
            if planned["status"] != "planned":
                raise PeerError("task is not ready for a new claim")
            return self.create_claim(board, agent, task, planned["scope"], planned["dependencies"])

    def join(self, agent: str, task: str, scopes: list[str], dependencies: list[str]):
        """Compatibility shortcut for work not yet represented by a published plan."""
        with self.transaction() as board:
            task = identifier(task)
            agent = identifier(agent)
            if task in board["tasks"]:
                prior = board["peers"].get(agent)
                if prior is None or prior["task"] != task:
                    raise PeerError("task already claimed or planned; use claim for published tasks")
            else:
                board["tasks"][task] = {
                    "task": task, "kind": "support", "scope": sorted(set(map(scope_path, scopes))),
                    "dependencies": sorted(set(map(identifier, dependencies))), "criteria": [],
                    "status": "planned", "claimant": None,
                    "contract_revision": self.setting(board, "contract_revision", False) or "legacy",
                    "validation_revision": self.setting(board, "validation_revision"), "stale": False,
                    "history": [{"status": "planned", "at": timestamp()}]}
            return self.create_claim(board, agent, task, scopes, dependencies)

    def public(self, peer: dict) -> dict:
        result = peer.copy()
        result["scope"], result["dependencies"] = list(peer["scope"]), list(peer["dependencies"])
        result["worktree"] = os.path.relpath(self.worktree(peer["agent"]), self.root)
        staging = self.directory / "integration" / peer["agent"]
        if staging.exists():
            result["staging_worktree"] = os.path.relpath(staging, self.root)
        return result

    def status(self):
        with self.transaction() as board:
            reviews = [review.copy() for group in board["reviews"].values() for review in group.values()]
            return {"integration": self.current(),
                    "contract_revision": self.setting(board, "contract_revision", False),
                    "validation_revision": self.setting(board, "validation_revision"),
                    "plan_revision": self.setting(board, "plan_revision"),
                    "tasks": sorted((value.copy() for value in board["tasks"].values()), key=lambda x: x["task"]),
                    "peers": [self.public(value) for value in sorted(board["peers"].values(), key=lambda x: x["agent"])],
                    "reviews": sorted(reviews, key=lambda x: (x["subject"], x["reviewer"])),
                    "reservations": sorted((value.copy() for value in board["reservations"].values()), key=lambda x: x["component"]),
                    "acceptance": [] if board["acceptance"] is None else [board["acceptance"].copy()]}

    def verify_scope(self, peer: dict, head: str):
        if git(self.root, "merge-base", "--is-ancestor", peer["base"], head, ok=False).returncode:
            raise PeerError("handoff no longer descends from the claimed base")
        comparison = git(self.root, "merge-base", self.current(), head).stdout.strip()
        paths = git(self.root, "diff", "--no-renames", "--name-only", "-z", comparison, head).stdout.split("\0")
        own_records = self.record_paths(peer)
        for path in filter(None, paths):
            if path.startswith("workspace/docs/work/multi-agent/") and path not in own_records:
                raise PeerError("handoff edits another peer record")
            if path not in own_records and not any(path == scope or path.startswith(scope + "/") for scope in peer["scope"]):
                raise PeerError(f"handoff changes an unclaimed path: {path}")

    def handoff(self, agent: str):
        with self.transaction() as board:
            peer = self.actor(board, agent)
            if peer["status"] == "complete":
                raise PeerError("completed task cannot publish a new handoff")
            self.clean(self.root)
            head = self.revision("HEAD")
            self.verify_scope(peer, head)
            for path in self.record_paths(peer):
                if git(self.root, "cat-file", "-e", f"{head}:{path}", ok=False).returncode:
                    raise PeerError("handoff must contain the peer's WIP records")
            git(self.root, "update-ref", f"refs/workspace-peers/checkpoints/{agent}", head)
            peer.update(status="handoff", head=head, updated=timestamp())
            peer.setdefault("history", []).append({"status": "handoff", "head": head, "at": timestamp()})
            board["tasks"][peer["task"]]["status"] = "handoff"
            board["tasks"][peer["task"]].setdefault("history", []).append(
                {"status": "handoff", "head": head, "at": timestamp()})
            board["reviews"].pop(agent, None)
            return {"agent": agent, "head": head, "status": "handoff"}

    def review(self, agent: str, subject: str, revision: str, decision: str):
        if decision not in {"approve", "reject"}:
            raise PeerError("review decision must be approve or reject")
        with self.transaction() as board:
            self.actor(board, agent)
            peer = self.registered(board, subject)
            if agent == subject:
                raise PeerError("review must come from a different peer")
            head = self.revision(revision)
            if peer["status"] != "handoff" or peer["head"] != head:
                raise PeerError("review must identify the current handoff revision")
            task = board["tasks"][peer["task"]]
            if (task["stale"] or task["contract_revision"] !=
                    (self.setting(board, "contract_revision", False) or "legacy")
                    or task["validation_revision"] != self.setting(board, "validation_revision")):
                raise PeerError("task must be reconciled with the current contract before review")
            review = {"subject": subject, "reviewer": agent, "head": head,
                      "contract_revision": self.setting(board, "contract_revision", False) or "legacy",
                      "validation_revision": self.setting(board, "validation_revision"),
                      "decision": decision}
            board["reviews"].setdefault(subject, {})[agent] = review
            return {key: review[key] for key in ("subject", "head", "reviewer", "decision")}

    def gates(self, path: Path, board: dict):
        for gate in self.setting(board, "validation_config")["commands"]:
            result = subprocess.run(gate, cwd=path, capture_output=True, text=True)
            if result.returncode:
                raise PeerError(f"{' '.join(gate)} failed; integration worktree retained for inspection")
        self.clean(path)

    def land(self, agent: str, subject: str):
        with self.transaction() as board:
            self.actor(board, agent)
            peer = self.registered(board, subject)
            if peer["status"] == "complete":
                return {"subject": subject, "status": "complete", "integration": self.current()}
            if peer["status"] != "handoff":
                raise PeerError("task must be handed off before integration")
            contract = self.setting(board, "contract_revision", False) or "legacy"
            validation = self.setting(board, "validation_revision")
            task = board["tasks"][peer["task"]]
            if (task["stale"] or task["contract_revision"] != contract
                    or task["validation_revision"] != validation):
                raise PeerError("task must be reconciled with the current contract before integration")
            reviews = [review for review in board["reviews"].get(subject, {}).values()
                       if review["head"] == peer["head"] and review["contract_revision"] == contract
                       and review["validation_revision"] == validation]
            if not reviews or any(review["decision"] != "approve" for review in reviews):
                raise PeerError("handoff needs independent approval and no unresolved rejection")
            author_tree = self.worktree(subject)
            self.clean(author_tree)
            if git(author_tree, "rev-parse", "HEAD").stdout.strip() != peer["head"]:
                raise PeerError("author advanced after review; publish another handoff")
            self.verify_scope(peer, peer["head"])
            base = self.current()
            if git(self.root, "merge-base", "--is-ancestor", peer["head"], base, ok=False).returncode == 0:
                peer.update(status="complete", updated=timestamp())
                peer.setdefault("history", []).append({"status": "complete", "at": timestamp()})
                board["tasks"][peer["task"]]["status"] = "integrated"
                board["tasks"][peer["task"]].setdefault("history", []).append(
                    {"status": "integrated", "revision": base, "at": timestamp()})
                return {"subject": subject, "status": "complete", "integration": base}
            path = self.directory / "integration" / subject
            path.parent.mkdir(exist_ok=True)
            if path.exists():
                self.clean(path)
                head = git(path, "rev-parse", "HEAD").stdout.strip()
                if head != base:
                    parents = git(path, "show", "-s", "--format=%P", "HEAD").stdout.strip().split()
                    if parents != [base, peer["head"]]:
                        raise PeerError("interrupted integration has a stale base; preserve it before retrying")
                    checkpoint = git(self.root, "rev-parse", "--verify", f"refs/workspace-peers/staging/{subject}", ok=False)
                    if checkpoint.returncode or checkpoint.stdout.strip() != head:
                        raise PeerError("staging commit changed or was interrupted; archive before retrying")
            else:
                git(self.root, "worktree", "add", "--detach", str(path), base)
                head = base
            if head == base:
                git(path, "merge", "--no-ff", "--no-edit", "--", peer["head"])
                for relative in self.record_paths(peer):
                    record = path / relative
                    content = record.read_text(encoding="utf-8")
                    content = re.sub(r"^status: .+$", "status: complete", content, count=1, flags=re.MULTILINE)
                    content = re.sub(r"^updated_at: .+$", f"updated_at: {timestamp()}", content, count=1, flags=re.MULTILINE)
                    content += (f"\nPeer integration contains reviewed source revision `{peer['head']}`. "
                                "The internal integration ref advances only after native gates pass; "
                                "this is not evidence of test or production release.\n")
                    record.write_text(content, encoding="utf-8")
                git(path, "add", "--", *self.record_paths(peer))
                git(path, "commit", "--amend", "--no-edit")
                staged = git(path, "rev-parse", "HEAD").stdout.strip()
                git(self.root, "update-ref", f"refs/workspace-peers/staging/{subject}", staged)
            if board["reservations"]:
                try:
                    entries = json.loads((path / "workspace/releases.json").read_text(encoding="utf-8"))["releases"]
                    for reservation in board["reservations"].values():
                        matches = [entry for entry in entries if entry.get("component") == reservation["component"]
                                   and entry.get("id") == reservation["release_id"]]
                        if len(matches) != 1 or any(matches[0].get(key) != reservation[key]
                                                    for key in ("baseline", "impact", "target")):
                            raise PeerError("integration ledger disagrees with atomic version reservation")
                except (OSError, ValueError, KeyError, TypeError, AttributeError):
                    raise PeerError("integration requires a valid ledger for atomic version reservations")
            self.gates(path, board)
            head = git(path, "rev-parse", "HEAD").stdout.strip()
            git(self.root, "update-ref", INTEGRATION, head, base)
            board["acceptance"] = None
            peer.update(status="complete", updated=timestamp())
            peer.setdefault("history", []).append({"status": "complete", "at": timestamp()})
            board["tasks"][peer["task"]]["status"] = "integrated"
            board["tasks"][peer["task"]].setdefault("history", []).append(
                {"status": "integrated", "revision": head, "at": timestamp()})
            return {"subject": subject, "status": "complete", "integration": head}

    def archive(self, agent: str, subject: str):
        with self.transaction() as board:
            self.actor(board, agent)
            self.registered(board, subject)
            path = self.directory / "integration" / subject
            self.inspect_worktree(path)
            archive_id = identifier(subject) + "-" + uuid.uuid4().hex[:12]
            destination = self.directory / "archives" / archive_id
            destination.parent.mkdir(exist_ok=True)
            head = git(path, "rev-parse", "HEAD").stdout.strip()
            git(self.root, "update-ref", f"refs/workspace-peers/archives/{archive_id}", head)
            git(self.root, "worktree", "move", str(path), str(destination))
            return {"subject": subject, "archive": os.path.relpath(destination, self.root)}

    def reserve(self, agent: str, component: str, release: str, baseline: str, impact: str):
        identifier(component), identifier(release)
        if not SEMVER.fullmatch(baseline):
            raise PeerError("reservation baseline must be normal SemVer")
        if impact not in {"major", "minor", "patch"}:
            raise PeerError("reservation impact must be major, minor or patch")
        ranks = {"patch": 1, "minor": 2, "major": 3}
        with self.transaction() as board:
            self.actor(board, agent)
            raw = git(self.root, "show", self.current() + ":workspace/releases.json", ok=False)
            entries = []
            if raw.returncode == 0:
                try:
                    entries = json.loads(raw.stdout)["releases"]
                    if not isinstance(entries, list) or any(not isinstance(entry, dict) for entry in entries):
                        raise ValueError
                except (ValueError, KeyError, TypeError):
                    raise PeerError("integrated release ledger is malformed")
                for entry in entries:
                    if entry.get("component") != component:
                        continue
                    if entry.get("status") != "released":
                        if entry.get("id") != release or entry.get("baseline") != baseline:
                            raise PeerError("component version is already reserved in the integrated ledger")
                        if ranks[entry["impact"]] > ranks[impact]:
                            impact = entry["impact"]
                published = [entry["target"] for entry in entries
                             if entry.get("component") == component and entry.get("status") == "released"]
                if published and baseline != max(published, key=lambda value: tuple(map(int, value.split(".")))):
                    raise PeerError("reservation baseline is stale against integrated releases")
            prior = board["reservations"].get(component)
            if prior:
                published_prior = any(entry.get("component") == component
                                      and entry.get("id") == prior["release_id"]
                                      and entry.get("status") == "released"
                                      and entry.get("target") == prior["target"] for entry in entries)
                if (prior["release_id"] != release or prior["baseline"] != baseline) and not published_prior:
                    raise PeerError("component version is already reserved by another release")
                if (prior["release_id"] == release and prior["baseline"] == baseline
                        and ranks[prior["impact"]] >= ranks[impact]):
                    return prior.copy()
            major, minor, patch = map(int, baseline.split("."))
            target = {"major": f"{major + 1}.0.0", "minor": f"{major}.{minor + 1}.0",
                      "patch": f"{major}.{minor}.{patch + 1}"}[impact]
            reservation = {"component": component, "release_id": release, "baseline": baseline,
                           "impact": impact, "target": target}
            board["reservations"][component] = reservation
            return reservation.copy()

    def reconcile(self, agent: str, task: str, compatible: bool):
        with self.transaction() as board:
            self.actor(board, agent)
            task = identifier(task)
            row = board["tasks"].get(task)
            if row is None or not row["stale"]:
                raise PeerError("task is not awaiting amendment reconciliation")
            if not compatible:
                row.update(status="blocked")
            row.update(stale=False, contract_revision=self.setting(board, "contract_revision"),
                       validation_revision=self.setting(board, "validation_revision"))
            row.setdefault("history", []).append(
                {"status": row["status"], "reconciled": compatible, "at": timestamp()})
            return row.copy()

    def accept(self, agent: str, revision: str, criteria: list[str]):
        criteria = sorted(set(criteria))
        with self.transaction() as board:
            self.actor(board, agent)
            head = self.revision(revision)
            if head != self.current():
                raise PeerError("acceptance must target the current combined integration revision")
            incomplete = [name for name, task in board["tasks"].items()
                          if task["kind"] != "support" and (task["status"] != "integrated" or task["stale"])]
            if incomplete:
                raise PeerError("whole-spec acceptance requires every planned task integrated and current")
            required = sorted(self.setting(board, "criteria"))
            if criteria != required:
                raise PeerError("acceptance evidence must cover every current criterion exactly")
            board["acceptance"] = {"reporter": agent, "revision": head,
                                   "contract_revision": self.setting(board, "contract_revision"),
                                   "validation_revision": self.setting(board, "validation_revision"),
                                   "criteria": criteria, "reviewer": None, "decision": None}
            return {"revision": head, "criteria": criteria, "status": "review"}

    def accept_review(self, agent: str, revision: str, decision: str):
        if decision not in {"approve", "reject"}:
            raise PeerError("acceptance review decision must be approve or reject")
        with self.transaction() as board:
            self.actor(board, agent)
            report = board["acceptance"]
            if report is None or report["revision"] != self.revision(revision):
                raise PeerError("acceptance review must identify the current report revision")
            if report["revision"] != self.current():
                raise PeerError("acceptance report is stale after integration changed")
            if report["reporter"] == agent:
                raise PeerError("acceptance review must come from a different peer")
            if (report["contract_revision"] != self.setting(board, "contract_revision")
                    or report["validation_revision"] != self.setting(board, "validation_revision")):
                raise PeerError("acceptance report is stale after a contract or validation change")
            report.update(reviewer=agent, decision=decision)
            return {"revision": report["revision"], "reviewer": agent, "decision": decision}

    def cleanup(self, agent: str, subject: str):
        with self.transaction() as board:
            self.actor(board, agent)
            peer = self.registered(board, subject)
            if peer["status"] != "complete":
                raise PeerError("cleanup requires completed integration")
            for path in (self.worktree(subject), self.directory / "integration" / subject):
                if not path.exists():
                    continue
                if path.resolve() == self.root:
                    raise PeerError("a peer cannot remove its currently executing worktree")
                self.clean(path)
                head = git(path, "rev-parse", "HEAD").stdout.strip()
                if git(self.root, "merge-base", "--is-ancestor", head, self.current(), ok=False).returncode:
                    raise PeerError("cleanup would lose uncaptured commits")
                git(self.root, "worktree", "remove", str(path))
            return {"subject": subject, "cleaned": True}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    init = commands.add_parser("init"); init.add_argument("--base", required=True)
    register = commands.add_parser("spec-register")
    register.add_argument("--spec", required=True); register.add_argument("--criterion", action="append", required=True)
    register.add_argument("--validation-config", required=True); register.add_argument("--expected-revision")
    plan = commands.add_parser("plan")
    plan.add_argument("--task", required=True); plan.add_argument("--kind", choices=sorted(TASK_KINDS), required=True)
    plan.add_argument("--scope", action="append", required=True); plan.add_argument("--depends-on", action="append", default=[])
    plan.add_argument("--criterion", action="append", default=[]); plan.add_argument("--expected-plan-revision", required=True, type=int)
    commands.add_parser("ready")
    claim = commands.add_parser("claim"); claim.add_argument("--agent", required=True); claim.add_argument("--task", required=True)
    join = commands.add_parser("join")
    join.add_argument("--agent", required=True); join.add_argument("--task", required=True)
    join.add_argument("--scope", action="append", required=True); join.add_argument("--depends-on", action="append", default=[])
    commands.add_parser("status")
    handoff = commands.add_parser("handoff"); handoff.add_argument("--agent", required=True)
    review = commands.add_parser("review")
    review.add_argument("--agent", required=True); review.add_argument("--subject", required=True)
    review.add_argument("--revision", required=True); review.add_argument("--decision", choices=["approve", "reject"], required=True)
    reserve = commands.add_parser("reserve")
    for field in ("agent", "component", "release", "baseline", "impact"):
        reserve.add_argument("--" + field, required=True)
    for name in ("land", "cleanup", "archive"):
        command = commands.add_parser(name); command.add_argument("--agent", required=True); command.add_argument("--subject", required=True)
    reconcile = commands.add_parser("reconcile")
    reconcile.add_argument("--agent", required=True); reconcile.add_argument("--task", required=True)
    reconcile.add_argument("--compatible", action=argparse.BooleanOptionalAction, required=True)
    accept = commands.add_parser("accept")
    accept.add_argument("--agent", required=True); accept.add_argument("--revision", required=True)
    accept.add_argument("--criterion", action="append", required=True)
    accept_review = commands.add_parser("accept-review")
    accept_review.add_argument("--agent", required=True); accept_review.add_argument("--revision", required=True)
    accept_review.add_argument("--decision", choices=["approve", "reject"], required=True)
    args = parser.parse_args(argv)
    try:
        runtime = Runtime(Path.cwd())
        if args.command == "init": result = runtime.initialize(args.base)
        elif args.command == "spec-register": result = runtime.register_spec(args.spec, args.criterion, args.validation_config, args.expected_revision)
        elif args.command == "plan": result = runtime.plan_task(args.task, args.kind, args.scope, args.depends_on, args.criterion, args.expected_plan_revision)
        elif args.command == "ready": result = runtime.ready()
        elif args.command == "claim": result = runtime.claim(args.agent, args.task)
        elif args.command == "join": result = runtime.join(args.agent, args.task, args.scope, args.depends_on)
        elif args.command == "review": result = runtime.review(args.agent, args.subject, args.revision, args.decision)
        elif args.command == "reserve": result = runtime.reserve(args.agent, args.component, args.release, args.baseline, args.impact)
        elif args.command == "reconcile": result = runtime.reconcile(args.agent, args.task, args.compatible)
        elif args.command == "accept": result = runtime.accept(args.agent, args.revision, args.criterion)
        elif args.command == "accept-review": result = runtime.accept_review(args.agent, args.revision, args.decision)
        elif args.command in {"land", "cleanup", "archive"}: result = getattr(runtime, args.command)(args.agent, args.subject)
        elif args.command == "handoff": result = runtime.handoff(args.agent)
        else: result = runtime.status()
        print(json.dumps(result, indent=2))
        return 0
    except (PeerError, OSError, ValueError, KeyError, TypeError) as exc:
        print(str(exc) if isinstance(exc, PeerError) else "Peer operation failed; preserve runtime state for inspection", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
