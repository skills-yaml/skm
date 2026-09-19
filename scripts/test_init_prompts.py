#!/usr/bin/env python3
"""Linux PTY smoke coverage; run task build, then task test:init."""

import atexit
import fcntl
import os
import pty
import select
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path


if sys.platform != "linux":
    print("SKIP: PTY smoke coverage runs on Linux; portable prompt tests run via task test")
    raise SystemExit(0)


BINARY = Path(__file__).resolve().parents[1] / "target/release/skm"
if not BINARY.is_file():
    raise SystemExit("Run task build before task test:init")


class Prompts:
    def __init__(self, project, env, args=()):
        self.master, self.slave = pty.openpty()
        self.original_terminal = termios.tcgetattr(self.slave)
        self.closed = False

        def session():
            os.setsid()
            fcntl.ioctl(self.slave, termios.TIOCSCTTY, 0)

        self.process = subprocess.Popen(
            [str(BINARY), "init", *args],
            cwd=project,
            env=env,
            stdin=self.slave,
            stdout=subprocess.PIPE,
            stderr=self.slave,
            preexec_fn=session,
        )
        self.output = b""
        atexit.register(self.close)

    def close(self):
        if self.closed:
            return
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait(timeout=5)
        for descriptor in (self.master, self.slave):
            try:
                os.close(descriptor)
            except OSError:
                pass
        self.closed = True
        atexit.unregister(self.close)

    def drain(self, duration=0.2):
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            ready, _, _ = select.select(
                [self.master], [], [], max(0, deadline - time.monotonic())
            )
            if not ready:
                continue
            try:
                chunk = os.read(self.master, 65536)
            except OSError:
                break
            if not chunk:
                break
            self.output += chunk

    def wait_for(self, expected):
        expected = expected.encode()
        deadline = time.monotonic() + 15
        while expected not in self.output and time.monotonic() < deadline:
            self.drain(0.1)
        assert expected in self.output, (expected, self.output[-2000:])

    def send_line(self, value=""):
        os.write(self.master, value.encode() + b"\n")
        self.drain()

    def finish(self, returncode=0):
        self.process.wait(timeout=15)
        self.drain()
        assert self.process.returncode == returncode, self.output[-2000:]
        assert self.process.stdout.read() == b"", "init prompts wrote to stdout"
        assert termios.tcgetattr(self.slave) == self.original_terminal
        assert b"\x1b[" not in self.output, "init emitted terminal control sequences"
        self.close()


def keep_common_sections(prompts):
    prompts.wait_for("Project version")
    prompts.send_line()
    prompts.wait_for("Agents [keep]")
    prompts.send_line()
    prompts.wait_for("Registry action")
    prompts.send_line()


with tempfile.TemporaryDirectory(prefix="skm-init-prompts-") as temp:
    root = Path(temp)
    project = root / "project"
    project.mkdir()
    home = root / "home"
    home.mkdir()
    env = dict(
        os.environ,
        HOME=str(home),
        XDG_CONFIG_HOME=str(root / "config"),
        XDG_CACHE_HOME=str(root / "cache"),
        SKM_NO_UPDATE_CHECK="1",
        TERM="xterm-256color",
    )
    manifest = project / "skills.yaml"

    prompts = Prompts(project, env)
    prompts.wait_for("Project name")
    assert not manifest.exists()
    prompts.send_line("prompt-project")
    keep_common_sections(prompts)
    prompts.wait_for("Skill action")
    prompts.send_line("a")
    prompts.wait_for("Skill name")
    prompts.send_line("team/spec")
    prompts.wait_for("Version")
    prompts.send_line()
    prompts.wait_for("Registry source")
    prompts.send_line("-")
    prompts.wait_for("Local path")
    prompts.send_line("./local/spec")
    prompts.wait_for("Skill action")
    prompts.send_line()
    prompts.wait_for("Edit these advanced settings?")
    prompts.send_line()
    prompts.wait_for("== Review ==")
    assert not manifest.exists()
    prompts.wait_for("Save skills.yaml?")
    prompts.send_line()
    prompts.finish()
    saved = manifest.read_text()
    assert "name: prompt-project" in saved
    assert "name: team/spec" in saved and "path: ./local/spec" in saved
    print("PASS sequential prompts, manual skill entry, review/save, and stdout separation")

    original = "# retained on cancel\n" + saved + "extension: keep-me\n"
    manifest.write_text(original)
    prompts = Prompts(project, env)
    prompts.wait_for("Project name")
    prompts.send_line("unsaved")
    prompts.wait_for("Project version")
    prompts.send_line(":q")
    prompts.finish()
    assert manifest.read_text() == original
    print("PASS cancellation preserves the existing manifest exactly")

    local_registry = root / "registry"
    skill = local_registry / "skills" / "ops" / "beta" / "v2.0.0"
    skill.mkdir(parents=True)
    (skill / "SKILL.md").write_text("# Fixture skill\n")
    manifest.write_text(
        "name: search-project\n"
        "version: 0.1.0\n"
        f"registries:\n  default: {local_registry}\n"
        "agents: [codex]\n"
        "skills: []\n"
        "extension: keep-me\n"
    )
    prompts = Prompts(project, env)
    prompts.wait_for("Project name")
    prompts.send_line()
    keep_common_sections(prompts)
    prompts.wait_for("Skill action")
    prompts.send_line("s")
    prompts.wait_for("Search query")
    prompts.send_line("beta")
    prompts.wait_for("ops/beta")
    prompts.send_line("1")
    prompts.wait_for("Skill action")
    refreshed_skill = local_registry / "skills" / "ops" / "fresh" / "v1.0.0"
    refreshed_skill.mkdir(parents=True)
    (refreshed_skill / "SKILL.md").write_text("# Refreshed fixture skill\n")
    prompts.send_line("r")
    prompts.wait_for("Search query")
    prompts.send_line("fresh")
    prompts.wait_for("ops/fresh")
    prompts.send_line()
    prompts.wait_for("Skill action")
    prompts.send_line()
    prompts.wait_for("Edit these advanced settings?")
    prompts.send_line()
    prompts.wait_for("Save skills.yaml?")
    prompts.send_line()
    prompts.finish()
    selected = manifest.read_text()
    assert "name: ops/beta" in selected
    assert "source: default" in selected and "version: v2.0.0" in selected
    assert "extension: keep-me" in selected
    print("PASS on-demand registry search, refresh, and selection preserve extension values")

    prompts = Prompts(project, env)
    prompts.wait_for("Project name")
    prompts.send_line("draft-name")
    external = "name: external-change\nagents: []\nskills: []\n"
    manifest.write_text(external)
    keep_common_sections(prompts)
    prompts.wait_for("Skill action")
    prompts.send_line()
    prompts.wait_for("Edit these advanced settings?")
    prompts.send_line()
    prompts.wait_for("Save skills.yaml?")
    prompts.send_line()
    prompts.finish(returncode=1)
    assert b"changed outside the prompt flow" in prompts.output
    assert manifest.read_text() == external
    print("PASS concurrent edits block saving")

    manifest.unlink()
    piped = subprocess.run(
        [str(BINARY), "init"],
        cwd=project,
        env=env,
        input=b"",
        capture_output=True,
        timeout=10,
    )
    assert piped.returncode == 1 and b"needs a terminal" in piped.stderr
    scripted = subprocess.run(
        [str(BINARY), "init", "--non-interactive", "--name", "scripted"],
        cwd=project,
        env=env,
        capture_output=True,
        timeout=10,
    )
    assert scripted.returncode == 0 and "name: scripted" in manifest.read_text()
    print("PASS piped input fails promptly and non-interactive creation still works")
