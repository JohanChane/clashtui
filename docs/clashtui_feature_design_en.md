# ClashTui Feature Design

## Feature Classification

- Core API-related features:
    - Status, Proxies, Connections, and Settings tabs
- Non-API features (may also use API):
    - Files tab
        - Profile panel
        - Template panel
    - CoreSrvCtl tab

## ClashTui File Structure Design

ClashTui config file structure:

```
.
├── clashtui.db                     # Stores ClashTui's persistent data
├── clashtui.log                    # ClashTui logs
├── config.yaml                     # ClashTui configuration
├── mihomo
│   ├── core_override_config.yaml   # When generating the config_path file, this file's top-level keys override the Profile's top-level keys
│   ├── profiles                    # YAML files corresponding to Profiles (Mihomo config format is YAML)
│   ├── template_proxy_providers.yaml    # Stores pvd_name-url pairs needed for generating template type profiles
│   └── templates                   # Template storage directory
└── sing-box
    ├── proxy-providers             # Root directory for proxy-provider files
    ├── core_override_config.json
    ├── profiles                    # JSON files corresponding to Profiles (sing-box config format is JSON)
    ├── template_proxy_providers.yaml
    └── templates
```

ClashTui Core file structure design:

```
.
├── bin
│   └── clashtui -> /home/johan/.local/bin/clashtui
├── mihomo
│   ├── clashtui_mihomo.service       # Mihomo Core systemd unit file
│   ├── config                        # Core Config Dir
│   │   └── config.yaml               # Core Config Path
│   └── mihomo -> /usr/bin/mihomo
└── sing-box
    ├── clashtui_singbox.service
    ├── config                        # Core Config Dir
    │   └── config.json               # Core Config Path
    └── sing-box -> /usr/bin/sing-box
```

## ServiceController

ClashTui supports multiple service managers, with the default automatically selected at compile time based on the platform:

| Controller      | Platform        | Implementation               | Description                  |
|-----------------|-----------------|------------------------------|------------------------------|
| Systemd         | Linux (default) | `systemctl` CLI              | systemd service management   |
| OpenRc          | Linux (optional)| `rc-service` CLI             | OpenRC service management    |
| WindowsService  | Windows (default)| `windows-service` Rust crate | Direct Windows SCM API       |
| Launchd         | macOS (default) | `launchctl` CLI              | launchd service management   |

Compile-time defaults:
- `cfg!(windows)` → WindowsService
- `cfg!(target_os = "macos")` → Launchd
- Other → Systemd

## ClashTui clashtui.db Format Design

```yaml
core_type: mihomo
mihomo:
  cur_profile:
  profiles:
sing-box:
  cur_profile:
  profiles:
```

Design principle: Mihomo and sing-box cannot be used together, so they are placed in separate `mihomo` and `sing-box` sections.

## ClashTui Config Design

```yaml
mihomo:
  core:
    config_dir: /opt/clashtui/mihomo/config
    bin_path: /opt/clashtui/mihomo/mihomo
    config_path: /opt/clashtui/mihomo/config/config.yaml
  core_service:
    service_name: clashtui_mihomo
    is_user: false
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
  edit_cmd: ghostty -e nvim "%s"
  open_dir_cmd: ghostty -e yazi "%s"
```

Design principle: Mihomo and sing-box cannot be used together, so they are placed in separate `mihomo` and `sing-box` sections.

## ClashTui Core File Management Design

ClashTui uses Unix group file permissions to manage Core files: the user only needs to join the group that owns each Core's files.

File permission detection and repair:
- On ClashTui startup, obtain the group name of the Core directory (e.g. `/opt/clashtui/mihomo`)
- Recursively check whether files under the Core directory have consistent group names
- If inconsistent, repair uniformly. Otherwise, do nothing.
- Also ensure the Core directory has the group sticky bit set.

This works on both Linux and macOS (true implementations in `macos.rs`, not stubs).

To let the user know what was modified, ClashTui switches to CLI mode and prompts the user for their password. After fixing file permissions, ClashTui restarts.

## Mihomo and sing-box Config Merge Design

### Mihomo Config Merge

Use the top-level keys of `basic_core_config` to override the profile's top-level keys.

I believe Mihomo's merge rules are better than sing-box's because they are less prone to pollution. Mihomo's top-level sections have low coupling.

### sing-box Config Merge

Because sing-box's top-level sections have high coupling, the following merge approach is used.

sing-box merging is implemented by Clashtui itself via recursive deep merge, no longer depending on the external `sing-box merge` command.

Merge algorithm:

- Object: recursive merge. Keys present in the override replace the profile's corresponding values; keys unique to the profile are preserved (no intersecting keys).
- Array: full replacement. Arrays in the override completely replace the profile's corresponding arrays. This prevents ending up with multiple inbounds.
- Scalar (string, number, bool, null): direct override.

Merge timing: triggered when the user selects a profile. The flow is:

1. Read `sing-box/profiles/<profile_name>.json` as base
2. Read `sing-box/core_override_config.json` as overlay (if the file doesn't exist, skip merge and use profile as-is)
3. Recursively deep-merge the overlay into the base
4. Write the merged result to the core config path
5. Reload core service

`core_override_config.json` uses standard sing-box JSON syntax, with fields consistent with the sing-box config documentation.
Users only need to write the parts they want to override. For example, overriding only inbounds + experimental + log:

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
    },
    {
      "type": "tun",
      "tag": "tun-in",
      "stack": "gvisor",
      "auto_route": true,
      "address": ["172.19.0.1/30"]
    }
  ],
  "log": {
    "level": "info"
  }
}
```

Merge example:

```
profile.json:                           core_override_config.json:
{                                       {
  "inbounds": [                           "inbounds": [
    {"type":"mixed","port":12345},          {"type":"mixed","port":20122},
    {"type":"http","port":8080}             {"type":"tun","stack":"gvisor"}
  ],                                      ],
  "route": { "rules": [...],              "log": { "level": "debug" }
    "final": "entry" },                  }
  "experimental": {
    "clash_api": {
      "external_controller": "0.0.0.0:9090"
    }
  }
}
                        ↓ Recursive deep merge ↓
