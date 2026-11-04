# ClashTui

<p>
  <a href="https://github.com/JohanChane/clashtui/releases"><img src="https://img.shields.io/github/v/release/JohanChane/clashtui" alt="Release"></a>
  <a href="https://github.com/JohanChane/clashtui/releases"><img src="https://img.shields.io/github/v/release/JohanChane/clashtui?include_prereleases&label=pre-release" alt="Pre-release"></a>
  <a href="https://github.com/JohanChane/clashtui/blob/main/LICENSE"><img src="https://img.shields.io/github/license/JohanChane/clashtui" alt="License"></a>
  <a href="https://github.com/JohanChane/clashtui/actions/workflows/ci.yml"><img src="https://github.com/JohanChane/clashtui/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/JohanChane/clashtui/actions/workflows/release.yml"><img src="https://github.com/JohanChane/clashtui/actions/workflows/release.yml/badge.svg" alt="Release CI"></a>
</p>

[clashtui_demo.webm](https://github.com/user-attachments/assets/51d3ee85-b1f4-4d02-9a43-623153e3825b)

Language: [English](./README.md) | [中文](./README_ZH.md)

ClashTui is a terminal user interface (TUI) proxy management tool supporting both **Mihomo** (Clash.Meta) and **sing-box** proxy cores. Switch nodes, update subscriptions, manage connections, and control services — all from the terminal.

## Features

- **Dual Core Support** — Compatible with both Mihomo and sing-box; switch anytime in the UI
- **Subscription Management** — Supports File, URL, and Template (ClashTui template) profile types
- **Proxy Switching** — Switch by group or by node, with latency testing
- **Connection Monitoring** — View all active connections in real time; close individual or all connections
- **Service Control** — Manage core start, stop, and restart via systemd
- **Log Viewing** — View core logs in real time within the interface
- **CLI Mode** — Supports `profile`, `mode`, `service`, `update` subcommands for scripting and automation
- **Config Override** — Override final config via `core_override_config` without modifying original subscription files
- **Template System** — Auto-generate config files using templates + proxy node groups, with variable expansion (recommended for sing-box to avoid configuration version issues)
- **Custom Key Bindings** — Customize shortcuts for each tab via `keymap.yaml`
- **Clipboard** — Copy subscription URLs with `y s` and file paths with `y p` in File; supports custom scripts and OSC 52 over SSH. See [clipboard](docs/clipboard_en.md) ([中文](docs/clipboard_zh.md)).
- **Custom themes** - user-definable via theme.yaml
- **which panel** - convenient for user operations
- **Supports fzf** - users can use fzf for selection

## Supported Platforms

-   [x] Linux
-   [x] macOs
-   [x] Windows

## Installation

### Requirements

Linux and macOS:
-   sudo
-   fzf

Windows:
-   nssm

### Shell installer (Linux/macOS)

The installer prompts by default. Defaults are a system installation with both cores, their templates, and mihomo rules data. The default `--service-controller auto-detect` detects systemd or OpenRC on Linux, launchd on macOS. `--no-prompt` (or `-y` / `--yes`) uses the same defaults without asking.

```sh
# Interactive installation from a local checkout
git clone https://github.com/JohanChane/clashtui.git
cd clashtui
bash installs/install

# Use a specific contrib directory
bash installs/install --contrib-dir /path/to/contrib

# Install for the current user without prompts
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install) --is-user --no-prompt
```

Use `--core mihomo` or `--core sing-box` to select one core. `--templates no` skips templates; `--rules-dat no` skips mihomo rules data. User installations use `~/.local/clashtui` and disable TUN. See `bash installs/install --help` for all options.

Scripts, default configs, and templates are selected in this order: `--contrib-dir PATH`, `contrib/` beside the installer or in its parent directory, then the release's `contrib.tar.gz`. `PATH` can be relative or absolute and must contain `scripts/`, `default_configs/`, and `templates/`. An explicit directory that is missing or incomplete causes an error without falling back. This option only selects resources; missing binaries and enabled rules data may still be downloaded.

The installer reuses existing installed binaries, then checks `PATH`, and downloads only missing binaries. Downloaded clashtui and contrib use the same selected release. `--version <tag>` pins those downloads (default: `latest`); local contrib still takes precedence. Missing cores always use their latest releases. `--repo <owner/repo>` changes the ClashTui download repository.

Reinstalling replaces `config.yaml`, `scripts.json`, core configs and overrides, and installed resources with the installer defaults. Previous contents are saved as `<filename>.bak.N`; identical files are not backed up again. The installer does not start services or enable autostart.

Without prompts, privileged commands use `sudo -n` and fail if authorization is unavailable. For a system installation, run `sudo -v` beforehand or run as root. OpenRC user services are written to `${XDG_CONFIG_HOME:-$HOME/.config}/rc/init.d` without sudo.

To uninstall without prompts, use `bash installs/install --uninstall --no-prompt`; add `--is-user` for a user installation. Uninstalling preserves configs, backups, logs, and system accounts.

### With root access (for TUN mode)

#### Linux

1. \[Optional\] Install mihomo and clashtui from your package repository:

```sh
sudo pacman -S mihomo sing-box clashtui  # ArchLinux
```

This step ensures mihomo, sing-box, and clashtui are available in your environment so the install script will skip downloading them. You can also download them manually and run `which mihomo sing-box clashtui` to verify they are correctly configured.

2. Run the install script:

```sh
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install) --core all
```

Tip: The install script downloads resources from GitHub. If downloads keep failing, try enabling a proxy before running the script.

3. \[Optional\] Enable `clashtui_mihomo.service` / `clashtui_singbox.service` on boot:

```sh
sudo systemctl enable clashtui_mihomo.service
# OR
sudo systemctl enable clashtui_singbox.service
```

#### macOS

1. \[Optional\] Install mihomo and clashtui from Homebrew:

```sh
brew install mihomo sing-box # Note: clashtui was NOT uploaded, please install it manually
```

2. Run the install script (same as Linux):

```sh
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install) --core all
```

3. \[Optional\] Enable `clashtui_mihomo` / `clashtui_singbox` launchd plists on boot:

```sh
sudo launchctl load -w /Library/LaunchDaemons/clashtui_mihomo.plist
# OR
sudo launchctl load -w /Library/LaunchDaemons/clashtui_singbox.plist
```

#### Windows

1. \[Optional\] Install mihomo and clashtui from Scoop:

```powershell
scoop install mihomo sing-box clashtui
# Verify
Get-Command mihomo sing-box clashtui
```

This step ensures mihomo, sing-box, and clashtui are in PATH so the install script will skip downloading them.

2. Run the install script (as Administrator):

```powershell
# Default install to D:\ClashTui
irm https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install.ps1 | iex

# Custom directory (no spaces allowed)
iex "& {$(irm https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install.ps1)} -Core all -InstallDir 'D:\MyTools\ClashTui'"

# Only install mihomo core
iex "& {$(irm https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install.ps1)} -Core mihomo"
```

3. Start clashtui, then use CoreSrvCtl to install and start core services:

Service and clipboard commands are configured in `CONFIG_ROOT/scripts.json`. See [service scripts](docs/service_scripts_en.md) ([中文](docs/service_scripts_zh.md)) for custom commands, per-core scopes and autostart.

The install script does NOT register Windows Services. Launch clashtui and use the built-in CoreSrvCtl to manage services. Enable only one core service at boot (do not enable both simultaneously).

### Without root access (no TUN)

#### Linux

```sh
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install) --core all --is-user
```

Enable on boot:

```sh
systemctl --user enable clashtui_mihomo.service
# OR
systemctl --user enable clashtui_singbox.service
```

#### macOS

```sh
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/installs/install) --core all --is-user
```

Enable on boot:

```sh
launchctl load -w ~/Library/LaunchAgents/clashtui_mihomo.plist
# OR
launchctl load -w ~/Library/LaunchAgents/clashtui_singbox.plist
```

## FAQ

-   File permission issues: Since clashtui requires group permissions, please log in again for the group permissions to take effect.
-   Connection refused issues (e.g., with mihomo): Please use `netstat -utapln | grep 9090` to check whether the Clash API port is open.

## Documentation

| Document | Description |
|----------|-------------|
| [Getting Started](docs/getting_started_en.md) | Detailed usage guide: UI operations, CLI, subscription management, config reference, FAQ |
| [Manual Installation](docs/install_manually_en.md) | Step-by-step manual installation for system/user mode |
| [Feature Design](docs/ClashTui_feature_design_en.md) | Feature design: config structure, subscription management, Template expansion, sing-box merge algorithm |
| [Architecture](docs/architecture_en.md) | Code architecture: module structure, startup flow, TUI event loop, Tab system |
| [Development Conventions](docs/development_conventions.md) | Branch naming, commit format, CHANGELOG conventions |

## Contributing

Issues and pull requests are welcome. Please read the [Development Conventions](docs/development_conventions.md) before contributing.

To get up to speed quickly:
1. [Feature Design](docs/ClashTui_feature_design_en.md) — understand the feature design
2. [Architecture](docs/architecture_en.md) — understand the code structure
