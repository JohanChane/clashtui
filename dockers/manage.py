#!/usr/bin/env python3
"""Manage clashtui Docker test containers (multiple targets)."""

import argparse
import os
import subprocess
import sys
from pathlib import Path

# ── target definitions ──────────────────────────────────────────────
# Each target maps to: image tag, container name, Dockerfile subdir, and
# the service controller the install script should use.
TARGETS = {
    "alpine": {
        "image": "clashtui-alpine-dev",
        "container": "clashtui-openrc-test",
        "subdir": "alpine",
        "service_controller": "openrc",
        "shell": "bash",
        "user": "johan",
    },
    "debian": {
        "image": "clashtui-debian-dev",
        "container": "clashtui-systemd-test",
        "subdir": "debian",
        "service_controller": "systemd",
        "shell": "bash",
        "user": "johan",
    },
}

DEFAULT_TARGET = "alpine"


def _find_project_root(start: Path) -> Path:
    for parent in [start, *start.parents]:
        if (parent / "Cargo.toml").exists():
            return parent
    raise FileNotFoundError("Could not find project root (Cargo.toml)")


SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = _find_project_root(SCRIPT_DIR)
BUILD_CTX = SCRIPT_DIR  # docker build context is the dockers/ directory


def _resolve_target(name: str) -> dict:
    t = TARGETS.get(name)
    if t is None:
        print(f"Unknown target: {name}. Available: {', '.join(TARGETS)}")
        sys.exit(1)
    return t


def run(cmd, check=True, **kwargs):
    return subprocess.run(cmd, check=check, **kwargs)


# ── commands ─────────────────────────────────────────────────────────


def list_targets(args):
    """List all available targets."""
    width_name = max(len(k) for k in TARGETS)
    width_img = max(len(v["image"]) for v in TARGETS.values())
    width_ctr = max(len(v["container"]) for v in TARGETS.values())
    header = f"  {'TARGET':<{width_name}}  {'IMAGE':<{width_img}}  {'CONTAINER':<{width_ctr}}  SERVICE"
    print(header)
    print("  " + "-" * (len(header) - 2))
    for name, t in TARGETS.items():
        print(
            f"  {name:<{width_name}}  {t['image']:<{width_img}}  {t['container']:<{width_ctr}}  {t['service_controller']}"
        )


def build(args):
    """Build the Docker image for a target."""
    t = _resolve_target(args.target)
    dockerfile = BUILD_CTX / "images" / t["subdir"] / "Dockerfile"
    image = t["image"]
    print(f"Building {image} (target={args.target})...")
    run(
        [
            "docker",
            "build",
            "-f",
            str(dockerfile),
            "-t",
            image,
            str(BUILD_CTX),
        ]
    )
    print(f"Image {image} built successfully.")


def run_container(args):
    """Run the container in detached mode."""
    t = _resolve_target(args.target)
    _stop_existing(t)
    extra_args = args.extra or []
    cmd = [
        "docker",
        "run",
        "-d",
        "--name",
        t["container"],
        "--hostname",
        f"clashtui-{args.target}",
        "--privileged",
    ]
    # systemd needs host cgroup namespace + writable cgroup fs
    if t["service_controller"] == "systemd":
        cmd += [
            "--cgroupns=host",
            "-v",
            "/sys/fs/cgroup:/sys/fs/cgroup:rw",
        ]
    cmd += [
        "-v",
        f"{PROJECT_ROOT}:/home/johan/workspace/clashtui",
        "-w",
        "/home/johan/workspace/clashtui",
        *extra_args,
        t["image"],
    ]
    # systemd images use /sbin/init from Dockerfile CMD; alpine uses sleep infinity
    if args.target == "alpine":
        cmd += ["sleep", "infinity"]

    print(f"Starting container: {t['container']}")
    run(cmd)
    print(f"Container {t['container']} is running.")


def _stop_existing(t: dict):
    if _container_exists(t):
        print(f"Removing existing container: {t['container']}")
        run(["docker", "stop", t["container"]], check=False)
        run(["docker", "rm", t["container"]], check=False)


