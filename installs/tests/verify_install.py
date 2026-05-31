#!/usr/bin/env python3
"""Verify that all paths referenced in a clashtui config.yaml exist.

Usage: verify_install.py <path-to-config.yaml>

When config.yaml has core_service.is_user: false, also verifies system users,
groups, and file ownership/permissions.
"""

import grp
import os
import pwd
import stat
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


def user_exists(name: str) -> bool:
    try:
        pwd.getpwnam(name)
        return True
    except KeyError:
        return False


def group_exists(name: str) -> bool:
    try:
        grp.getgrnam(name)
        return True
    except KeyError:
        return False


def user_in_group(user: str, group: str) -> bool:
    try:
        g = grp.getgrnam(group)
        return user in g.gr_mem
    except KeyError:
        return False


def check_user(label: str, name: str) -> None:
    global errors
    if user_exists(name):
        print(f"{GREEN}[OK]{NC} user {label}: {name}")
    else:
        print(f"{RED}[MISSING]{NC} user {label} not found: {name}")
        errors += 1


def check_group(label: str, name: str) -> None:
    global errors
    if group_exists(name):
        print(f"{GREEN}[OK]{NC} group {label}: {name}")
    else:
        print(f"{RED}[MISSING]{NC} group {label} not found: {name}")
        errors += 1


def check_user_in_group(user: str, group: str) -> None:
    global errors
    if user_in_group(user, group):
        print(f"{GREEN}[OK]{NC} user '{user}' is in group '{group}'")
    else:
        print(f"{RED}[MISSING]{NC} user '{user}' is not in group '{group}'")
        errors += 1


def check_owner(label: str, path: str, expected_user: str, expected_group: str) -> None:
    global errors
    st = os.stat(path)
    owner = pwd.getpwuid(st.st_uid).pw_name
    grp_name = grp.getgrgid(st.st_gid).gr_name
    ok = True
    if owner != expected_user:
        print(f"{RED}[FAIL]{NC} {label} owner: expected {expected_user}, got {owner}")
        errors += 1
        ok = False
    if grp_name != expected_group:
        print(f"{RED}[FAIL]{NC} {label} group: expected {expected_group}, got {grp_name}")
        errors += 1
        ok = False
    if ok:
        print(f"{GREEN}[OK]{NC} {label} owner: {owner}:{grp_name}")


def check_group_readable(label: str, path: str) -> None:
    global errors
    st = os.stat(path)
    if st.st_mode & stat.S_IRGRP:
        print(f"{GREEN}[OK]{NC} {label} group-readable: {path}")
    else:
        print(f"{RED}[FAIL]{NC} {label} not group-readable: {path}")
        errors += 1


def check_group_writable(label: str, path: str) -> None:
    global errors
    st = os.stat(path)
    if st.st_mode & stat.S_IWGRP:
        print(f"{GREEN}[OK]{NC} {label} group-writable: {path}")
    else:
        print(f"{RED}[FAIL]{NC} {label} not group-writable: {path}")
        errors += 1


def verify_system(cfg: dict) -> None:
    is_linux = sys.platform.startswith("linux")
    is_macos = sys.platform == "darwin"
    if not (is_linux or is_macos):
        return

    current_user = os.environ.get("USER", pwd.getpwuid(os.getuid()).pw_name)

    def get_core(section_name: str) -> dict:
        section = cfg.get(section_name, {})
        return section.get("core", {}) if isinstance(section, dict) else {}

    mihomo_core = get_core("mihomo")
    singbox_core = get_core("singbox")

    if is_linux:
        if mihomo_core:
            check_user("mihomo", "mihomo")
            check_group("mihomo", "mihomo")
            check_user_in_group(current_user, "mihomo")
        if singbox_core:
            check_user("sing-box", "sing-box")
            check_group("sing-box", "sing-box")
            check_user_in_group(current_user, "sing-box")

    if mihomo_core:
        config_path = mihomo_core.get("config_path", "")
        config_dir = mihomo_core.get("config_dir", "")
        if is_linux:
            if config_path and os.path.isfile(config_path):
                check_owner("mihomo config", config_path, "mihomo", "mihomo")
                check_group_readable("mihomo config", config_path)
            if config_dir and os.path.isdir(config_dir):
                check_group_writable("mihomo config_dir", config_dir)
        elif is_macos:
            if config_path and os.path.isfile(config_path):
                check_group_readable("mihomo config", config_path)
            if config_dir and os.path.isdir(config_dir):
                check_group_writable("mihomo config_dir", config_dir)

    if singbox_core:
        config_path = singbox_core.get("config_path", "")
        config_dir = singbox_core.get("config_dir", "")
        if is_linux:
            if config_path and os.path.isfile(config_path):
                check_owner("singbox config", config_path, "sing-box", "sing-box")
                check_group_readable("singbox config", config_path)
            if config_dir and os.path.isdir(config_dir):
                check_group_writable("singbox config_dir", config_dir)
        elif is_macos:
            if config_path and os.path.isfile(config_path):
                check_group_readable("singbox config", config_path)
            if config_dir and os.path.isdir(config_dir):
                check_group_writable("singbox config_dir", config_dir)


def is_system_install(cfg: dict) -> bool:
    mihomo = cfg.get("mihomo", {})
    if isinstance(mihomo, dict):
        service = mihomo.get("core_service", {})
        if isinstance(service, dict) and service.get("is_user") is False:
            return True
    singbox = cfg.get("singbox", {})
    if isinstance(singbox, dict):
        service = singbox.get("core_service", {})
        if isinstance(service, dict) and service.get("is_user") is False:
            return True
    if isinstance(mihomo, dict):
        service = mihomo.get("core_service", {})
        if isinstance(service, dict) and service.get("is_user") is True:
            return False
    if isinstance(singbox, dict):
        service = singbox.get("core_service", {})
        if isinstance(service, dict) and service.get("is_user") is True:
            return False
    return False


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

    core = get_section("mihomo")
    if core:
        check_path("mihomo config_path", core.get("config_path", ""), "file")
        check_path("mihomo bin_path", core.get("bin_path", ""), "bin")
        check_path("mihomo config_dir", core.get("config_dir", ""), "dir")

    core = get_section("singbox")
    if core:
        check_path("singbox config_path", core.get("config_path", ""), "file")
        check_path("singbox bin_path", core.get("bin_path", ""), "bin")
        check_path("singbox config_dir", core.get("config_dir", ""), "dir")

    check_path("default_keymap.yaml", os.path.join(config_dir, "default_keymap.yaml"), "file")
    check_path("default_theme.yaml", os.path.join(config_dir, "default_theme.yaml"), "file")

    if is_system_install(cfg):
        verify_system(cfg)

    if errors == 0:
        print(f"{GREEN}All verifications passed.{NC}")
        sys.exit(0)
    else:
        print(f"{RED}{errors} check(s) failed.{NC}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
