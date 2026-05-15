# ClashTui

<img width="2254" height="1254" alt="demo" src="https://github.com/user-attachments/assets/4059d70c-c7d8-4835-b177-0a768c32d91b" />


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
- **fzf Support** — Fuzzy finding support

## Installation

With root access (for TUN mode):

```sh
./install
```

Without root access (no TUN):

```sh
./install --is-user
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
