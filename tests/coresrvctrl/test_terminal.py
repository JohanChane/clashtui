#!/usr/bin/env python3
"""Real PTY handoff tests. Only temporary scripts run; no services or sudo."""
import errno
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
import unittest

BINARY = Path(sys.argv.pop(1) if len(sys.argv) > 1 else "target/debug/clashtui").resolve()
ENTER = b"Press Enter to return to clashtui..."
RESTORED = b"\x1b[?1049h"


class Session:
    def __init__(self, root):
        self.pid, self.master = pty.fork()
        if self.pid == 0:
            os.environ["TERM"] = "xterm-256color"
            os.execv(str(BINARY), [str(BINARY), f"--config-dir={root}"])
        fcntl.ioctl(self.master, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
        self.output = b""

    def send(self, value):
        os.write(self.master, value)

    def drain(self, duration=0.15):
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            if select.select([self.master], [], [], 0.02)[0]:
                try:
                    self.output += os.read(self.master, 65536)
                except OSError as error:
                    if error.errno == errno.EIO:
                        return
                    raise

    def dismiss(self):
        self.send(b"\x1b")
        self.drain()

    def expect(self, value, timeout=10):
        deadline = time.monotonic() + timeout
        while value not in self.output:
            if time.monotonic() >= deadline:
                plain = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", self.output)
                raise AssertionError(f"PTY missing {value!r}; text={plain[-1800:]!r}")
            ready, _, _ = select.select([self.master], [], [], 0.05)
            if ready:
                try:
                    data = os.read(self.master, 65536)
                except OSError as error:
                    if error.errno == errno.EIO:
                        raise AssertionError(f"PTY closed before {value!r}; tail={self.output[-4000:]!r}") from error
                    raise
                if not data:
                    raise AssertionError("PTY closed")
                self.output += data
        end = self.output.index(value) + len(value)
        self.output = self.output[end:]

    def enter_menu(self):
        self.expect(b"CoreSrvCtl")
        # Other tabs initialize too and may report an unavailable local core API.
        self.drain(0.3)
        for _ in range(5):
            self.dismiss()
        self.send(b"7")
        self.expect(b"Test action")
        self.output = b""

    def close(self):
        if not self.pid:
            return
        try:
            os.kill(self.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            pid, _ = os.waitpid(self.pid, os.WNOHANG)
            if pid:
                self.pid = None
                self.final_modes = termios.tcgetattr(self.master)
                os.close(self.master)
                return
            self.drain(0.02)
        os.kill(self.pid, signal.SIGKILL)
        os.waitpid(self.pid, 0)
        self.pid = None
        os.close(self.master)
        raise AssertionError("TUI did not stop after SIGTERM")


class TerminalTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="clashtui-terminal-")
        self.root = Path(self.temp.name)
        (self.root / "config.yaml").write_text("mihomo: {}\nsingbox: {}\n", encoding="utf-8")
        self.session = None

    def tearDown(self):
        if self.session:
            self.session.close()
        self.temp.cleanup()

    def configure(self, body, program="/bin/sh"):
        (self.root / "action.sh").write_text(body, encoding="utf-8")
        (self.root / "scripts.json").write_text(json.dumps({
            "root": ".", "actions": [
                {"id": "restart_core", "label": "Test action", "cmd": {
                    "program": program, "args": ["${root}/action.sh"]}},
                {"id": "status_core", "cmd": {
                    "program": "/bin/sh", "args": ["-c", "printf 'inactive\\n'"]}},
            ]}), encoding="utf-8")

    def start(self):
        self.session = Session(self.root)
        self.session.enter_menu()
        return self.session

    def set_command_options(self, **options):
        path = self.root / "scripts.json"
        config = json.loads(path.read_text())
        config["actions"][0]["cmd"].update(options)
        path.write_text(json.dumps(config))

    def test_capture_popup_shows_both_streams_without_terminal_handoff(self):
        self.configure("printf 'POPUP_STDOUT'; printf 'POPUP_STDERR' >&2\n")
        self.set_command_options(interactive=False)
        session = self.start()
        session.send(b"\r")
        session.drain(1)
        self.assertIn(b"POPUP_STDOUT", session.output)
        self.assertIn(b"POPUP_STDERR", session.output)
        self.assertNotIn(ENTER, session.output)
        self.assertNotIn(b"\x1b[?1049l", session.output)

    def test_capture_silent_suppresses_both_streams(self):
        self.configure("printf 'SILENT_STDOUT'; printf 'SILENT_STDERR' >&2; printf done > completed\n")
        self.set_command_options(interactive=False, output="silent")
        session = self.start()
        session.send(b"\r")
        session.drain(1)
        self.assertEqual((self.root / "completed").read_text(), "done")
        self.assertNotIn(b"SILENT_STDOUT", session.output)
        self.assertNotIn(b"SILENT_STDERR", session.output)
        self.assertNotIn(ENTER, session.output)

    def test_capture_terminal_forwards_raw_stdout_on_render_thread(self):
        # A title sequence exercises raw bytes without touching the real clipboard.
        self.configure("printf '\\033]0;SCRIPT_TITLE\\007'; printf 'SCRIPT_DIAGNOSTIC' >&2\n")
        self.set_command_options(interactive=False, output="terminal")
        session = self.start()
        session.send(b"\r")
        session.drain(1)
        self.assertIn(b"\x1b]0;SCRIPT_TITLE\x07", session.output)
        self.assertIn(b"SCRIPT_DIAGNOSTIC", session.output)
        self.assertNotIn(ENTER, session.output)
        self.assertNotIn(b"\x1b[?1049l", session.output)

    def configure_switch(self, start_body, stop_interactive):
        self.configure("printf stopped\n")
        path = self.root / "scripts.json"
        config = json.loads(path.read_text())
        config["actions"][0]["id"] = "stop_core"
        config["actions"][0]["cmd"].update(interactive=stop_interactive, output="silent")
        (self.root / "start.sh").write_text(start_body)
        config["actions"].append({"id": "start_core", "cmd": {
            "program": "/bin/sh", "args": ["${root}/start.sh"],
            "interactive": False, "output": "terminal"}})
        path.write_text(json.dumps(config))

    def test_switch_delivers_captured_output_before_exiting(self):
        self.configure_switch("printf '\\033]0;SWITCH_FINISHED\\007'\n", False)
        session = self.start()
        session.send(b"j\r")
        session.expect(b"\x1b]0;SWITCH_FINISHED\x07")

    def test_ctrl_c_cancels_captured_step_in_interactive_switch(self):
        self.configure_switch("printf ready > start-ready; sleep 2; printf started > start-completed\n", True)
        session = self.start()
        session.send(b"j\r")
        deadline = time.monotonic() + 5
        while not (self.root / "start-ready").exists():
            self.assertLess(time.monotonic(), deadline, "Captured start did not begin")
            session.drain(0.02)
        session.send(b"\x03")
        session.expect(RESTORED)
        self.assertFalse((self.root / "start-completed").exists())
        self.assertIn("core_type: mihomo", (self.root / "clashtui.db").read_text())

    def test_file_copy_chords_send_subscription_and_paths(self):
        self.configure("exit 0\n")
        url = "https://example.test/sub?token=a%2Fb&flag=1"
        (self.root / "clashtui.db").write_text(
            "core_type: mihomo\nmihomo:\n  profiles:\n    subscription: !Url " + json.dumps(url) + "\n")
        profile = self.root / "mihomo/profiles/subscription.yaml"
        template = self.root / "mihomo/templates/template.yaml"
        for path in (profile, template):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("proxies: []\n")
        path = self.root / "scripts.json"
        config = json.loads(path.read_text())
        config["actions"].append({"id": "copy", "cmd": {
            "program": "/bin/sh", "args": ["-c", 'cat > .copied; printf "%s/%s" "$CLASHTUI_COPY_SOURCE" "$CLASHTUI_COPY_FORMAT" > copy-metadata; mv .copied copied'],
            "interactive": False, "output": "silent"}})
        path.write_text(json.dumps(config))
        session = self.start()
        session.send(b"2")
        session.expect(b"subscription")
        copied = self.root / "copied"
        session.send(b"y")
        session.drain(0.2)
        self.assertFalse(copied.exists(), "Copy must wait for the chord's second key")
        for keys, expected, source in (
            (b"s", url, "profile"),
            (b"yp", str(profile.resolve()), "profile"),
            (b"lyp", str(template.resolve()), "template"),
        ):
            copied.unlink(missing_ok=True)
            session.send(keys)
            deadline = time.monotonic() + 5
            while not copied.exists():
                self.assertLess(time.monotonic(), deadline, "Copy action did not finish")
                session.drain(0.02)
            self.assertEqual(copied.read_text(), expected)
            self.assertEqual((self.root / "copy-metadata").read_text(), source + "/text")
            session.drain(0.15)
        self.assertNotIn(ENTER, session.output)

    def test_child_owns_stdin_and_repeated_actions_restore_terminal(self):
        self.configure("printf 'INPUT_READY\\n'\nIFS= read -r value\nprintf '%s\\n' \"$value\" >> received\n")
        session = self.start()
        for value in [b"one two 'quotes' $(data)", "中文参数".encode()]:
            session.send(b"\r")
            session.expect(b"INPUT_READY")
            session.send(value + b"\n")
            session.expect(ENTER)
            session.send(b"\n")
            session.expect(RESTORED)
            # Wait for a fresh menu draw before submitting the next action.
            session.expect(b"Test action")
        self.assertEqual((self.root / "received").read_bytes().splitlines(), [b"one two 'quotes' $(data)", "中文参数".encode()])

    def test_fast_child_and_queued_enter_do_not_replay_action(self):
        self.configure("printf 'run\\n' >> invocations\n")
        session = self.start()
        session.send(b"\r\r\r")
        session.expect(ENTER)
        session.send(b"\n")
        session.expect(RESTORED)
        session.expect(b"Test action")
        self.assertEqual((self.root / "invocations").read_text().splitlines(), ["run"])

    def test_spawn_failure_restores_terminal(self):
        self.configure("", program="/clashtui-test/no-such-program")
        session = self.start()
        session.send(b"\r")
        session.expect(b"Failed to start")
        session.expect(ENTER)
        session.send(b"\n")
        session.expect(RESTORED)
        session.drain()
        session.dismiss()
        session.send(b"1")
        session.expect(b"Status")

    def test_ctrl_c_interrupts_child_and_restores_tui(self):
        self.configure("printf 'INTERRUPT_READY\\n'\nexec sleep 30\n")
        session = self.start()
        session.send(b"\r")
        session.expect(b"INTERRUPT_READY")
        session.send(b"\x03")
        session.expect(RESTORED)
        session.drain()
        session.dismiss()
        session.send(b"1")
        session.expect(b"Status")

    def test_cli_uses_exit_status_and_never_waits_for_enter(self):
        self.configure("printf 'Error is only explanatory text\\n'\nexit 0\n")
        command = [str(BINARY), f"--config-dir={self.root}", "service", "restart"]
        result = subprocess.run(command, stdin=subprocess.DEVNULL, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn(ENTER, result.stdout)

        self.configure("printf 'success text cannot hide failure\\n'\nexit 17\n")
        result = subprocess.run(command, stdin=subprocess.DEVNULL, capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"17", result.stderr)
        self.assertNotIn(ENTER, result.stdout)

    def test_nested_command_owns_stdin_and_propagates_failure(self):
        # Model a script waiting for sudo without invoking the host's real sudo.
        (self.root / "fake-sudo.sh").write_text(
            "printf 'NESTED_INPUT_READY\\n'\nIFS= read -r value\n"
            "printf '%s\\n' \"$value\" > nested-input\nexit 17\n",
            encoding="utf-8",
        )
        self.configure('/bin/sh ./fake-sudo.sh\nresult=$?\nexit "$result"\n')
        session = self.start()
        session.send(b"\r")
        session.expect(b"NESTED_INPUT_READY")
        session.send(b"fixture input\n")
        session.expect(b"17")
        session.expect(ENTER)
        self.assertEqual((self.root / "nested-input").read_text(), "fixture input\n")
        session.send(b"\n")
        session.expect(RESTORED)

    def test_sigterm_reaps_foreground_command_and_restores_terminal(self):
        self.configure("printf '%s\\n' \"$$\" > wrapper-pid\n"
                       "/bin/sh -c 'echo $$ > child-pid; printf \"CANCEL_READY\\n\"; exec sleep 30'\n"
                       "result=$?\nexit \"$result\"\n")
        session = self.start()
        session.send(b"\r")
        session.expect(b"CANCEL_READY")
        child = int((self.root / "child-pid").read_text())
        wrapper = int((self.root / "wrapper-pid").read_text())
        session.close()
        with self.assertRaises(ProcessLookupError):
            os.kill(child, 0)
        with self.assertRaises(ProcessLookupError):
            os.kill(wrapper, 0)

    def test_stop_all_uses_one_terminal_handoff(self):
        self.configure("printf '%s\\n' \"$1\" >> stopped\n")
        path = self.root / "scripts.json"
        config = json.loads(path.read_text())
        config["actions"][0]["id"] = "stop_core"
        config["actions"][0]["cmd"]["args"].append("${core_type}")
        path.write_text(json.dumps(config))
        session = self.start()
        session.send(b"jj\r")
        session.expect(ENTER)
        self.assertEqual((self.root / "stopped").read_text().splitlines(), ["mihomo", "singbox"])
        session.send(b"\n")
        session.expect(RESTORED)
        session.expect(b"Stop All Services")

    def test_child_terminal_mode_changes_are_restored_before_enter(self):
        self.configure("stty raw -echo\nprintf 'MODES_CHANGED\\n'\n")
        session = self.start()
        session.send(b"\r")
        session.expect(b"MODES_CHANGED")
        session.expect(ENTER)
        session.send(b"\r")
        session.expect(RESTORED)
        session.close()
        self.assertTrue(session.final_modes[3] & termios.ECHO)
        self.assertTrue(session.final_modes[3] & termios.ICANON)

    def test_cli_capture_cancellation_reaps_its_command(self):
        for sig in (signal.SIGINT, signal.SIGTERM):
            with self.subTest(signal=sig):
                marker = self.root / "capture-pid"
                marker.unlink(missing_ok=True)
                self.configure("printf '%s\\n' \"$$\" > capture-pid; exec sleep 30\n")
                self.set_command_options(interactive=False, output="silent")
                process = subprocess.Popen(
                    [str(BINARY), f"--config-dir={self.root}", "service", "restart"],
                    stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                child = None
                try:
                    deadline = time.monotonic() + 5
                    while not marker.exists():
                        self.assertLess(time.monotonic(), deadline, "Captured CLI command did not start")
                        time.sleep(0.01)
                    child = int(marker.read_text())
                    process.send_signal(sig)
                    process.communicate(timeout=3)
                    self.assertNotEqual(process.returncode, 0)
                    with self.assertRaises(ProcessLookupError):
                        os.kill(child, 0)
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.communicate()
                    if child:
                        try:
                            os.kill(child, signal.SIGKILL)
                        except ProcessLookupError:
                            pass

    def test_cli_sigterm_reaps_its_command(self):
        self.configure("printf '%s\\n' \"$$\" > child-pid\nexec sleep 30\n")
        process = subprocess.Popen(
            [str(BINARY), f"--config-dir={self.root}", "service", "restart"],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        child = None
        try:
            deadline = time.monotonic() + 5
            while not (self.root / "child-pid").exists():
                self.assertLess(time.monotonic(), deadline, "CLI command did not start")
                time.sleep(0.01)
            child = int((self.root / "child-pid").read_text())
            process.terminate()
            process.communicate(timeout=5)
            with self.assertRaises(ProcessLookupError):
                os.kill(child, 0)
        finally:
            if process.poll() is None:
                process.kill()
                process.communicate()
            if child:
                try:
                    os.kill(child, signal.SIGKILL)
                except ProcessLookupError:
                    pass


if __name__ == "__main__":
    unittest.main()
