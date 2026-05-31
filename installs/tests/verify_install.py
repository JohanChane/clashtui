#!/usr/bin/env python3
"""Verify that all paths referenced in a clashtui config.yaml exist.

Usage: verify_install.py <path-to-config.yaml>
"""

import os
import sys

try:
    from ruamel.yaml import YAML
except ImportError:
    print("ruamel.yaml is required. Install it: pip install ruamel.yaml", file=sys.stderr)
    sys.exit(2)

RED = "\033[0;31m"
GREEN = "\033[0;32m"
NC = "\033[0m"

errors = 0


def check_path(label: str, path: str, kind: str) -> None:
    global errors
    if not path:
        return
    missing = False
    if kind == "file":
        missing = not os.path.isfile(path)
    elif kind == "dir":
        missing = not os.path.isdir(path)
    elif kind == "bin":
        missing = not (os.path.isfile(path) and os.access(path, os.X_OK))
    if missing:
        print(f"{RED}[MISSING]{NC} {label} {kind} not found: {path}")
        errors += 1
    else:
        print(f"{GREEN}[OK]{NC} {label}: {path}")


def main() -> None:
    if len(sys.argv) < 2:
        print(f"{RED}[ERROR]{NC} Usage: {sys.argv[0]} <path-to-config.yaml>", file=sys.stderr)
        sys.exit(1)

    config_path = sys.argv[1]
    if not os.path.isfile(config_path):
        print(f"{RED}[ERROR]{NC} Config file not found: {config_path}", file=sys.stderr)
        sys.exit(1)

    config_dir = os.path.dirname(os.path.realpath(config_path))

    with open(config_path) as f:
        yaml = YAML(typ="safe")
    with open(config_path) as f:
        cfg = yaml.load(f.read()) or {}

    def get_section(section_name: str) -> dict:
        section = cfg.get(section_name, {})
        return section.get("core", {}) if isinstance(section, dict) else {}

    # mihomo
    core = get_section("mihomo")
    if core:
        check_path("mihomo config_path", core.get("config_path", ""), "file")
        check_path("mihomo bin_path", core.get("bin_path", ""), "bin")
        check_path("mihomo config_dir", core.get("config_dir", ""), "dir")

    # singbox
    core = get_section("singbox")
    if core:
        check_path("singbox config_path", core.get("config_path", ""), "file")
        check_path("singbox bin_path", core.get("bin_path", ""), "bin")
        check_path("singbox config_dir", core.get("config_dir", ""), "dir")

    # default configs
    check_path("default_keymap.yaml", os.path.join(config_dir, "default_keymap.yaml"), "file")
    check_path("default_theme.yaml", os.path.join(config_dir, "default_theme.yaml"), "file")

    if errors == 0:
        print(f"{GREEN}All paths verified successfully.{NC}")
        sys.exit(0)
    else:
        print(f"{RED}{errors} path(s) are missing.{NC}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
