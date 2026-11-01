"""Clipboard script contract using fake tools; never touches a real clipboard."""
import base64
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "contrib/scripts/clipboard/copy.sh"


@unittest.skipIf(os.name == "nt", "POSIX clipboard script")
class CopyScriptTests(unittest.TestCase):
    def test_backends_preserve_input_and_receive_the_correct_arguments(self):
        payload = "中文 'quotes' $(must-not-run)\r\n\n".encode()
        with tempfile.TemporaryDirectory(prefix="clashtui copy ") as directory:
            root = Path(directory)
            for backend, expected in {
                "pbcopy": b"\n",
                "wl-copy": b"--type\ntext/plain;charset=utf-8\n",
                "xclip": b"-selection\nclipboard\n-in\n",
                "xsel": b"--clipboard\n--input\n",
            }.items():
                with self.subTest(backend=backend):
                    tool = root / backend
                    tool.write_text('#!/bin/sh\nprintf "%s\\n" "$@" > "$COPY_ARGS"\ncat > "$COPY_TEXT"\n')
                    tool.chmod(0o700)
                    env = dict(os.environ, PATH=f"{root}{os.pathsep}{os.environ['PATH']}",
                               COPY_ARGS=str(root / "args"), COPY_TEXT=str(root / "text"))
                    result = subprocess.run(["/bin/sh", str(SCRIPT), backend], input=payload,
                                            env=env, capture_output=True, timeout=5)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual((root / "text").read_bytes(), payload)
                    self.assertEqual((root / "args").read_bytes(), expected)
                    self.assertEqual(result.stdout, b"")

    def test_osc52_preserves_empty_unicode_newlines_and_large_input(self):
        for payload in (b"", "中文\r\n\n".encode(), b"x" * (1024 * 1024)):
            with self.subTest(size=len(payload)):
                result = subprocess.run(["/bin/sh", str(SCRIPT), "osc52"], input=payload,
                                        capture_output=True, timeout=5)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, b"\x1b]52;c;" + base64.b64encode(payload) + b"\x07")
                self.assertEqual(result.stderr, b"")

    def test_auto_uses_osc52_for_ssh_and_without_local_tools(self):
        payload = b"remote\n\n"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            # Use an isolated PATH: no real clipboard command can be selected.
            for name in ("base64", "tr"):
                import shutil
                (root / name).symlink_to(shutil.which(name))
            (root / "uname").write_text("#!/bin/sh\necho Linux\n")
            (root / "uname").chmod(0o700)
            for ssh in (False, True):
                env = {k: v for k, v in os.environ.items()
                       if k not in ("SSH_TTY", "SSH_CONNECTION", "SSH_CLIENT", "DISPLAY", "WAYLAND_DISPLAY")}
                env["PATH"] = str(root)
                if ssh:
                    env.update(SSH_CONNECTION="test", WAYLAND_DISPLAY="wayland-test")
                    (root / "wl-copy").write_text("#!/bin/sh\nexit 99\n")
                    (root / "wl-copy").chmod(0o700)
                result = subprocess.run(["/bin/sh", str(SCRIPT), "auto"], input=payload,
                                        env=env, capture_output=True, timeout=5)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, b"\x1b]52;c;" + base64.b64encode(payload) + b"\x07")

    def test_selected_backend_failure_is_not_hidden_by_osc52_fallback(self):
        with tempfile.TemporaryDirectory() as directory:
            tool = Path(directory) / "wl-copy"
            tool.write_text("#!/bin/sh\necho failed >&2\nexit 7\n")
            tool.chmod(0o700)
            result = subprocess.run(["/bin/sh", str(SCRIPT), "wl-copy"], input=b"text",
                                    env=dict(os.environ, PATH=directory), capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 7)
            self.assertEqual(result.stdout, b"")
            self.assertIn(b"failed", result.stderr)

    def test_invalid_backend_or_extra_arguments_fail(self):
        for args in (["invalid"], ["pbcopy", "extra"]):
            result = subprocess.run(["/bin/sh", str(SCRIPT), *args], input=b"text",
                                    capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 2)
            self.assertTrue(result.stderr)


@unittest.skipUnless(os.name == "nt", "Windows PowerShell clipboard script")
class WindowsCopyScriptTests(unittest.TestCase):
    def test_osc52_preserves_utf8_and_newlines(self):
        script = SCRIPT.with_suffix(".ps1")
        payload = "中文\r\n\n".encode()
        for backend in ("osc52", "auto"):
            result = subprocess.run(["powershell.exe", "-NoProfile", "-NonInteractive", "-File", str(script), backend],
                                    input=payload, env=dict(os.environ, SSH_CONNECTION="test"),
                                    capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, b"\x1b]52;c;" + base64.b64encode(payload) + b"\x07")

    def test_native_uses_utf8_input_with_a_mock_clipboard(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "input.txt"
            payload = "中文\r\n\n".encode()
            command = "function Set-Clipboard { param($Value) [IO.File]::WriteAllText($env:COPY_TEST_TEXT, $Value, (New-Object Text.UTF8Encoding($false))) }; & $env:COPY_TEST_SCRIPT native"
            result = subprocess.run(["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", command],
                                    input=payload, capture_output=True, timeout=10,
                                    env=dict(os.environ, COPY_TEST_TEXT=str(target), COPY_TEST_SCRIPT=str(SCRIPT.with_suffix(".ps1"))))
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, b"")
            self.assertEqual(target.read_bytes(), payload)


if __name__ == "__main__":
    unittest.main()
