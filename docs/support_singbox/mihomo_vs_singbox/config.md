# mihomo vs sing-box: Configuration Comparison

## 1. Basic Config (top-level settings)

mihomo (`basic_clash_config.yaml` + profile YAML):

```yaml
external-controller: 127.0.0.1:9090
mixed-port: 7890
mode: Rule             # Rule | Global | Direct
tun:
  enable: true
  stack: System        # Mixed | Gvisor | System
log-level: info        # Silent | Error | Warning | Info | Debug
allow-lan: true
ipv6: false
profile:
  store-selected: true # stores proxy selection per group
```

sing-box (native JSON config):

```json
{
  "experimental": {
    "clash_api": {
      "external_controller": "127.0.0.1:9090",
      "secret": "your-secret"
    }
  },
  "inbounds": [
    { "type": "mixed", "tag": "mixed-in", "listen": "127.0.0.1", "listen_port": 7890 },
    { "type": "tun", "tag": "tun-in", "inet4_address": "172.19.0.1/30", "auto_route": true, "stack": "system" }
  ],
  "log": {
    "level": "info"
  }
}
```

Key differences:
- mihomo has `external-controller` at top level; sing-box nests it under `experimental.clash_api`
- mihomo has `mixed-port` as a top-level scalar; sing-box has `inbounds[]` entries — the HTTP/socks port is a `mixed` inbound
- mihomo has `tun.enable` boolean; sing-box has TUN as an `inbounds[]` entry
- mihomo has `log-level` top-level; sing-box nests under `log.level`
- mihomo has `profile.store-selected`; sing-box has no equivalent
- sing-box `clash_api` supports a `secret` field for API auth

---

## 2. Proxy Provider

mihomo:

```yaml
proxy-providers:
  my-provider:
    type: http
    url: https://example.com/proxies.yaml
    path: ./proxy-providers/my-provider.yaml
    interval: 3600
    health-check:
      enable: true
      url: https://www.gstatic.com/generate_204
      interval: 300
```

- Providers download **YAML** files containing proxy node lists
- Proxy-groups reference providers via `use: [my-provider]`
- demotui's "no proxy-provider" mode downloads and embeds all provider proxies inline
- Providers are cacheable to `$clash_config_dir/providers/`

sing-box:

```
(NOT SUPPORTED)
```

sing-box has **no concept of proxy-provider**. Users must:
- Include all proxy nodes explicitly in the profile JSON as `outbounds[]`
- Profile updates re-download the full JSON (no incremental proxy-provider refresh)
- The `no_pp` toggle is ignored for `ProfileType::Singbox` (shown as "N/A" in TUI)

---

## 3. Proxy Group

mihomo:

```yaml
proxy-groups:
  - name: Proxy
    type: select                  # select | url-test | fallback | load-balance
    proxies:
      - DIRECT
      - Auto                     # references another group by name
      - <TemplateParam>          # template placeholder
    use:                         # references proxy-providers
      - my-provider
  - name: Auto
    type: url-test
    proxies:
      - <TemplateParam>
    url: https://www.gstatic.com/generate_204
    interval: 300
    tolerance: 50
```

- Standalone top-level sequence: `proxy-groups: [...]`
- Groups can reference other groups by name in `proxies:`
- Groups can reference proxy-providers by name in `use:`
- Template placeholders (`<Name>`) are expanded during template processing
- `url-test` groups automatically pick the fastest proxy

sing-box:

```json
{
  "outbounds": [
    {
      "type": "selector",
      "tag": "Proxy",
      "outbounds": ["Auto", "node-hk-01", "node-hk-02"]
    },
    {
      "type": "urltest",
      "tag": "Auto",
      "outbounds": ["node-hk-01", "node-hk-02"],
      "url": "https://www.gstatic.com/generate_204",
      "interval": "5m"
    },
    {
      "type": "vless",
      "tag": "node-hk-01",
      "server": "1.2.3.4",
      "server_port": 443,
      "uuid": "...",
      "tls": { "enabled": true, "server_name": "example.com" }
    }
  ]
}
```

Key differences:

