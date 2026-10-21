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

Use the `on / action / desc` list format in `keymap.yaml` in your config directory. The default Linux path is `~/.config/clashtui/keymap.yaml`. Configure only the bindings you want to change; omitted pages and bindings keep their built-in defaults.

```yaml
logs:
  - on: "x"
    action: Clear
    desc: Clear logs
  - on: ["g", "e"]
    action: GoBottom
    desc: Go to the end
  - on: "<C-j>"
    action: MoveDown

file:
  profile:
    - on: "e"
      action: Edit
      desc: Edit profile
```

- `on`: a single key string or a list of two keys. A modified key counts as one key, as in `["<C-g>", "c"]`. Write `gg` as `["g", "g"]`, not `"gg"`.
- Key notation includes `"j"`, `"G"`, `"<Enter>"`, `"<Up>"`, and `"<F1>"`. Prefixes `C-`, `A-`, `S-`, and `D-` mean Ctrl, Alt, Shift, and Super. Uppercase letters imply Shift; `"<S-g>"` and `"G"` are equivalent.
- `action`: an action for that page. Use a plain name, such as `Edit` on a file page or `SortByDelay` on the proxies page; `action: null` unbinds a key. Optional `desc` appears in the `?` help panel; omit it to use the action's default description.
- An identical complete key sequence replaces its default binding. Adding a new key for an action does not automatically remove the original key.

Use `action: null` to unbind a default sequence. For example, replace the default single key `j` with the chord `jj`:

```yaml
logs:
  - on: "j"
    action: null
  - on: ["j", "j"]
    action: MoveDown
```

Supported scopes are `app`, `connections`, `proxies`, `srvctl`, `settings`, `logs`, `file.profile`, and `file.template`. The two file scopes use the nested YAML structure shown above. The `app` scope configures global shortcuts, including the default number keys, Tab, `q`, `?`, and `<C-g>` chords. Conflict checking is limited to each scope; page bindings are not checked against application bindings.

Duplicate sequences and single-key/chord prefix conflicts (such as `d` and `dd`) stop startup with an error. Chords such as `gg` and `ge` can coexist. Restart after editing; keymaps are not reloaded live.

After the first key of a chord, a **Which?** panel appears near the bottom with sorted remaining keys and their descriptions, including custom `desc` values. The file page uses the focused profile or template scope. A matching second key executes the action and closes the panel. Esc or an unmatched key cancels and consumes that input, so plain `<Esc>` cannot be configured as a chord's second key. While a chord is pending, keys such as `q`, `?`, and Tab belong to that chord; cancel first to use their standalone actions. Narrow terminals use one column, and limited height shows a count of hidden choices.

The default global `<C-g>` shortcuts work on every page. Press Ctrl+g to show these second keys in the which panel:

| Second key | Action |
|---|---|
| `c` | Open the active core's data directory |
| `m` | Open the active core's installation directory (the parent of its `config_dir`) |
| `f` | Start/restart the active core service |
| `t` | Close all connections on the active core |

Directory actions use `extra.open_dir_cmd`, or the system default application when unset. Global and page chords can share a first key: for example, `gp` in `app` and `gg` on a page both remain available. The Which panel combines their candidates, and the second key selects the action. Only an identical complete chord gives the global binding priority, including its description in the hints. A chord prefix takes priority over a single key in the other scope; other global single keys run only when the page has not consumed the input. A pending second key cannot start another chord. Esc or an unmatched key cancels both scopes and consumes that input. Input and confirmation popups retain priority.

Global chords also work from the help panel, using only global candidates in that context. When help exceeds the screen height, scroll with Up/Down or `j`/`k`, use PageUp/PageDown to move a page, or Home/End to reach the beginning or end. Esc or another non-scrolling key closes help. Narrow terminals use a single column.

Customize global shortcuts under `app` using the same merge and unbinding rules as page scopes. For example, move restart from `<C-g> f` to `<C-g> r` and add F1 for help:

```yaml
app:
  - on: ["<C-g>", "r"]
    action: RestartCoreService
    desc: Restart core
  - on: ["<C-g>", "f"]
    action: null
  - on: "<F1>"
    action: Help
```

Available actions: `OpenCoreDataDir`, `OpenCoreInstallDir`, `RestartCoreService`, `CloseAllConnections`, `Quit`, `Help`, `TabNext`, `TabPrev`, and `TabStatus`, `TabFile`, `TabProxies`, `TabConnections`, `TabLogs`, `TabSettings`, and `TabCoreSrvCtl`. Use plain names; shell commands and action parameters are not supported. Help and which panels show the configured bindings and descriptions.

`TabNext` switches to the next page (default `<Tab>`); `TabPrev` switches to the previous page (default `<S-Tab>`). Both wrap around at the ends. You can rebind or unbind them in `app`.

The tab bar shows each page's configured navigation keys from `app`, including chords. Multiple bindings are separated by `/`. If all direct navigation bindings for a page are removed, its tab shows only the page name.

As in dev, place the same page sections under top-level `mihomo:` or `sing-box:` for core-specific configuration. An active core's page section replaces the corresponding common section before merging with built-in defaults.

Only the list format above is supported. Legacy structured Key mappings, `common/submap`, top-level `files/profile` paths, and nested actions such as `action: { Action: Edit }` are no longer accepted. Use nested `file.profile` sections and plain action names such as `action: Edit`.

See [`default_keymap.yaml`](../contrib/default_configs/default_keymap.yaml) for a complete reference. Missing files, empty files, and `{}` use the defaults.

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
