#!/usr/bin/env python3
"""Linux PTY smoke coverage; run task build, then task test:tui."""
import sys

if sys.platform != "linux":
    print("SKIP: PTY smoke coverage runs on Linux; portable wizard tests run via task test")
    raise SystemExit(0)

import atexit
import fcntl
import os
import pty
import select
import struct
import subprocess
import tempfile
import termios
import time
from pathlib import Path

binary = Path(__file__).resolve().parents[1] / 'target/release/skm'
if not binary.is_file():
    raise SystemExit('Run task build before task test:tui')

class Wizard:
    def __init__(self, project, env, args=()):
        self.master, self.slave = pty.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack('HHHH', 32, 120, 0, 0))
        self.original_terminal = termios.tcgetattr(self.slave)
        def session():
            os.setsid()
            fcntl.ioctl(self.slave, termios.TIOCSCTTY, 0)
        self.process = subprocess.Popen([str(binary), 'init', *args], cwd=project, env=env,
            stdin=self.slave, stdout=subprocess.PIPE, stderr=self.slave, preexec_fn=session)
        atexit.register(self.close)
        self.output = b''
        self.wait_for(b'SKM setup')

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait(timeout=5)
        for fd in (self.master, self.slave):
            try:
                os.close(fd)
            except OSError:
                pass
        atexit.unregister(self.close)

    def drain(self, duration=0.2):
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            ready, _, _ = select.select([self.master], [], [], max(0, deadline - time.monotonic()))
            if ready:
                self.output += os.read(self.master, 65536)

    def wait_for(self, expected):
        deadline = time.monotonic() + 10
        while expected not in self.output and time.monotonic() < deadline:
            self.drain(0.1)
        assert expected in self.output, (expected, self.output[-1500:])

    def send(self, data):
        os.write(self.master, data)
        self.drain()

    def finish(self):
        self.process.wait(timeout=10)
        self.drain()
        assert self.process.returncode == 0, self.output[-1500:]
        assert self.process.stdout.read() == b'', 'wizard wrote to stdout'
        assert termios.tcgetattr(self.slave) == self.original_terminal, 'terminal was not restored'
        assert b'\x1b[?1049l' in self.output, 'alternate screen was not restored'
        self.close()

with tempfile.TemporaryDirectory(prefix='skm-wizard-smoke-') as temp:
    root = Path(temp)
    project = root / 'project'
    project.mkdir()
    env = dict(os.environ, XDG_CONFIG_HOME=str(root / 'config'), XDG_CACHE_HOME=str(root / 'cache'),
        SKM_CHECK_UPDATE='false', TERM='xterm-256color')
    manifest = project / 'skills.yaml'

    wizard = Wizard(project, env)
    wizard.wait_for(b'New configuration')
    wizard.send(b'\r\x15wizard-project\r')
    wizard.send(b'\t' * 5)
    wizard.wait_for(b'Ready to save')
    assert not manifest.exists()
    wizard.send(b'\r')
    wizard.finish()
    assert 'name: wizard-project' in manifest.read_text()
    print('PASS new configuration, live editing, review/save, stdout separation, terminal restoration')

    original = '# retained on cancel and no-op\n' + manifest.read_text() + 'extension: keep-me\n'
    manifest.write_text(original)
    wizard = Wizard(project, env)
    wizard.wait_for(b'Existing file')
    wizard.send(b'\r-unsaved\r\x1b')
    wizard.finish()
    assert manifest.read_text() == original
    print('PASS existing file loads and cancellation preserves exact bytes')

    wizard = Wizard(project, env, ['--advanced'])
    wizard.send(b'\t' * 5)
    wizard.send(b'\r')
    wizard.finish()
    assert manifest.read_text() == original
    print('PASS advanced alias and no-op save preserve comments')

    wizard = Wizard(project, env)
    wizard.send(b'\r\x15updated-project\r')
    wizard.send(b'\t' * 5)
    wizard.send(b'\r')
    wizard.finish()
    assert 'name: updated-project' in manifest.read_text()
    assert 'extension: keep-me' in manifest.read_text()
    print('PASS existing configuration edits retain extension fields')

    wizard = Wizard(project, env)
    wizard.send(b'\r\x15draft-name\r')
    external = 'name: external-change\n'
    manifest.write_text(external)
    wizard.send(b'\t' * 5)
    wizard.send(b'\r')
    wizard.wait_for(b'changed outside')
    wizard.send(b'\x03')
    wizard.finish()
    assert manifest.read_text() == external
    print('PASS concurrent edits block save and Ctrl-C restores terminal')

    manifest.unlink()
    result = subprocess.run([str(binary), 'init'], cwd=project, env=env, input=b'', capture_output=True, timeout=10)
    assert result.returncode == 1 and b'needs a terminal' in result.stderr
    assert not manifest.exists()
    result = subprocess.run([str(binary), 'init', '--non-interactive', '--name', 'scripted'], cwd=project, env=env, capture_output=True, timeout=10)
    assert result.returncode == 0 and 'name: scripted' in manifest.read_text()
    print('PASS piped input fails promptly and non-interactive creation still works')