| Concept         | mihomo                                | sing-box                                         |
| --------------- | ------------------------------------- | ------------------------------------------------ |
| Location        | `proxy-groups:` (separate section)     | `outbounds[]` (same sequence as proxy nodes)     |
| Selector type   | `type: select`                        | `type: selector`                                 |
| URL-Test type   | `type: url-test`                      | `type: urltest`                                  |
| Children        | `proxies: [...]`                      | `outbounds: [...]`                               |
| References      | Group names + provider names          | Only `tag` of other outbound entries             |
| Provider refs   | `use: [provider-name]`                | Not supported (no proxy-provider concept)        |
| Interval format | Integer (seconds)                     | Duration string (`"5m"`, `"300s"`)               |

All outbounds (proxy nodes + groups) form a **flat list** in sing-box — groups reference nodes by their `tag`. In mihomo, proxies and groups are separate sections, and `use:` bridges them via proxy-providers.

---

## 4. Rules (路由)

mihomo:

```yaml
rules:
  - DOMAIN-SUFFIX,google.com,Proxy
  - DOMAIN-KEYWORD,chat,Proxy
  - GEOSITE,google,Proxy
  - GEOIP,CN,DIRECT
  - MATCH,Proxy

rule-providers:
  reject:
    type: http
    behavior: classical          # classical | domain | ipcidr
    url: https://...
    path: ./rule-providers/reject.yaml
    interval: 86400
```

- Inline string rules: `<MATCHER>,<VALUE>,<TARGET>` format
- `rule-providers` download external rule sets (YAML `payload`/`rules` list)
- Geo rules (`GEOSITE`, `GEOIP`) reference built-in geo databases
- Last rule is typically `MATCH` as catch-all

sing-box:

```json
{
  "route": {
    "rules": [
      { "domain_suffix": ["google.com"], "outbound": "Proxy" },
      { "domain_keyword": ["chat"], "outbound": "Proxy" },
      { "rule_set": "geosite-google", "outbound": "Proxy" },
      { "rule_set": "geoip-cn", "outbound": "direct" },
      { "outbound": "Proxy" }
    ],
    "rule_set": [
      {
        "tag": "geosite-google",
        "type": "remote",
        "format": "binary",
        "url": "https://github.com/.../geosite-google.srs"
      },
      {
        "tag": "geoip-cn",
        "type": "remote",
        "format": "binary",
        "url": "https://github.com/.../geoip-cn.srs"
      }
    ],
    "final": "Proxy"
  }
}
```

Rule translation map:

| mihomo matcher          | sing-box equivalent                                      | Notes                                      |
| ----------------------- | -------------------------------------------------------- | ------------------------------------------ |
| `DOMAIN-SUFFIX`         | `{ "domain_suffix": [...] }`                             | Value becomes a string array               |
| `DOMAIN-KEYWORD`        | `{ "domain_keyword": [...] }`                            | Value becomes a string array               |
| `DOMAIN`                | `{ "domain": [...] }`                                    | Full domain match                          |
| `DOMAIN-REGEX`          | Not natively supported in clash_api mode                 | Requires custom rule set (`.srs`)          |
| `GEOSITE,name`          | `{ "rule_set": "geosite-name" }`                         | Requires `rule_set[]` entry with `.srs` URL|
| `GEOIP,CN`              | `{ "rule_set": "geoip-cn" }`                             | Requires `rule_set[]` entry with `.srs` URL|
| `IP-CIDR,...`           | `{ "ip_cidr": [...] }`                                   | Array of CIDR strings                      |
| `PROCESS-NAME,...`      | `{ "process_name": [...] }`                              | Array of process name patterns             |
| `MATCH,...`             | `{ "outbound": "..." }` or `"final": "..."` in route     | Catch-all with no matcher fields           |
| `rule-providers`        | `route.rule_set[]`                                       | Remote `.srs` binary format only           |

Key differences:
- mihomo uses **inline string rules**; sing-box uses **structured JSON objects**
- mihomo geo-rules use **built-in databases** (`GEOSITE`/`GEOIP`); sing-box requires **external `.srs` binary files** referenced via `rule_set[]`
- mihomo has `rule-providers` that download YAML rulesets; sing-box uses `rule_set[]` that download `.srs` binary files
- sing-box has additional matchers not in mihomo: `process_name`, `process_path`, `wifi_ssid`
- sing-box's `final` field in `route` is the equivalent of mihomo's last `MATCH` rule

