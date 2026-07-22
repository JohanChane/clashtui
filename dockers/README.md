# Docker

Test containers for clashtui. Supports multiple Linux targets via `--target` / `-t`.

## Targets

| Target   | Base            | Init system | Service controller |
|----------|-----------------|-------------|---------------------|
| `alpine` | Alpine 3.22     | OpenRC      | `openrc`            |
| `debian` | Debian 13 (Trixie) | systemd   | `systemd`           |

## Usage

All commands accept `-t <target>` (default: `alpine`).

```bash
# Build images
./dockers/manage.py -t alpine build
./dockers/manage.py -t debian build

# Run container (detached, mounts project at /home/johan/workspace/clashtui)
./dockers/manage.py -t alpine run
./dockers/manage.py -t debian run

# Open a shell in the container
./dockers/manage.py -t debian shell

# Run the install script inside the container (auto-selects service controller)
./dockers/manage.py -t alpine test-install
./dockers/manage.py -t debian test-install

# Show container status (all targets)
./dockers/manage.py status

# Show logs
./dockers/manage.py -t debian logs         # One-shot
./dockers/manage.py -t debian logs -f      # Follow

# Stop and remove the container
./dockers/manage.py -t debian stop

# Stop container and remove image
./dockers/manage.py -t debian clean --image
```

The script can run from any directory. It locates the project root by searching upwards for `Cargo.toml`.
