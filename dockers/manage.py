#!/usr/bin/env python3
"""Manage the Alpine OpenRC test container for clashtui."""

import argparse
import os
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent
IMAGE_NAME = "clashtui-alpine-dev"
CONTAINER_NAME = "clashtui-openrc-test"


def run(cmd, check=True, **kwargs):
    return subprocess.run(cmd, check=check, **kwargs)


def build(args):
    """Build the Docker image."""
    print(f"Building {IMAGE_NAME}...")
    run(
        [
            "docker", "build",
            "-f", str(SCRIPT_DIR / "alpine" / "Dockerfile"),
            "-t", IMAGE_NAME,
            str(PROJECT_ROOT),
        ]
    )
    print(f"Image {IMAGE_NAME} built successfully.")


def run_container(args):
    """Run the container in detached mode."""
    _stop_existing()
    extra_args = args.extra or []
    cmd = [
        "docker", "run",
        "-d",
        "--name", CONTAINER_NAME,
        "--hostname", "clashtui-dev",
        "--privileged",
        "-v", f"{PROJECT_ROOT}:/workspace/clashtui",
        "-w", "/workspace/clashtui",
        *extra_args,
        IMAGE_NAME,
        "sleep", "infinity",
    ]
    print(f"Starting container: {CONTAINER_NAME}")
    run(cmd)
    print(f"Container {CONTAINER_NAME} is running.")


def _stop_existing():
    if _container_exists():
        print(f"Removing existing container: {CONTAINER_NAME}")
        run(["docker", "stop", CONTAINER_NAME], check=False)
        run(["docker", "rm", CONTAINER_NAME], check=False)


def _container_exists():
    result = subprocess.run(
        ["docker", "ps", "-a", "--filter", f"name=^{CONTAINER_NAME}$", "--format", "{{.Names}}"],
        capture_output=True, text=True
    )
    return CONTAINER_NAME in result.stdout


def stop(args):
    """Stop and remove the container."""
    print(f"Stopping {CONTAINER_NAME}...")
    run(["docker", "stop", CONTAINER_NAME], check=False)
    run(["docker", "rm", CONTAINER_NAME], check=False)
    print(f"Container {CONTAINER_NAME} removed.")


def shell(args):
    """Open a bash shell in the running container."""
    extra_args = args.extra or []
    cmd = [
        "docker", "exec",
        "-it",
        "-w", "/workspace/clashtui",
        *extra_args,
        CONTAINER_NAME,
        "bash",
    ]
    os.execvp("docker", cmd)


def status(args):
    """Show container status."""
    run(["docker", "ps", "-a", "--filter", f"name={CONTAINER_NAME}"])


def logs(args):
    """Show container logs."""
    follow = ["-f"] if args.follow else []
    run(["docker", "logs", *follow, CONTAINER_NAME])


def clean(args):
    """Remove container and image."""
    stop(args)
    if args.image:
        print(f"Removing image {IMAGE_NAME}...")
        run(["docker", "rmi", IMAGE_NAME], check=False)


def test_install(args):
    """Run install script with openrc inside container."""
    extra_args = args.extra or []
    install_args = " ".join(extra_args)
    cmd = [
        "docker", "exec",
        "-it",
        "-w", "/workspace/clashtui",
        CONTAINER_NAME,
        "bash", "-c",
        f"installs/install --service-controller openrc --no-prompt {install_args}",
    ]
    os.execvp("docker", cmd)


def main():
    parser = argparse.ArgumentParser(
        description="Manage clashtui Alpine OpenRC test container"
    )
    sub = parser.add_subparsers(dest="command")

    sub.add_parser("build", help="Build the Docker image")

    p_run = sub.add_parser("run", help="Run the container (detached)")
    p_run.add_argument("extra", nargs="*", help="Extra docker run args")

    sub.add_parser("stop", help="Stop and remove the container")

    p_shell = sub.add_parser("shell", help="Open bash in the container")
    p_shell.add_argument("extra", nargs="*", help="Extra docker exec args")

    sub.add_parser("status", help="Show container status")

    p_logs = sub.add_parser("logs", help="Show container logs")
    p_logs.add_argument("-f", "--follow", action="store_true", help="Follow log output")

    p_clean = sub.add_parser("clean", help="Stop container and optionally remove image")
    p_clean.add_argument("--image", action="store_true", help="Also remove the image")

    p_test = sub.add_parser("test-install", help="Run install script with openrc in container")
    p_test.add_argument("extra", nargs="*", help="Extra args to pass to install script")

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(1)

    commands = {
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
