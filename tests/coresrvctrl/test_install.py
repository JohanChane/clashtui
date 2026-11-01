#!/usr/bin/env python3
"""Installer config contracts, using local resources and temporary destinations only."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]

HARNESS = r'''source "$TASK_REPO/installs/install"
CLASHTUI_CONFIG_DIR="$TASK_CONFIG"
CONTRIB_SOURCE=local
CONTRIB_DIR="$TASK_REPO/contrib"
IS_USER="$TASK_USER"
IS_MACOS="$TASK_MACOS"
SERVICE_CONTROLLER="$TASK_BACKEND"
MIHOMO_USER_CONFIG_DIR="$TASK_CONFIG/mihomo"
SINGBOX_USER_CONFIG_DIR="$TASK_CONFIG/sing-box"
MIHOMO_CONFIG_DIR="$TASK_CONFIG/core-mihomo"
SINGBOX_CONFIG_DIR="$TASK_CONFIG/core-singbox"
INSTALL_DIR_MIHOMO="$TASK_CONFIG/bin-mihomo"
INSTALL_DIR_SINGBOX="$TASK_CONFIG/bin-singbox"
# No network, authentication or real service commands in this harness.
curl() { echo 'Unexpected network access' >&2; return 97; }
sudo() { echo 'Unexpected sudo' >&2; return 97; }
systemctl() { return 97; }
launchctl() { return 97; }
rc-service() { return 97; }
rc-update() { return 97; }
'''


@unittest.skipIf(os.name == "nt", "Bash installer runs on Unix")
class UnixInstallTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="clashtui install ")
        self.addCleanup(self.temp.cleanup)
        self.config = Path(self.temp.name) / "config"
        self.env = dict(os.environ, TASK_REPO=str(ROOT), TASK_CONFIG=str(self.config),
                        TASK_USER="false", TASK_MACOS="false", TASK_BACKEND="systemd")

    def run_helper(self, command="create_service_scripts"):
        return subprocess.run(["bash", "-c", HARNESS + command], env=self.env,
                              text=True, capture_output=True)

    def test_backend_and_scope_matrix(self):
        for backend in ("systemd", "openrc", "launchd"):
            for scope in ("system", "user"):
                with self.subTest(backend=backend, scope=scope):
                    self.env.update(TASK_BACKEND="systemd" if backend == "launchd" else backend,
                                    TASK_MACOS=str(backend == "launchd").lower(),
                                    TASK_USER=str(scope == "user").lower())
                    result = self.run_helper()
                    self.assertEqual(result.returncode, 0, result.stderr)
                    source = ROOT / "contrib/scripts/coresrvctrl" / backend
                    expected = json.loads((source / "default.json").read_text().replace('"${scope}"', f'"{scope}"'))
                    actual = json.loads((self.config / "scripts.json").read_text())
                    self.assertEqual(actual, expected)
                    self.assertEqual((self.config / f"scripts/coresrvctrl/{backend}/service.sh").read_bytes(),
                                     (source / "service.sh").read_bytes())
                    self.assertEqual((self.config / "scripts/clipboard/copy.sh").read_bytes(),
                                     (ROOT / "contrib/scripts/clipboard/copy.sh").read_bytes())
                    self.assertEqual((self.config / "scripts/files/open.sh").read_bytes(),
                                     (ROOT / "contrib/scripts/files/open.sh").read_bytes())
                    self.assertFalse(list(self.config.glob(".service-scripts.*")))

    def test_config_generation_backs_up_and_refreshes_defaults(self):
        result = self.run_helper("create_clashtui_config all")
        self.assertEqual(result.returncode, 0, result.stderr)
        (self.config / "scripts.json").write_text("custom config")
        script = self.config / "scripts/coresrvctrl/systemd/service.sh"
        script.write_text("custom script")
        self.env["TASK_USER"] = "true"
        result = self.run_helper("create_clashtui_config all")
        self.assertEqual(result.returncode, 0, result.stderr)
        backup = self.config.with_name("config_1")
        self.assertEqual((backup / "scripts.json").read_text(), "custom config")
        self.assertEqual((backup / "scripts/coresrvctrl/systemd/service.sh").read_text(), "custom script")
        config = json.loads((self.config / "scripts.json").read_text())
        self.assertEqual(config["actions"][0]["cmd"]["args"][2], "user")
        yaml = (self.config / "config.yaml").read_text()
        self.assertNotIn("is_user", yaml)
        self.assertNotIn("service_controller", yaml)

    def test_failed_download_keeps_published_files(self):
        self.assertEqual(self.run_helper().returncode, 0)
        before = {path: path.read_bytes() for path in self.config.rglob("*") if path.is_file()}
        # Earlier resources succeed; the last script fails before publishing.
        command = r'''copy_contrib() {
  case "$1" in
    */files/open.sh) return 23 ;;
    *) cp "$CONTRIB_DIR/$1" "$2" ;;
  esac
}
create_service_scripts
'''
        result = self.run_helper(command)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual({path: path.read_bytes() for path in self.config.rglob("*") if path.is_file()}, before)
        self.assertFalse(list(self.config.glob(".service-scripts.*")))

    def test_config_generation_propagates_service_install_failure(self):
        result = self.run_helper("create_service_scripts() { return 23; }; create_clashtui_config all")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.config / "mihomo/profiles").exists())

    def test_unknown_backend_fails_without_files(self):
        self.env["TASK_BACKEND"] = "unknown"
        self.assertNotEqual(self.run_helper().returncode, 0)
        self.assertFalse(self.config.exists())


if __name__ == "__main__":
    unittest.main()
