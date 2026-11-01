"""File opener contracts with fake handlers; never opens real applications."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "contrib/scripts/files/open.sh"


@unittest.skipIf(os.name == "nt", "POSIX opener")
class OpenScriptTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="clashtui-open-")
        self.root = Path(self.temp.name)
        self.env = dict(os.environ, PATH=str(self.root), OPEN_ARGS=str(self.root / "args"))
        self.env.pop("DISPLAY", None)
        self.env.pop("WAYLAND_DISPLAY", None)
        self.tool("uname", "printf Linux")
        for tool in ("open", "xdg-open"):
            self.tool(tool, 'printf "%s\\0" "$@" > "$OPEN_ARGS"')

    def tearDown(self):
        self.temp.cleanup()

    def tool(self, name, body):
        path = self.root / name
        path.write_text("#!/bin/sh\n" + body + "\n")
        path.chmod(0o700)

    def run_open(self, *args):
        return subprocess.run(["/bin/sh", str(SCRIPT), *args], env=self.env,
                              capture_output=True, timeout=5)

    def test_platform_openers_receive_one_literal_path(self):
        for platform, backend in (("Darwin", "open"), ("Linux", "xdg-open")):
            with self.subTest(platform=platform):
                self.tool("uname", "printf " + platform)
                self.env["WAYLAND_DISPLAY"] = "fake-session"
                self.tool("xdg-open" if backend == "open" else "open", "exit 99")
                self.tool(backend, 'printf "%s\\0" "$@" > "$OPEN_ARGS"')
                path = str(self.root / '中文 space "quote" $(touch BAD) ${root} [1]\n.yaml')
                result = self.run_open(path)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((self.root / "args").read_bytes(), path.encode() + b"\0")
                self.assertEqual(result.stdout, b"")
                self.assertFalse((self.root / "BAD").exists())

    def test_headless_linux_reports_error_without_launching_a_terminal_tool(self):
        result = self.run_open(str(self.root))
        self.assertEqual(result.returncode, 1)
        self.assertIn(b"interactive: true", result.stderr)
        self.assertFalse((self.root / "args").exists())

    def test_opener_failure_is_propagated(self):
        self.env["DISPLAY"] = ":fake"
        self.tool("xdg-open", "echo failed >&2; exit 7")
        result = self.run_open(str(self.root))
        self.assertEqual(result.returncode, 7)
        self.assertIn(b"failed", result.stderr)

    def test_invalid_arguments_are_rejected(self):
        for args in ((), ("",), ("relative",), ("/one", "/two")):
            with self.subTest(args=args):
                result = self.run_open(*args)
                self.assertEqual(result.returncode, 2)
                self.assertTrue(result.stderr)


@unittest.skipUnless(os.name == "nt", "Windows PowerShell opener")
class WindowsOpenScriptTests(unittest.TestCase):
    def invoke(self, fail):
        with tempfile.TemporaryDirectory(prefix="clashtui-open-") as directory:
            root = Path(directory)
            path = str(root / "中文 space 'quote' $literal [1].yaml")
            env = dict(os.environ, FILE_SCRIPT=str(ROOT / "contrib/scripts/files/open.ps1"),
                       OPEN_PATH=path, OPEN_ARGS=str(root / "args"), OPEN_FAIL=str(int(fail)))
            # Override the cmdlet inside this process; never invoke a real application.
            command = r'''
function Invoke-Item {
    [CmdletBinding()]
    param([string]$LiteralPath)
    if ($env:OPEN_FAIL -eq '1') { throw 'fake handler failure' }
    [System.IO.File]::WriteAllText($env:OPEN_ARGS, $LiteralPath, (New-Object System.Text.UTF8Encoding($false)))
}
& $env:FILE_SCRIPT -Path $env:OPEN_PATH
'''
            result = subprocess.run(["powershell.exe", "-NoProfile", "-NonInteractive",
                                     "-ExecutionPolicy", "Bypass", "-Command", command],
                                    env=env, capture_output=True, timeout=10)
            if fail:
                self.assertEqual(result.returncode, 1)
                self.assertIn(b"fake handler failure", result.stderr)
            else:
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((root / "args").read_text(encoding="utf-8"), path)

    def test_literal_path(self):
        self.invoke(False)

    def test_failure(self):
        self.invoke(True)


if __name__ == "__main__":
    unittest.main()
