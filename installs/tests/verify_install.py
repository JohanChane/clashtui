#!/usr/bin/env python3
"""Verify that all paths referenced in a clashtui config.yaml exist.

Usage: verify_install.py [--verbose] <path-to-config.yaml>

When config.yaml has core_service.is_user: false, also verifies system users,
groups, and file ownership/permissions.

--verbose / -v  Print file trees of config/core install dirs and file contents.
"""

import argparse
import os
import stat
import subprocess
import sys

try:
    import grp
    import pwd
    _HAS_UNIX = True
except ImportError:
    _HAS_UNIX = False


try:
    from ruamel.yaml import YAML
except ImportError:
    print("ruamel.yaml is required. Install it: pip install ruamel.yaml", file=sys.stderr)
    sys.exit(2)

RED = "\033[0;31m"
GREEN = "\033[0;32m"
NC = "\033[0m"

errors = 0


def _walk_tree(dirpath: str) -> str:
    lines = [dirpath]
    for root, dirs, files in os.walk(dirpath):
        dirs.sort()
        level = root.replace(dirpath, "").count(os.sep) + 1
        for name in sorted(files):
            prefix = "│  " * (level - 1) + "├── "
            lines.append(f"{prefix}{name}")
    return "\n".join(lines)


def _run_tree(dirpath: str) -> str | None:
    try:
        if sys.platform == "win32":
            result = subprocess.run(
                ["cmd", "/c", "tree", "/F", "/A", dirpath],
                capture_output=True, text=True, timeout=10,
            )
        else:
            result = subprocess.run(
                ["tree", "--noreport", dirpath],
                capture_output=True, text=True, timeout=10,
            )
        if result.returncode == 0:
            return result.stdout.rstrip()
    except (FileNotFoundError, subprocess.TimeoutExpired, OSError):
        pass
    return None


def print_tree(label: str, dirpath: str) -> None:
    if not os.path.isdir(dirpath):
        print(f"\n{RED}[MISSING]{NC} {label}: {dirpath}")
        return
    print(f"\n{GREEN}── {label}: {dirpath}{NC}")
    output = _run_tree(dirpath)
    if output is None:
        output = _walk_tree(dirpath)
    print(output)


def print_file(label: str, filepath: str) -> None:
    if not os.path.isfile(filepath):
        print(f"\n{RED}[MISSING]{NC} {label}: {filepath}")
        return
    print(f"\n{GREEN}── {label}: {filepath}{NC}")
    with open(filepath) as f:
        print(f.read().rstrip())


def _install_root(path: str) -> str:
    return os.path.dirname(os.path.dirname(path))


def find_install_roots(cfg: dict) -> set[str]:
    roots = set()
    for section_name in ("mihomo", "singbox"):
        core = cfg.get(section_name, {})
        if not isinstance(core, dict):
            continue
        core = core.get("core", {}) if isinstance(core, dict) else {}
        for key in ("bin_path", "config_dir"):
            p = core.get(key, "")
            if p and os.path.isabs(p):
                root = _install_root(p)
                if root and root not in ("/", "/usr", "/usr/local"):
                    roots.add(root)
    return roots


SERVICE_PATHS = {
    ("linux", "systemd", False): "/usr/lib/systemd/system/{name}.service",
    ("linux", "systemd", True): "~/.config/systemd/user/{name}.service",
    ("linux", "openrc", False): "/etc/init.d/{name}",
    ("linux", "openrc", True): "/etc/user/init.d/{name}",
    ("darwin", "launchd", False): "/Library/LaunchDaemons/{name}.plist",
    ("darwin", "launchd", True): "~/Library/LaunchAgents/{name}.plist",
}


def resolve_service_path(name: str, controller: str, is_user: bool) -> str | None:
    platform = sys.platform
    if platform.startswith("linux"):
        plat = "linux"
    elif platform == "darwin":
        plat = "darwin"
    else:
        return None
    controller = (controller or "").lower()
    template = SERVICE_PATHS.get((plat, controller, is_user))
    if template is None:
        return None
    return os.path.expanduser(template.format(name=name))


def show_service_files(cfg: dict) -> None:
    for section_name in ("mihomo", "singbox"):
        section = cfg.get(section_name, {})
        if not isinstance(section, dict):
            continue
        svc = section.get("core_service", {})
        if not isinstance(svc, dict):
            continue
        name = svc.get("service_name", "")
        controller = svc.get("service_controller", "")
        is_user = svc.get("is_user", True)
        if not name or not controller:
            continue
        path = resolve_service_path(name, controller, is_user)
        if path:
            print_file(f"{section_name} service file", path)


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
    parser = argparse.ArgumentParser(
        description="Verify clashtui install by checking paths in config.yaml."
    )
    parser.add_argument("config_path", help="Path to clashtui config.yaml")
    parser.add_argument("-v", "--verbose", action="store_true",
                        help="Show file trees and config file contents")
    args = parser.parse_args()

    config_path = args.config_path
    if not os.path.isfile(config_path):
        print(f"{RED}[ERROR]{NC} Config file not found: {config_path}", file=sys.stderr)
        sys.exit(1)

    config_dir = os.path.dirname(os.path.realpath(config_path))

    with open(config_path) as f:
        yaml = YAML(typ="safe")
    with open(config_path) as f:
        cfg = yaml.load(f.read()) or {}

    if args.verbose:
        print_tree("clashtui config dir", config_dir)
        for root in sorted(find_install_roots(cfg)):
            print_tree("core install dir", root)
        print_file("clashtui config.yaml", config_path)
        for core_name in ("mihomo", "singbox"):
            core = cfg.get(core_name, {})
            if isinstance(core, dict):
                core = core.get("core", {})
            if isinstance(core, dict) and core.get("config_path"):
                print_file(f"{core_name} core config", core["config_path"])
        show_service_files(cfg)
        print()

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
