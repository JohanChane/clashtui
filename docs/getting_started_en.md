# Clashtui User Guide

Clashtui is a terminal tool for managing Mihomo (Clash.Meta) and sing-box proxy cores. In a clean interface, you can switch nodes, update subscriptions, view connection status, and control service start/stop.

## Download & Install

Download the appropriate platform binary (e.g. `clashtui-linux-amd64`) from [GitHub Releases](https://github.com/JohanChane/clashtui/releases), extract it, and place it in your `PATH`:

```sh
chmod +x clashtui
sudo mv clashtui /usr/local/bin/
```

## Starting

```sh
clashtui
```

On first run, it will automatically create the config directory and default files under `~/.config/clashtui`.

### Specifying a Config Directory

Use the `--config-dir` argument or `CLASHTUI_CONFIG_DIR` environment variable to specify a different directory:

```sh
clashtui --config-dir /my/config/path
```

If a `data/` subdirectory exists next to the executable, it will be used automatically (portable mode, suitable for USB drives).

## TUI Interface

After entering the TUI, a row of tabs appears at the top, with a hint bar at the bottom.

### Tabs

| Key | Tab | Description |
|-----|-----|-------------|
| `1` | Status | View core status: up/down rate, memory, uptime |
| `2` | Files | Manage subscriptions (Profiles) and templates |
| `3` | Proxies | Switch proxy nodes, view latency, manage proxy groups |
| `4` | Connections | View all current connections, close individual or all connections |
| `5` | Logs | View core logs in real time |
| `6` | Settings | Modify Clashtui settings |
| `7` | CoreSrvCtl | Control core services: start, stop, restart, switch between Mihomo / sing-box |

### Shortcuts

**Global (works on all pages):**

| Shortcut | Action |
|----------|--------|
| `1` ~ `7` | Jump to corresponding tab |
| `Tab` | Next tab |
| `q` or `Ctrl-c` | Quit |
| `?` | Show shortcut help |
| `Ctrl-g` then `c` | Open Clashtui config directory in file manager |
| `Ctrl-g` then `m` | Open core config directory |
| `Ctrl-g` then `f` | Start core service |
| `Ctrl-g` then `t` | Close all connections |

> For page-specific shortcuts, press `?` in each tab.

### Custom Key Bindings

Create a `keymap.yaml` file in the config directory to modify key bindings for each page. For example, changing up/down navigation to `j` `k`:

```yaml
proxies:
  j: SelectDown
  k: SelectUp
  enter: ToggleExpand

connections:
  j: SelectDown
  k: SelectUp

settings:
  j: SelectDown
  k: SelectUp
  enter: Edit

file:
  profile:
    j: SelectDown
    k: SelectUp
  template:
    j: SelectDown
    k: SelectUp
```

You can also use the list format to customize descriptions:

```yaml
proxies:
  - key: j
    action: SelectDown
    desc: Move down
  - keys: ["g", "g"]
    action: ToggleExpand
    desc: Expand/Collapse
```

## CLI Mode

You can operate without entering the TUI, suitable for scripting or automation.

### Managing Subscriptions (Profiles)

```sh
# View current profile
clashtui profile select

# List all profiles
clashtui profile list

# Show names only
clashtui profile list --name-only

# Filter by type: file / url / template / singbox
clashtui profile list --type url

# Switch to a specific profile
clashtui profile select --name "my-subscription"

# Update current profile
clashtui profile update --name "my-subscription"

# Update all profiles (including the current one)
clashtui profile update --all

# Update with proxy
clashtui profile update --all --with-proxy
```

With the above commands, you can use [cron](https://wiki.archlinux.org/title/Cron) to schedule periodic profile updates.

### Switching Mode

```sh
# View current mode
clashtui mode

# Set to Rule mode
clashtui mode rule

# Set to Global mode
clashtui mode global

# Set to Direct mode
clashtui mode direct
```

### Controlling Services

```sh
# Hard restart (via systemd)
clashtui service restart

# Soft restart (via API, without restarting the process)
clashtui service restart --soft

# Stop service
clashtui service stop
```

### Checking for Updates

```sh
# Check Clashtui updates
clashtui update clashtui

# Check Mihomo core updates
clashtui update mihomo
```

## Profile (Subscription) Types

Clashtui supports three subscription types:

### 1. File Profile

Directly select a local YAML (Mihomo) or JSON (sing-box) config file as a profile. The file is not modified; Clashtui only applies overrides on top of it.

### 2. URL Profile

Enter a proxy subscription link. Clashtui will automatically download and track updates. When updating, proxy-provider resources are also downloaded.

### 3. Template Profile

The most flexible approach. You write a template file defining the skeleton (DNS, route rules, inbounds, etc.), then manage proxy node groups via `template_proxy_providers.yaml`.

**How Templates Work:**

- Templates are placed in the `mihomo/templates/` (or `sing-box/templates/`) directory
- Node groups are defined in `template_proxy_providers.yaml`
- Templates use variables like `${PPG.group_name}` to reference nodes
- Clashtui automatically expands variables into specific proxies, generating the final config file

A simple example: in the Files tab, create a Template-type Profile, select the corresponding template file and proxy-provider groups, and Clashtui will automatically synthesize a usable final configuration.

## Config File Reference

### config.yaml

Clashtui's main config, defining core paths and service names:

```yaml
mihomo:
  core:
    config_dir: /opt/clashtui/mihomo/config     # Core config directory
    bin_path: /opt/clashtui/mihomo/mihomo       # Core binary path
    config_path: /opt/clashtui/mihomo/config/config.yaml  # Final config file path
  core_service:
    service_name: clashtui_mihomo                # systemd service name
    is_user: false                               # Whether it's a user service
singbox:
  core:
    bin_path: /opt/clashtui/sing-box/sing-box
    config_dir: /opt/clashtui/sing-box/config
    config_path: /opt/clashtui/sing-box/config/config.json
  core_service:
    service_name: clashtui_singbox
    is_user: false
timeout: null
extra:
  edit_cmd: kitty -e nvim "%s"      # Command for editing files, %s is replaced by file path
  open_dir_cmd: kitty -e yazi "%s"  # Command for opening directories
```

### core_override_config.yaml

The **top-level keys** of this file will override the corresponding keys of the subscription config when switching profiles.

```yaml
mixed-port: 7890
allow-lan: false
mode: Rule
log-level: info
```

For example, no matter what port your subscription specifies, `mixed-port` will always be `7890`.

> This is Mihomo-specific. For sing-box, edit `sing-box/core_override_config.json`.

### core_override_config.json (sing-box)

sing-box uses deep merge. You only need to write the parts you want to override; other fields retain the original subscription values.

```json
{
  "experimental": {
    "clash_api": {
      "external_controller": "127.0.0.1:9090",
      "secret": ""
    }
  },
  "inbounds": [
    {
      "type": "mixed",
      "tag": "mixed-in",
      "listen": "127.0.0.1",
      "listen_port": 7890
    }
  ],
  "log": { "level": "info" }
}
```

Override rules:
- Objects: recursive merge (your fields override the subscription's; subscription-only fields are kept)
- Arrays: full replacement (what you write is exactly what you get)
- Numbers/Strings: direct override

### Config Directory Full Structure

```
~/.config/clashtui/
├── clashtui.db                     # Database: saves profile list, current selection, etc.
├── clashtui.log                    # Log file
├── config.yaml                     # Main config
├── keymap.yaml                     # Custom key bindings (optional)
├── theme.yaml                      # Custom theme (optional)
├── mihomo/
│   ├── core_override_config.yaml   # Override config
│   ├── profiles/                   # Downloaded subscription raw files
│   ├── templates/                  # Template files
│   ├── provider-cache/             # Provider cache
│   └── template_proxy_providers.yaml  # Template proxy node groups
└── sing-box/
    ├── core_override_config.json
    ├── profiles/
    ├── templates/
    ├── proxy-providers/
    └── template_proxy_providers.yaml
```

## Core Management

### Switching Cores

In the CoreSrvCtl tab (press `7`), you can switch between Mihomo and sing-box cores. After switching, restart Clashtui.

Clashtui automatically detects whether the currently running core type matches the configured one. If there's a mismatch, a popup will warn you and prevent displaying incorrect data.

### File Permissions (Linux)

Clashtui uses Linux group file permissions to manage access to core directories. On startup, it automatically checks file permissions under `config_dir` and will prompt you to confirm if repairs are needed.

## Logging

Logs are written to `<config_dir>/clashtui.log`. To increase log verbosity for troubleshooting:

```sh
clashtui -v     # More info
clashtui -vv    # Debug info
```

## FAQ

**In non-is-user mode, the core can't download rule files after starting?**
Non-`is_user` installations enable TUN by default, which may prevent the core from downloading required startup files (like rule-set, geoip, etc.). A temporary workaround: switch mode to Direct (`clashtui mode direct`), wait for the core to download the needed files, then switch back to the original mode (`clashtui mode rule`).

**No data displayed after startup?**
Make sure the Mihomo or sing-box core has been started. Go to the CoreSrvCtl tab (press `7`) to start the service.

**Seeing "core mismatch" warning?**
This means the running core doesn't match Clashtui's configured core. Go to the CoreSrvCtl tab to confirm which core you actually want to use, switch to it, then restart Clashtui.

**How do I add a subscription?**
Go to the Files tab (press `2`), press `?` to see shortcuts. Typically press `a` to add a new Profile, then enter the subscription URL. Clashtui will automatically download and update.

**How do I create a sing-box template subscription?**
1. Place a JSON template file in `sing-box/templates/`
2. Define your node groups and URLs in `template_proxy_providers.yaml`
3. In the Files tab, create a Template type Profile and select the template you just wrote

**What is the profiles directory in the config directory for?**
It stores the raw content of your subscriptions. These files are not affected by override configs — overrides only take effect when switching subscriptions and writing to the core config directory.