def _container_exists(t: dict):
    result = subprocess.run(
        [
            "docker",
            "ps",
            "-a",
            "--filter",
            f"name=^{t['container']}$",
            "--format",
            "{{.Names}}",
        ],
        capture_output=True,
        text=True,
    )
    return t["container"] in result.stdout


def stop(args):
    """Stop and remove the container."""
    t = _resolve_target(args.target)
    print(f"Stopping {t['container']}...")
    run(["docker", "stop", t["container"]], check=False)
    run(["docker", "rm", t["container"]], check=False)
    print(f"Container {t['container']} removed.")


def _maybe_user_env(t: dict):
    """Return docker exec args for user when target defines one."""
    if "user" in t:
        u = t["user"]
        return [
            "-u",
            u,
            "-e",
            f"USER={u}",
            "-e",
            f"LOGNAME={u}",
            "-e",
            f"HOME=/home/{u}",
        ]
    return []


def shell(args):
    """Open a shell in the running container."""
    t = _resolve_target(args.target)
    extra_args = args.extra or []
    cmd = [
        "docker",
        "exec",
        "-it",
        "-w",
        "/home/johan/workspace/clashtui",
        *_maybe_user_env(t),
        *extra_args,
        t["container"],
        t["shell"],
    ]
    os.execvp("docker", cmd)


def status(args):
    """Show container status (all targets)."""
    containers = "|".join(v["container"] for v in TARGETS.values())
    run(["docker", "ps", "-a", "--filter", f"name={containers}"])


def logs(args):
    """Show container logs."""
    t = _resolve_target(args.target)
    follow = ["-f"] if args.follow else []
    run(["docker", "logs", *follow, t["container"]])


def clean(args):
    """Remove container and optionally image."""
    t = _resolve_target(args.target)
    stop(args)
    if args.image:
        print(f"Removing image {t['image']}...")
        run(["docker", "rmi", t["image"]], check=False)


def test_install(args):
    """Run install script inside the container."""
    t = _resolve_target(args.target)
    extra_args = args.extra or []
    install_args = " ".join(extra_args)
    sc = t["service_controller"]
    cmd = [
        "docker",
        "exec",
        "-it",
        "-w",
        "/home/johan/workspace/clashtui",
        *_maybe_user_env(t),
        t["container"],
        t["shell"],
        "-c",
        f"installs/install --service-controller {sc} --no-prompt {install_args}",
    ]
    os.execvp("docker", cmd)


# ── main ─────────────────────────────────────────────────────────────


def main():
    parser = argparse.ArgumentParser(
        description="Manage clashtui Docker test containers"
    )
    parser.add_argument(
        "-t",
        "--target",
        default=DEFAULT_TARGET,
        choices=list(TARGETS.keys()),
        help=f"Target image to use (default: {DEFAULT_TARGET})",
    )

    sub = parser.add_subparsers(dest="command")

    sub.add_parser("list-targets", help="List all available targets")
    sub.add_parser("build", help="Build the Docker image")

    p_run = sub.add_parser("run", help="Run the container (detached)")
    p_run.add_argument("extra", nargs="*", help="Extra docker run args")

    sub.add_parser("stop", help="Stop and remove the container")

    p_shell = sub.add_parser("shell", help="Open a shell in the container")
    p_shell.add_argument("extra", nargs="*", help="Extra docker exec args")

    sub.add_parser("status", help="Show container status (all targets)")

    p_logs = sub.add_parser("logs", help="Show container logs")
    p_logs.add_argument("-f", "--follow", action="store_true", help="Follow log output")

    p_clean = sub.add_parser("clean", help="Stop container and optionally remove image")
    p_clean.add_argument("--image", action="store_true", help="Also remove the image")

    p_test = sub.add_parser("test-install", help="Run install script in container")
    p_test.add_argument("extra", nargs="*", help="Extra args to pass to install script")

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(1)

    commands = {
        "list-targets": list_targets,
        "build": build,
        "run": run_container,
        "stop": stop,
        "shell": shell,
        "status": status,
        "logs": logs,
        "clean": clean,
        "test-install": test_install,
    }

    commands[args.command](args)


if __name__ == "__main__":
    main()