Result (config.json):
{
  "inbounds": [                           ← Array fully replaced
    {"type":"mixed","port":20122},
    {"type":"tun","stack":"gvisor"}
  ],
  "route": { "rules": [...],              ← Object preserved (override didn't touch)
    "final": "entry"
  },
  "experimental": {                       ← Object recursively merged
    "clash_api": {
      "external_controller": "127.0.0.1:9090",  ← Scalar overridden
      "secret": ""                              ← Newly added
    }
  },
  "log": { "level": "debug" }            ← Newly added
}
```

Design rationale:

- Unlike Mihomo's entire-top-level-key replacement, sing-box needs deep merge because users may only want to override inbounds without losing the profile's route/dns/outbounds.
- Using standard sing-box JSON syntax lowers the learning curve; users can reference the sing-box documentation directly.
- Not depending on `sing-box merge` avoids version compatibility issues with external commands, and the merge logic is fully controlled by Clashtui.
- Array full replacement (rather than element-level merge) is consistent behavior with GUI.for.SingBox and has clear semantics: whatever inbounds the user writes are exactly what they get.

## Profile Management Design

Profiles' information is stored in `clashtui.db`, with the following format:

```yaml
mihomo_cur_profile: my
singbox_cur_profile: johan
mihomo_profiles:
  my:
    dtype: !Url https://example.com
    no_pp: false
  file:
    dtype: !File
    no_pp: false
  template:
    dtype: !Template
    no_pp: false
  common_tpl.yaml.tpl:    # Template type profile names end with a `.tpl` suffix
    dtype: !Template
      template: common_tpl.yaml
        proxy_provider_group:
          pvd:
            foo_pvd: https://example.com
            bar_pvd: https://example.com
    no_pp: false
singbox_profiles:
  my:
    dtype: !Url https://example.com
    no_pp: false
  file:
    dtype: !File
    no_pp: false
  template:
    dtype: !Template
    no_pp: false
  common_tpl.json.tpl:    # Template type profile names end with a `.tpl` suffix
    dtype: !Template
      template: common_tpl.json
        proxy_provider_group:
          pvd:
            foo_pvd: https://example.com
            bar_pvd: https://example.com
    no_pp: false
```

The corresponding yaml/json profile is retrieved by profile name from `profiles/<profile_name>.{yaml | json}`.

Profiles cannot be renamed. If a user wants to rename, they must delete + import, so this management approach is feasible.

Files under the profiles directory are the profile's raw files, unaffected by other factors (e.g. the `no_pp` option).

File/Url Profile import:
- If the user input is a file path, the profile type is `File`
- If it is a URL, the type is `Url`

File/Url Profile update:
- If it's a URL Profile, update the profile content first
- Ensure the profile file is stored in the profiles directory
- Retrieve the profile's network resources (proxy-providers and rule-providers), then update them to the corresponding directories under Core Config Dir

File/Url Profile selection:
- Refer to the config merge design

Why not use the API to update Profiles:
- Because the API does not return values when updating Profiles (you don't know if the update succeeded), so you don't know what needs updating.
- Therefore, implementing profile updates ourselves provides a better experience.

*When Mihomo proxy-providers and rule-providers don't have a path, the path defaults to `proxies/<md5 of url>`. ClashTui needs to support this convention.*

## Template Management Design

Concept definitions:
- raw profile: a format close to core config. For example: file/url profiles are raw profiles, while template profiles are not — the files generated from them are raw profiles.

Because I prefer grouping each proxy-provider rather than mixing them together, I designed the Template feature.

Mihomo/sing-box template profile generation:
- Directly merge the template content with `template_proxy_providers` (placed at the front of the file)
- Then place the merged file in the profiles directory

    For example:

    ```yaml
    clashtui:
      proxy_provider_groups:
        pvd: # proxy-provider group name
          foo_pvd: https://example.com
          bar_pvd: https://example.com

    # template file content
    ...
    ```

- clashtui.db record:

    ```yaml
      common_tpl.yaml.tpl:
        dtype: !Template
          template: common_tpl.yaml
    ```

Template files primarily contain the following information:
- Generate proxy-provider groups. E.g.: pvd {pvd0, pvd1, ...}
- Generate a proxy-group for each proxy-provider:

    For example:

    ```yaml
    - name: "At"
      expand_group_with: ["${pvd}"] # Can also specify multiple proxy-provider names, e.g. ["${PPG.pvd.pvd0}", "${PPG.pvd.pvd2}"]
      type: url-test
      <<: *pa_dt
    ```

    Will expand to `At-pvd0, At-pvd1, ...`

- Use proxy-provider groups in proxy-groups:
    - For example: using `${pvd}` means using the proxy-provider group. It will be expanded to `pvd0, pvd1, ...`

A key point of Templates is that the template file content does not include proxy-provider proxy names, so you just need to write the proxy-provider group name (pvd) and proxy-provider names (pvd0, pvd1, ...) to know what kind of file the template will generate.

In summary, as long as you provide proxy-provider names + proxy-provider URLs, you can generate a Profile file.

Similarly, the same applies to sing-box. For example:

Expanding outbounds for proxy-providers:

```json
  "outbounds": [
    {
      "type": "urltest",
      "tag": "auto-proxy",
      "expand_outbound_with": ["${PPG.pvd}"], // Can also specify multiple proxy-provider names, e.g. ["${PPG.pvd.pvd0}", "${PPG.pvd.pvd2}"]
      "url": "https://www.gstatic.com/generate_204",
      "interval": "5m",
      "tolerance": 50
    },
  ]
```

Proxy-provider expansion:

```json
  "outbounds": [
    {
      "type": "selector",
      "tag": "select-proxy",
      "outbounds": ["auto-proxy", "${PPG.pvd.pvd0}"],
      "default": "auto-proxy"
    },
  ]
```

Because sing-box doesn't support proxy-providers, the Template feature can be used as an alternative:
- When generating a Template type profile, store the URLs in the profile
- Proxy-providers also have path info for URL files, e.g.: placed in `~/.config/clashtui/sing-box/proxy-providers/<md5 of url>.json`
- With the above information, the proxy-provider functionality can be replaced.

Template type profile generation:
- Prerequisite: proxy-provider content has already been updated. If there's no content, update; otherwise, don't.
- From the "template generation" section above, we know how Profile content is generated — store it in the profiles directory (same for sing-box)
- Generate the profile info in clashtui.db

Template type profile update:
- Download YAML profiles' proxy_provider_urls to the proxy-providers directory (these files are used when selecting the profile)
- Update proxy_provider_urls to the corresponding paths (for sing-box, update to the proxy-providers directory)
- Do not regenerate the template profile. Only regenerate when entering the template. However, if the user's current profile is this profile, a selection operation must be performed.

Mihomo/sing-box template type profile selection:
- If any proxy_provider_url lacks a corresponding file, do not use the template profile to generate the raw profile (to prevent generating malformed raw profiles)
- Generate a raw profile from the template profile according to the template generation rules (this file is equivalent to a Url/File profile)
- Selection is the same as File/Url profiles, except the object being operated on is the raw profile generated from the template profile

*To prevent writing malformed files, profiles and proxy-providers should be tested with the core before writing — only write on success. (For template profiles, the test uses the raw profile generated from the template profile)*

## sing-box Template Example

```json
{
  "log": {
    "level": "info",
    "timestamp": true
  },
  "dns": {
    "servers": [
      {
        "tag": "dns-remote",
        "address": "https://1.1.1.1/dns-query",
        "address_resolver": "dns-direct",
        "detour": "entry",
        "strategy": "prefer_ipv4"
      },
      {
        "tag": "dns-direct",
        "address": "https://dns.alidns.com/dns-query",
        "address_resolver": "dns-direct",
        "detour": "direct"
      },
      {
        "tag": "dns-local",
        "address": "local",
        "detour": "direct"
      },
      {
        "tag": "dns-fake",
        "address": "fakeip"
      }
    ],
    "rules": [
      {
        "rule_set": ["geosite-geolocation-cn"],
        "server": "dns-direct"
      },
      {
        "rule_set": ["geosite-google"],
        "server": "dns-remote"
      },
      {
        "query_type": ["A", "AAAA"],
        "server": "dns-fake"
      },
      {
        "server": "dns-direct"
      }
    ],
    "final": "dns-direct",
    "strategy": "prefer_ipv4"
  },
  "inbounds": [
    {
      "type": "mixed",
      "tag": "mixed-in",
      "listen": "127.0.0.1",
      "listen_port": 7890
    },
    {
      "type": "tun",
      "tag": "tun-in",
      "address": ["172.19.0.1/30"],
      "mtu": 9000,
      "auto_route": true,
      "strict_route": true,
      "auto_redirect": true,
      "stack": "system"
    }
  ],
  "outbounds": [
    {
      "type": "selector",
      "tag": "entry",
      "outbounds": ["${PGG.auto}", "${PGG.select}", "${PPG.pvd}"] // OR `"outbounds": ["${PGG.auto}", "${PGG.select}", "${PPG.pvd.pvd0}", "${PPG.pvd.pvd2}"],`
    },
    // `"${PG.auto}"` will expand to `auto-pvd0, auto-pvd1, ...`
    {
      "type": "urltest",
      "tag": "auto",
      "expand_group_with": ["${PPG.pvd}"], // OR `"expand_group_with": ["${PPG.pvd.pvd0}", "${PPG.pvd.pvd2}"],`
      "url": "https://www.gstatic.com/generate_204",
      "interval": "5m",
      "tolerance": 50
    },
    // Similar to the group above
    {
      "type": "urltest",
      "tag": "select",
      "expand_group_with": ["${PPG.pvd}"], // OR `"expand_group_with": ["${PPG.pvd.pvd0}", "${PPG.pvd.pvd2}"],`
      "url": "https://www.gstatic.com/generate_204",
      "interval": "5m",
      "tolerance": 50
    },
    {
      "type": "direct",
      "tag": "direct"
    },
    {
      "type": "block",
      "tag": "block"
    },
    {
      "type": "dns",
      "tag": "dns-out"
    },
    // ===
    // Place proxy-provider proxies here whose type is not selector, urltest, etc.
    // ===
  ],
  "route": {
    "rule_set": [
      {
        "type": "remote",
        "tag": "geoip-cn",
        "format": "binary",
        "url": "https://github.com/SagerNet/sing-geoip/raw/rule-set/geoip-cn.srs",
        "download_detour": "direct",
        "update_interval": "7d"
      },
      {
        "type": "remote",
        "tag": "geosite-geolocation-cn",
        "format": "binary",
        "url": "https://github.com/SagerNet/sing-geosite/raw/rule-set/geosite-geolocation-cn.srs",
        "download_detour": "direct",
        "update_interval": "7d"
      },
      {
        "type": "remote",
        "tag": "geosite-google",
        "format": "binary",
        "url": "https://github.com/SagerNet/sing-geosite/raw/rule-set/geosite-google.srs",
        "download_detour": "direct",
        "update_interval": "7d"
      },
      {
        "type": "remote",
        "tag": "geosite-category-ads-all",
        "format": "binary",
        "url": "https://github.com/SagerNet/sing-geosite/raw/rule-set/geosite-category-ads-all.srs",
        "download_detour": "direct",
        "update_interval": "7d"
      }
    ],
    "rules": [
      {
        "rule_set": ["geosite-category-ads-all"],
        "outbound": "block"
      },
      {
        "rule_set": ["geoip-cn"],
        "outbound": "direct"
      },
      {
        "rule_set": ["geosite-geolocation-cn"],
        "outbound": "direct"
      },
      {
        "rule_set": ["geosite-google"],
        "outbound": "entry"
      },
      {
        "ip_is_private": true,
        "outbound": "direct"
      },
      {
        "protocol": ["bittorrent"],
        "outbound": "direct"
      },
      {
        "outbound": "entry"
      }
    ],
    "auto_detect_interface": true,
    "final": "entry"
  },
  "experimental": {
    "clash_api": {
      "external_controller": "127.0.0.1:9090",
      "external_ui": "dashboard",
      "secret": "",
      "default_mode": "Rule"
    },
    "cache_file": {
      "enabled": true,
      "path": "cache.db",
      "store_fakeip": true
    }
  }
}
```

template_proxy_providers.yaml:
```yaml
pvd:  # proxy-provider group name
  pvd0: https://example.com
  pvd1: https://example.com
```

Domains:
- PPG: proxy-provider group
- PGG: proxy-group group

Expansion rules:
- PPG: expands to proxies
- PGG: expands to proxy-group(s)

For example: expansion rules
- `"${PPG.pvd}"`: expands to proxies
- `"${PPG.pvd.pvd0}"`: expands to the proxies of proxy-provider `pvd0`
- `"${PGG.auto}"`: expands to proxy-group groups. E.g.: `auto-pvd0, auto-pvd1, ...`
- `"${PGG.auto.pvd0}"`: represents a single proxy-group, e.g. `auto-pvd0`

## Mihomo Template Example

Prerequisite: Familiarity with [mihomo configuration](https://wiki.metacubex.one/config/) and YAML syntax.

### Proxy-Providers Template

Purpose: Generates a proxy-provider for each subscription in `template_proxy_providers`.

For example:

```yaml
proxy-anchor:
  - delay_test: &pa_dt {url: https://www.gstatic.com/generate_204, interval: 300}
  - proxy_provider: &pa_pp {interval: 3600, health-check: {enable: true, url: https://www.gstatic.com/generate_204, interval: 300}}

proxy-providers:
  pvd:
    tpl_param:
    type: http    # The type field must be placed here, not in pa_pp. The reason is that ClashTUI uses this field to detect if it is a network resource.
    <<: *pa_pp
```

### Proxy-Groups Template

Purpose: Generates a Proxy-Group for each proxy-provider created by the Proxy-Providers template.

```yaml
proxy-groups:
  - name: "select"
    expand_group_with: ["${PPG.pvd}"]
    type: select

  - name: "auto"
    expand_group_with: ["${PPG.pvd}"]
    type: url-test
    <<: *pa_dt
```

### Using Proxy-Groups Template

Use `${auto}` to enclose the name of the Proxy-Group template to utilize each proxy-group generated by the Proxy-Group template.

For example:

```yaml
proxy-groups:
  - name: "entry"
    type: select
    proxies:
      - ${PGG.auto}
      - ${PGG.select}
```

---

template_proxy_providers.yaml:
```yaml
pvd:  # proxy-provider group name
  pvd0: https://example.com
  pvd1: https://example.com
```

## Resolving Mihomo/sing-box Proxy-Provider Proxy Name Conflicts

Steps:
- Place each proxy-provider into a Set
- Create a temporary set, then add the proxies of each proxy-provider in order
- If a name collision occurs, rename it to `<origin_name>-<proxy_provider_name>`
- Simultaneously record a rename entry: `Set name: [{origin_name, new_name}, ...]`

## Key Conflict Detection Design

Currently there are two layers of checking:
1. Runtime — when loading keymap.yaml, check for duplicate keys within the same section. If found, only log `log::warn!` — the config is not rejected.
2. Compile-time test — verify that within the default key bindings defined by the `mod_agent!` macro, each tab has no duplicate keys internally.

So if you customize keymap.yaml with duplicate keys in the same section, you'll get warn logs, but startup won't be blocked.

When keys conflict, the first one defined takes priority. Key combinations defined in keymap.yaml have higher priority than the defaults.

Key ambiguity:
- Within the same scope, one key combination is identical to another, or one is a subset of another.

## When API Data Doesn't Match the Current Core

The `core_type` in `clashtui.db` takes precedence. If the API returns core data that doesn't match the configuration, that data is invalid and must not be used — otherwise users would be confused by dashboard data from an unknown source.

### Mismatch Scenarios

- The user started another core's service outside of Clashtui
- Both cores happen to listen on the same port, and API requests return data from the wrong core
- CoreSrvCtl switched cores but Clashtui was not restarted (current design)

### Detection Method

**Identify the core via the `version` field from `/version`**:

- sing-box returns `"version": "sing-box 1.13.11"` → contains the `"sing-box"` substring
- mihomo returns `"version": "v1.18.10"` → does not contain it

> Note: sing-box ≥ 1.13 also returns `"meta": true` in clash API emulation — this field is not reliable.

### Architecture

**Two-layer defense**:

| Layer | Location | Mechanism |
|-------|----------|-----------|
| Panel layer | Each Tab's `on_enter` / `after_sync` | Block spawning async tasks or clear existing data |
| API layer | Entry of `request()` function | Uniformly reject non-`/version` requests, return `"core mismatch"` |

**Global flag**:

`config.rs` maintains `static CORE_MISMATCH: AtomicBool`:

- `set_core_mismatch(bool)` — write (only StatusTab)
- `is_core_mismatch() -> bool` — read (all panels + `request()`)

**Detection timing**:

`StatusTab.on_enter()` **synchronously** calls `detect_core_type()` (localhost HTTP, <10ms), setting `CORE_MISMATCH` before other panels fetch data for the first time. `after_sync` continues async polling detection (`or_set` silently).

**Popup**:

On first detection of a mismatch (when `detected_core_type` changes from `None` to a non-matching value), a Confirm popup is shown to the user. Subsequent mismatches do not trigger popups.

### Returning to Normal

`after_sync` keeps detecting. When `detected == configured`, the `CORE_MISMATCH` flag is automatically cleared. When the user switches to each panel, API requests resume normally and data is displayed again.

## Support macOS

### macOS Core File Structure (launchd)

ClashTui Core file structure (e.g. `/usr/local/opt/clashtui`):

```
.
├── bin
│   └── clashtui -> /usr/local/bin/clashtui
├── mihomo
│   ├── config                        # Core Config Dir
│   │   └── config.yaml               # Core Config Path
│   └── mihomo -> /usr/local/bin/mihomo
└── sing-box
    ├── config                        # Core Config Dir
    │   └── config.json               # Core Config Path
    └── sing-box -> /usr/local/bin/sing-box

launchd plist (stored separately):
  User Mode:   ~/Library/LaunchAgents/clashtui_mihomo.plist
               ~/Library/LaunchAgents/clashtui_singbox.plist
  System Mode: /Library/LaunchDaemons/clashtui_mihomo.plist
               /Library/LaunchDaemons/clashtui_singbox.plist
```

User mode default path is `~/.local/clashtui`, same as Linux.

### systemd vs launchd Comparison

| Operation     | Linux (systemd)                          | macOS (launchd)                                    |
|---------------|------------------------------------------|----------------------------------------------------|
| **User Mode** |                                          |                                                    |
| unit location | `~/.config/systemd/user/<name>.service`  | `~/Library/LaunchAgents/<name>.plist`              |
| start service | `systemctl --user start <name>`          | `launchctl load <plist>` (RunAtLoad starts immediately) |
| stop service  | `systemctl --user stop <name>`           | `launchctl unload <plist>`                         |
| check status  | `systemctl --user is-active <name>`      | `launchctl print gui/$UID/<name>`                  |
| auto-start    | `systemctl --user enable <name>`         | `launchctl load -w <plist>` (persists across reboots) |
| survive logout| `loginctl enable-linger` (supported)      | Not supported (stops on logout)                     |
| crash restart | `systemd service Restart=always`         | plist `KeepAlive=true`                             |
| **System Mode** |                                       |                                                    |
| unit location | `/usr/lib/systemd/system/<name>.service` | `/Library/LaunchDaemons/<name>.plist`              |
| start service | `sudo systemctl start <name>`            | `sudo launchctl load <plist>` (RunAtLoad starts immediately) |
| stop service  | `sudo systemctl stop <name>`             | `sudo launchctl unload <plist>`                    |
| check status  | `systemctl is-active <name>`             | `sudo launchctl print system/<name>`               |
| auto-start    | `sudo systemctl enable <name>`           | `sudo launchctl load -w <plist>` (persists across reboots) |
| run as        | Dedicated user (mihomo / sing-box)        | root (launchd system daemon)                       |
| TUN access    | Linux capabilities (setcap)              | sudo / root (no setcap on macOS)                    |

Key differences:
- **enable/disable concept**: systemd's `enable` only sets auto-start, `start` starts immediately. launchd's `load` starts the service now (RunAtLoad=true). `load -w` additionally persists auto-start across reboots by removing the disabled override. `unload` stops and removes from launchd. `unload -w` also marks as disabled for boot.
- **logout behavior**: launchd `LaunchAgents` stop on logout, no config can change this. `LaunchDaemons` (system mode) start at boot and survive login/logout cycles.
- **TUN permissions**: Linux uses `setcap` to grant capabilities, running TUN as non-root. macOS has no such mechanism; system mode runs as root for utun device access.

ClashTui service commands on macOS:

```sh
# Start now (system mode)
sudo launchctl load /Library/LaunchDaemons/clashtui_mihomo.plist
sudo launchctl load /Library/LaunchDaemons/clashtui_singbox.plist

# Start + auto-start on boot
sudo launchctl load -w /Library/LaunchDaemons/clashtui_mihomo.plist
sudo launchctl load -w /Library/LaunchDaemons/clashtui_singbox.plist

# Stop
sudo launchctl unload /Library/LaunchDaemons/clashtui_mihomo.plist
sudo launchctl unload /Library/LaunchDaemons/clashtui_singbox.plist

# Check status
sudo launchctl list | grep clashtui

# User mode (no sudo)
launchctl load ~/Library/LaunchAgents/clashtui_mihomo.plist
launchctl unload ~/Library/LaunchAgents/clashtui_mihomo.plist
launchctl load -w ~/Library/LaunchAgents/clashtui_mihomo.plist
```

> `load` starts the service immediately because `RunAtLoad=true` is set in the plist.
> `load -w` additionally persists auto-start across reboots.

### macOS File Permissions

macOS and Linux use a unified Unix group permission model for managing Core files:

| Item | Linux | macOS |
|---|---|---|
| Core dir owner | `mihomo:mihomo` / `sing-box:sing-box` | `root:admin` |
| Add user to group | `gpasswd -a $USER mihomo` | Not needed (macOS users are in `admin` group by default) |
| Dir SGID + group rwx | `chmod g+rwxs` | `chmod g+rwxs` (same) |
| Config file perms | `chown mihomo:mihomo` + `chmod g+r` | `chmod g+rw` |
| Startup permission check/repair | ✅ | ✅ (real impl in `macos.rs`, not stubs) |

Principle: On macOS system mode, Core services run as root (required for TUN), while regular users gain file read/write access through the `admin` group. At startup, ClashTui checks the Core directory's SGID bit, group consistency, and group writability. If inconsistent, it repairs via `sudo chmod g+s` / `sudo chown :<group>` / `sudo chmod g+w`.

### Launchd Plist Files

#### 1. User Mode — clashtui_mihomo.plist

Path: `~/Library/LaunchAgents/clashtui_mihomo.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>clashtui_mihomo</string>
    <key>ProgramArguments</key>
    <array>
        <string>~/.local/clashtui/mihomo/mihomo</string>
        <string>-d</string>
        <string>~/.local/clashtui/mihomo/config</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>~/Library/Logs/clashtui_mihomo.log</string>
    <key>StandardErrorPath</key>
    <string>~/Library/Logs/clashtui_mihomo.log</string>
    <key>WorkingDirectory</key>
    <string>~/.local/clashtui/mihomo/config</string>
</dict>
</plist>
```

#### 2. User Mode — clashtui_singbox.plist

Path: `~/Library/LaunchAgents/clashtui_singbox.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>clashtui_singbox</string>
    <key>ProgramArguments</key>
    <array>
        <string>~/.local/clashtui/sing-box/sing-box</string>
        <string>-D</string>
        <string>~/.local/clashtui/sing-box/config</string>
        <string>-c</string>
        <string>~/.local/clashtui/sing-box/config/config.json</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>~/Library/Logs/clashtui_singbox.log</string>
    <key>StandardErrorPath</key>
    <string>~/Library/Logs/clashtui_singbox.log</string>
    <key>WorkingDirectory</key>
    <string>~/.local/clashtui/sing-box/config</string>
</dict>
</plist>
```

#### 3. System Mode — clashtui_mihomo.plist

Path: `/Library/LaunchDaemons/clashtui_mihomo.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>clashtui_mihomo</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/opt/clashtui/mihomo/mihomo</string>
        <string>-d</string>
        <string>/usr/local/opt/clashtui/mihomo/config</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/usr/local/var/log/clashtui_mihomo.log</string>
    <key>StandardErrorPath</key>
    <string>/usr/local/var/log/clashtui_mihomo.log</string>
    <key>WorkingDirectory</key>
    <string>/usr/local/opt/clashtui/mihomo/config</string>
</dict>
</plist>
```

#### 4. System Mode — clashtui_singbox.plist

Path: `/Library/LaunchDaemons/clashtui_singbox.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>clashtui_singbox</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/opt/clashtui/sing-box/sing-box</string>
        <string>-D</string>
        <string>/usr/local/opt/clashtui/sing-box/config</string>
        <string>-c</string>
        <string>/usr/local/opt/clashtui/sing-box/config/config.json</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/usr/local/var/log/clashtui_singbox.log</string>
    <key>StandardErrorPath</key>
    <string>/usr/local/var/log/clashtui_singbox.log</string>
    <key>WorkingDirectory</key>
    <string>/usr/local/opt/clashtui/sing-box/config</string>
</dict>
</plist>
```

## Support Windows

### Windows Core File Structure

ClashTui Core file structure (System mode, e.g. `C:\Program Files\clashtui`):

```
.
├── bin
│   └── clashtui.exe
├── mihomo
│   ├── config                          # Core Config Dir
│   │   └── config.yaml                 # Core Config Path
│   └── mihomo.exe -> C:\bin\mihomo.exe # or place .exe directly
└── sing-box
    ├── config                          # Core Config Dir
    │   └── config.json                 # Core Config Path
    └── sing-box.exe -> C:\bin\sing-box.exe
```

User mode default path is `%LOCALAPPDATA%\clashtui` (e.g. `C:\Users\<User>\AppData\Local\clashtui`).

ClashTui config file structure is the same as Linux/macOS, stored at `%APPDATA%\clashtui` (e.g. `C:\Users\<User>\AppData\Roaming\clashtui`).

Like Linux/macOS, Windows supports symlinks (`mklink` / `mklink /D`) to point to binary paths, but requires Administrator privileges. Without admin rights, users can place `.exe` files directly.

### Core Services Management (Windows SCM API)

Windows uses the Rust [`windows-service`](https://crates.io/crates/windows-service) crate to directly call the Windows SCM (Service Control Manager) API — no external tools required (sc.exe / WinSW / NSSM). Both Clash Verge Rev and FlClash use the same approach.

#### systemd vs launchd vs Windows SCM Comparison

| Operation       | Linux (systemd)                          | macOS (launchd)                               | Windows (SCM API)                                      |
|-----------------|------------------------------------------|-----------------------------------------------|--------------------------------------------------------|
| **User Mode**   |                                          |                                               |                                                        |
| install service | `systemctl --user link <unit>`           | (plist in `~/Library/LaunchAgents/` = installed)| `ServiceManager::create_service()`                     |
| uninstall       | `systemctl --user disable <name>`        | (delete plist + `launchctl bootout`)           | `service.stop()` → `service.delete()`                  |
| start service   | `systemctl --user start <name>`          | `launchctl bootstrap gui/$UID <plist>`         | `service.start()`                                      |
| stop service    | `systemctl --user stop <name>`           | `launchctl bootout gui/$UID/<name>`            | `service.stop()`                                       |
| check status    | `systemctl --user is-active <name>`      | `launchctl print gui/$UID/<name>`              | `service.query_status()` → `ServiceState`              |
| crash restart   | `Restart=always` (unit file)             | `KeepAlive=true` (plist)                       | `SERVICE_CONFIG_FAILURE_ACTIONS` (via SCM API)          |
| **System Mode** |                                          |                                               |                                                        |
| install service | `sudo systemctl link <unit>`             | `sudo launchctl bootstrap system <plist>`      | `ServiceManager::create_service()` (Admin required)     |
| uninstall       | `sudo systemctl disable <name>`          | `sudo launchctl bootout system/<name>`         | `stop()` → `delete()`  (Admin required)                 |
| start service   | `sudo systemctl start <name>`            | `sudo launchctl bootstrap system <plist>`      | `service.start()` (Admin required)                      |
| stop service    | `sudo systemctl stop <name>`             | `sudo launchctl bootout system/<name>`         | `service.stop()` (Admin required)                       |
| check status    | `systemctl is-active <name>`             | `sudo launchctl print system/<name>`           | `service.query_status()`                                |
| TUN access      | `setcap` (Linux capabilities)            | root (no setcap)                               | Administrator suffices (LocalSystem by default)         |

Key differences:
- **Zero external dependencies**: The `windows-service` crate calls the SCM API directly — the SCM is a core Windows OS component. No third-party tools need to be installed by the user.
- **Type-safe API**: Uses Rust's strong typing (`ServiceState`, `ServiceType`, `ServiceStartType`) instead of parsing CLI string output, avoiding parsing errors.
- **Crash restart**: Configured via SCM API `ChangeServiceConfig2W` + `SERVICE_CONFIG_FAILURE_ACTIONS`. The `windows-service` crate doesn't expose this directly yet; may require supplemental `windows` crate calls, or post-install `sc failure` configuration.

#### Installation Example

ClashTui calls the SCM API directly (no external commands):

```rust
// Pseudocode
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

let manager = ServiceManager::local_computer(None, ServiceManagerAccess::CREATE_SERVICE)?;
let service = manager.create_service(
    &ServiceInfo {
        name: "clashtui_mihomo".into(),
        display_name: "ClashTui Mihomo".into(),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: r"C:\Program Files\clashtui\mihomo\mihomo.exe",
        launch_arguments: vec![r#"-d "C:\Program Files\clashtui\mihomo\config""#.into()],
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    },
    ServiceAccess::START | ServiceAccess::STOP,
)?;
```

#### CoreSrvCtl New Operations

Since Windows command line is inconvenient, CoreSrvCtl tab provides three additional operations on Windows:

**1. Install Srv**

- Calls SCM API via the `windows-service` crate: `ServiceManager::create_service()`
- service type: `OWN_PROCESS`, start type: `AutoStart`, account: `LocalSystem` (Administrator privileges)
- `executable_path` = `bin_path`, `launch_arguments` derived from CoreType
- After installation, service status becomes `installed`
- Optional: post-install crash restart configuration via `sc failure`

**2. Uninstall Srv**

- If the service is running, first `service.stop()`
- Then `service.delete()` to remove the service
- After uninstallation, service status becomes `uninstalled`

**3. Toggle System Proxy**

Based on clashtui v0.2.3 implementation, system proxy is toggled via Windows Registry:

| Interface              | Action                                                                    |
|------------------------|--------------------------------------------------------------------------|
| Check proxy status     | Read `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings\ProxyEnable` (REG_DWORD): `0` = disabled, `1` = enabled |
| Enable system proxy    | `ProxyEnable` → `1`; `ProxyServer` → `127.0.0.1:<port>`; `ProxyOverride` → `<-loopback>`; broadcast `WM_SETTINGCHANGE` |
| Disable system proxy   | `ProxyEnable` → `0`; broadcast `WM_SETTINGCHANGE`                        |

The proxy port is obtained from the core's mixed port (typically `7890`) via REST API `GET /configs` reading the mixed inbound's `listen_port`.

Implementation: Use the `winreg` crate to directly manipulate the registry (recommended for better error handling). After modifying registry values, call `SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, ...)` to notify the system to refresh proxy settings.

#### CoreSrvCtl Operation List (Windows)

| Operation        | Description                                                 |
|------------------|-------------------------------------------------------------|
| Stop Service     | Stop the current core's service                             |
| Start Service    | Start the current core's service                            |
| Install Srv      | Install current core as Windows Service (SCM API create_service) |
| Uninstall Srv    | Uninstall current core's Windows Service (stop then delete) |
| Toggle SysProxy  | Toggle system proxy (enable/disable)                        |
| Switch Core      | Switch to the other core (mihomo ↔ sing-box)                |
| Stop All         | Stop all core services                                      |

#### Service States

| State          | Meaning                                  |
|----------------|------------------------------------------|
| `active`       | Service is running                       |
| `inactive`     | Service installed but not running         |
| `installed`    | Windows Service registered (not started)    |
| `uninstalled`  | No Windows Service found (needs Install)    |
| `?`            | Unable to determine (e.g. insufficient permissions) |

State detection priority:
1. Query via SCM API `service.query_status()` to get `ServiceState`
2. `Running` → `"active"`, `Stopped` → `"inactive"`
3. Service not found (ERROR_SERVICE_DOES_NOT_EXIST) → `"uninstalled"`

### File Permissions

Windows uses NTFS ACL (Access Control List) for file permissions, which is fundamentally different from Unix mode bits.

**Windows strategy:**
- `check_file_permissions()` → always returns `true` (permissions always considered OK)
- `repair_file_permissions()` → always returns `Ok("Permissions OK on Windows")` (no repair needed)
- `correct_cap_for_tun()` → always returns `Ok("No setcap on Windows")` (TUN capabilities managed by the core)
- `check_startup_perms()` → no-op (skip permission checks)

Rationale: Windows' permission model is based on user/group ACLs — concepts like group sticky bit and mode bits do not exist. Core files running as Administrator have sufficient privileges. The regular user's TUI tool only needs read/write access to `%APPDATA%\clashtui` config directory (which users have by default).

### CoreSrvCtl Tab Windows Adaptation

#### Current State — CoreSrvCtl Operations

```rust
enum SrvCtlOp {
    Stop,        // "Stop Service"
    Restart,     // "Start Service"
    SwitchCore,  // "Switch Core"
    StopAll,     // "Stop All Services"
}
```

#### Windows Extensions

On Windows, when `ServiceController::default()` returns `WindowsService`, three additional operations are available:

```rust
#[cfg(windows)]
SrvCtlOp::Install,       // "Install Service" — SCM API create_service
#[cfg(windows)]
SrvCtlOp::Uninstall,     // "Uninstall Service" — stop + delete
#[cfg(windows)]
SrvCtlOp::ToggleSysProxy, // "Toggle System Proxy" — registry read/write
```

**Install** execution logic:
1. Call `ServiceManager::create_service()` via the `windows-service` crate
2. service type: `OWN_PROCESS`, account: `LocalSystem`, start: `AutoStart`
3. `executable_path` = `bin_path`, `launch_arguments` derived from CoreType
4. Update status to `installed`

**Uninstall** execution logic:
1. Open service; if running, call `service.stop()`
2. Then `service.delete()`
3. Update status to `uninstalled`

**Toggle System Proxy** execution logic:
1. Read current `ProxyEnable` registry value
2. If currently disabled → enable: set `ProxyEnable=1`, `ProxyServer=127.0.0.1:<port>`, `ProxyOverride=<-loopback>`, broadcast `WM_SETTINGCHANGE`
3. If currently enabled → disable: set `ProxyEnable=0`, broadcast `WM_SETTINGCHANGE`
4. Mixed port obtained from REST API `GET /configs` mixed inbound config

#### Status Query Adaptation

The current srvctl status query hardcodes `systemctl is-active` for non-Launchd cases. It needs to be adapted:

```rust
match ServiceController::default() {
    ServiceController::Launchd => launchd_status(...),
    ServiceController::WindowsService => windows_service_status(...), // new
    _ => systemd_status(...),
}
```

`windows_service_status()` implementation:
1. Open service via `windows-service` crate → `service.query_status()`
2. Parse `ServiceState`:
   - `Running` → `"active"`
   - `Stopped` / `Paused` etc. → `"inactive"`
   - `ERROR_SERVICE_DOES_NOT_EXIST` → `"uninstalled"`

### Install Script

To lower the deployment barrier for Windows users, a PowerShell install script (`installs/install.ps1`) handles the following:

#### Features

1. **Choose install directory**: Default `C:\Program Files\clashtui`, user can specify a custom path via parameter (e.g. `D:\clashtui`)
2. **Create directory structure**: Automatically creates `mihomo/config/`, `sing-box/config/` etc.
3. **Copy files**:
   - Copy or prompt user to place `mihomo.exe` / `sing-box.exe` in the respective core directory
   - Copy `clashtui.exe` to `bin/`
4. **Register Windows Services**: clashtui registers both core services via the `windows-service` crate's SCM API (no external tools needed)
5. **Generate config.yaml template**: Auto-fill `bin_path` and `config_dir` with the user's chosen install directory

#### Usage

```powershell
# Default install to C:\Program Files\clashtui
.\installs\install.ps1

# Install to custom directory
.\installs\install.ps1 -InstallDir "D:\MyTools\clashtui"
```

#### Windows-specific Parameters

| Parameter      | Default                              | Description                 |
|----------------|--------------------------------------|-----------------------------|
| `-InstallDir`  | `C:\Program Files\clashtui`          | Installation root directory |

#### Post-Install File Structure

Assuming `-InstallDir "D:\clashtui"`:

```
D:\clashtui\
├── bin
│   └── clashtui.exe
├── mihomo
│   ├── config
│   │   └── config.yaml             # Core config (managed by clashtui)
│   └── mihomo.exe -> C:\bin\mihomo.exe  # symlink or direct copy
├── sing-box
    ├── config
    │   └── config.json             # Core config (managed by clashtui)
    └── sing-box.exe -> C:\bin\sing-box.exe
```

#### Service Registration

The script registers Windows Services via clashtui's own `clashtui service install` subcommand, which uses the `windows-service` crate to call the SCM API — no external tools required.

## Linux Platform Support for `openrc` Service Controller

### Overview

ClashTui defaults to `systemd` as the service manager on Linux. Support for `openrc` is added for Linux distributions running OpenRC (e.g. Gentoo, Alpine).

### Configuration Field

The `service_controller` field is added to `core_service` sections in `config.yaml`:

```yaml
core_service:
  service_name: clashtui_mihomo
  is_user: false
  service_controller: openrc   # Optional: systemd (default) / openrc
```

- `"systemd"` or absent → uses systemd
- `"openrc"` → uses OpenRC (`rc-service`/`rc-update`)
- Non-Linux platforms ignore this field, using platform default

### Runtime Behavior

| Operation    | systemd Command                    | OpenRC Command                    |
|-------------|------------------------------------|----------------------------------|
| Start        | `systemctl start <name>`           | `rc-service <name> start`        |
| Stop         | `systemctl stop <name>`            | `rc-service <name> stop`         |
| Restart      | `systemctl restart <name>`         | `rc-service <name> restart`      |
| Reload       | `systemctl reload <name>`          | `rc-service <name> restart`      |
| Status       | `systemctl is-active <name>`       | `rc-service <name> status`       |

`is_user` determines sudo/--user prefix:

- `is_user: false`: `sudo rc-service <name> <op>` (system mode)
- `is_user: true`: `rc-service --user <name> <op>` (user mode)
- OpenRC has supported user services since v0.60; scripts go in `/etc/user/init.d/`; requires `XDG_RUNTIME_DIR` to be set

### Install Script (`installs/install`)

The `install` script accepts a `--service-controller` argument:

```bash
# Default: systemd
./install --core all

# OpenRC (system mode)
./install --service-controller openrc --core all

# OpenRC (user mode)
./install --service-controller openrc --is-user --core all
```

When `--service-controller openrc`:
- System mode: generates OpenRC init scripts in `/etc/init.d/`, uses `command_user` for the service user
- User mode: generates OpenRC init scripts in `/etc/user/init.d/` (requires sudo to write), no `command_user` (runs as current user)
- `config.yaml` is written with `service_controller: openrc` and the corresponding `is_user` value
- Uses `supervise-daemon` to manage foreground processes

Generated OpenRC init script — system mode (mihomo):

```sh
#!/sbin/openrc-run

supervisor="supervise-daemon"
name="clashtui_mihomo"
description="mihomo Daemon, Another Clash Kernel."
command="/opt/clashtui/mihomo/mihomo"
command_args="-d /opt/clashtui/mihomo/config"
command_user="mihomo:mihomo"
pidfile="/run/${RC_SVCNAME}.pid"

depend() {
    need net
}
```

Generated OpenRC init script — user mode (mihomo):

```sh
#!/sbin/openrc-run

supervisor="supervise-daemon"
name="clashtui_mihomo"
description="mihomo Daemon, Another Clash Kernel."
command="/home/<user>/.local/clashtui/mihomo/mihomo"
command_args="-d /home/<user>/.local/clashtui/mihomo/config"

depend() {
    need net
}
```

### Uninstall (`--uninstall`)

`install --uninstall` correctly handles openrc mode:
- System mode: runs `sudo rc-update del`, `sudo rc-service stop`, removes scripts from `/etc/init.d/`
- User mode: runs `rc-update --user del`, `rc-service --user stop`, removes scripts from `/etc/user/init.d/`