### Example: DOMAIN-REGEX → sing-box

mihomo:
```yaml
rules:
  - DOMAIN-REGEX,.*\.github\.com,Direct
```

sing-box (requires external `.srs` rule set, no native regex support in clash_api):
```
# Not directly translatable — must be converted to a .srs binary rule_set
# workaround: use domain_suffix ".github.com" which is functionally equivalent
{ "domain_suffix": ["github.com"], "outbound": "direct" }
```

---

## 5. Field Ownership (Basic vs Profile)

### Concept

Config is split into two layers at merge time:

| Layer   | Source                    | Semantics                             |
| ------- | ------------------------- | ------------------------------------- |
| Basic   | `basic_*_config` in demotui | System preferences, managed via Settings tab |
| Profile | Downloaded subscription    | Proxy nodes, groups, rules — managed by profile select/update |

During profile **enter**, profile content is merged onto basic config: **basic fields always win** (overwrite profile), except sequences which are **concatenated** (basic first, then profile appended).

### mihomo

**Profile fields** (all other fields are Basic):

| Profile field     | mihomo YAML key       | Purpose                              |
| ----------------- | --------------------- | ------------------------------------ |
| Proxy nodes       | `proxies`             | Individual proxy node definitions     |
| Proxy groups      | `proxy-groups`        | Selector / url-test / etc. groups     |
| Proxy providers   | `proxy-providers`     | Remote proxy node sources             |
| Rules             | `rules`               | Inline routing rules                  |
| Rule providers    | `rule-providers`      | Remote rule set sources               |
| Sub-rules         | `sub-rules`           | Nested / imported rule presets        |

**Basic fields** (everything else at top-level YAML):

`external-controller`, `mixed-port`, `mode`, `tun`, `log-level`, `allow-lan`, `ipv6`, `dns`, `sniffer`, `hosts`, `secret`, `profile`, `geodata-mode`, `find-process-mode`, `tcp-concurrent`, `unified-delay`, `keep-alive-interval`, etc.

Basic fields are stored in `basic_clash_config.yaml` and merged on top of any profile during `select()`.

### sing-box

**Profile fields** (all other fields are Basic):

| Profile field     | sing-box JSON path         | Purpose                              |
| ----------------- | -------------------------- | ------------------------------------ |
| Outbounds         | `outbounds[]`              | Proxy nodes + proxy groups (selector/urltest) — flat list |
| Route rules       | `route.rules[]`            | Inline routing rules                  |
| Route rule sets   | `route.rule_set[]`         | Remote `.srs` rule set references    |

Note: sing-box has **no** proxy-provider, sub-rules, or rule-provider concepts. All proxy nodes and groups live together in a flat `outbounds[]` sequence, identified by their `tag`.

**Basic fields** (everything else):

| Field             | sing-box JSON path              | Notes                              |
| ----------------- | ------------------------------- | ---------------------------------- |
| Clash API         | `experimental.clash_api`        | `external_controller`, `secret`    |
| Inbounds          | `inbounds[]`                    | mixed / tun / tun gateway entries  |
| Logging           | `log`                           | `log.level`                        |
| DNS               | `dns`                           | DNS servers, rules, etc.           |
| Route config      | `route` (minus `rules`/`rule_set`) | `route.auto_detect_interface`, `route.final`, etc. |
| Domain strategy   | `domain_strategy`               | DNS resolution strategy             |

### Merge Behavior Summary

| Rule                                              | mihomo            | sing-box           |
| ------------------------------------------------- | ----------------- | ------------------ |
| Basic overwrites profile (scalar/mapping fields)    | Yes               | Yes                |
| Sequences concatenate (basic + profile)            | Yes (`rules`, etc.) | Yes (`outbounds`, `route.rules`, `route.rule_set`) |
| Profile fields NOT in basic are preserved as-is    | Yes               | Yes                |
| Basic fields NOT in profile are added              | Yes               | Yes                |
