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
├── mihomo
│   ├── clashtui_mihomo.service       # Mihomo Core systemd unit file
│   ├── config                        # Core Config Dir
│   │   ├── config.yaml               # Core Config Path
│   └── mihomo -> /usr/bin/mihomo
└── sing-box
    ├── clashtui_singbox.service
    ├── config                        # Core Config Dir
    │   ├── config.json               # Core Config Path
    └── sing-box -> /usr/bin/sing-box
```

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
  edit_cmd: kitty -e nvim "%s"
  open_dir_cmd: kitty -e yazi "%s"
```

Design principle: Mihomo and sing-box cannot be used together, so they are placed in separate `mihomo` and `sing-box` sections.

## ClashTui Core File Management Design

ClashTui uses Linux group file permissions to manage Core files: the user only needs to join the group that owns each Core's files.

File permission detection and repair:
- On ClashTui startup, obtain the group name of the Core directory (e.g. `/opt/clashtui/mihomo`)
- Recursively check whether files under the Core directory have consistent group names
- If inconsistent, repair uniformly. Otherwise, do nothing.
- Also ensure the Core directory has the group sticky bit set.

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

*When Mihomo proxy-providers and rule-providers don't have a path, the path is set to `<md5 of url>.yaml`. ClashTui needs to support this convention.*

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
- Proxy-providers also have path info for URL files, e.g.: placed in `~/.config/clashtui/sing-box/proxy-providers/<md5 of url>.yaml`
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
