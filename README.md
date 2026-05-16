> **Important Notice**
>
> clashtui v0.3.0 is now pre-released. Barring any critical issues, it will go into full release after May 23. The following changes will be made to the repository:
> - The `main` and `dev` branches will be renamed to `archive/main` and `archive/dev`, respectively. They have been tagged as `archive/v0.2.3` and `archive/v0.2.3-dev`.
> - The `demotui` branch will be renamed to `main`, and a new `dev` branch will be created.
>
> ***Package maintainers, please take note.***

# ClashTui

<p>
  <a href="https://github.com/JohanChane/clashtui/releases"><img src="https://img.shields.io/github/v/release/JohanChane/clashtui" alt="Release"></a>
  <a href="https://github.com/JohanChane/clashtui/releases"><img src="https://img.shields.io/github/v/release/JohanChane/clashtui?include_prereleases&label=pre-release" alt="Pre-release"></a>
  <a href="https://github.com/JohanChane/clashtui/blob/main/LICENSE"><img src="https://img.shields.io/github/license/JohanChane/clashtui" alt="License"></a>
  <a href="https://github.com/JohanChane/clashtui/actions/workflows/build.yml"><img src="https://github.com/JohanChane/clashtui/actions/workflows/build.yml/badge.svg" alt="Build"></a>
  <a href="https://github.com/JohanChane/clashtui/actions/workflows/pr.yml"><img src="https://github.com/JohanChane/clashtui/actions/workflows/pr.yml/badge.svg" alt="PR"></a>
  <a href="https://github.com/JohanChane/clashtui/actions/workflows/release.yml"><img src="https://github.com/JohanChane/clashtui/actions/workflows/release.yml/badge.svg" alt="Release CI"></a>
</p>

<video src="https://github.com/user-attachments/assets/7808534a-84bc-4967-a024-534487ab7aaf" controls width="100%"></video>


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
- **Custom themes** - user-definable via theme.yaml
- **which panel** - convenient for user operations
- **Supports fzf** - users can use fzf for selection

## Supported Platforms

-   [x] Linux
-   [x] macOs
-   [ ] Windows

## Installation

### Requirements

-   sudo
-   fzf

### With root access (for TUN mode)

1. \[Optional\] Install mihomo and clashtui from your package repository:

```sh
sudo pacman -S mihomo sing-box clashtui  # ArchLinux. (Note: the latest clashtui may not be uploaded yet — please build and install it manually)
```

This step ensures mihomo, sing-box, and clashtui are available in your environment so the install script will skip downloading them. You can also download them manually and run `which mihomo sing-box clashtui` to verify they are correctly configured.

2. Run the install script:

```sh
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/demotui/install) --repo JohanChane/clashtui --branch demotui --core all
```

Tip: The install script downloads resources from GitHub. If downloads keep failing, try enabling a proxy before running the script.

3. \[Optional\] Enable `clashtui_mihomo.service` / `clashtui_singbox.service` on boot:

```sh
sudo systemctl enable clashtui_mihomo.service
# OR
sudo systemctl enable clashtui_singbox.service
```

### Without root access (no TUN)

```sh
bash <(curl -fsSL https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/demotui/install) --repo JohanChane/clashtui --branch demotui --core all --is-user
```

## Documentation

| Document | Description |
|----------|-------------|
| [Getting Started](docs/getting_started_en.md) | Detailed usage guide: UI operations, CLI, subscription management, config reference, FAQ |
| [Feature Design](docs/ClashTui_feature_design_en.md) | Feature design: config structure, subscription management, Template expansion, sing-box merge algorithm |
| [Architecture](docs/architecture_en.md) | Code architecture: module structure, startup flow, TUI event loop, Tab system |

## Contributing

Issues and pull requests are welcome.

To get up to speed quickly:
1. [Feature Design](docs/ClashTui_feature_design_en.md) — understand the feature design
2. [Architecture](docs/architecture_en.md) — understand the code structure
