---
name: generating-sing-box-configs
description: Use when generating a new sing-box configuration file from scratch, when asked to create a sing-box client or server config, when setting up a TUN-based proxy client with DNS/route rules, or when generating a clashtui sing-box template with ${} placeholders.
---

# Generating Sing-Box Configs

## Overview

Generate complete, valid sing-box configs tailored to the user's needs. Always ask 4 mandatory questions before generating anything. Verify with `sing-box check`. Reference community examples and official docs when available.

## Mandatory Upfront Questions

Before generating ANY config, ask ALL 4 questions at once:

1. **Focus**: 通用方便 (general convenience), 性能 (performance), or 隐私安全 (privacy/security)?
2. **Platform**: PC/client side or server side? sing-box can build proxy servers too.
3. **Clash API**: Enable Clash API + web dashboard? (needed for clash meta GUI clients like clash-verge, metacubexd)
4. **Version**: What sing-box version? (affects available fields and syntax)

**Do NOT generate anything until all 4 are answered.** If the user is unsure, explain the tradeoffs.

## After Questions — Ask About Clashtui Template

Ask: "是否需要生成 clashtui 的 sing-box 模板配置?"

Clashtui templates use `${PPG.<group>}` and `${PGG.<name>}` placeholders for dynamic outbound expansion. If yes, follow the clashtui template section below.

If unsure about clashtui, see the clashtui project source for reference templates (e.g. `src/functions/file/template/testdata/singbox_common_tpl.json`).

## Config Patterns by Focus

### General Convenience (通用方便)

- TUN inbound + mixed inbound (port 7890) for system proxy fallback
- FakeIP DNS for zero-leak domain routing
- `auto_route: true, strict_route: true` for automatic routing
- `domain_resolver` on outbounds for transparent resolution
- Clash API enabled for GUI switching
- cache_file with `store_fakeip: true`

### Performance (性能)

- DNS: prefer UDP/TCP upstreams (lower latency than DoH/DoT)
- `domain_resolver` with `strategy` set per-outbound to avoid extra resolution hops
- TUN stack: `system` on Linux/Windows, `gvisor` on Android
- Multiplex enabled on proxy outbounds (h2mux or smux)
- `sniff` enabled with caching via `sniff_override_destination: false`
- Avoid unnecessary rule_set complexity — pin to specific rules
- Skip clash_api if not needed (saves memory)

### Privacy/Security (隐私安全)

- DNS: always use DoH (type:https) or DoT (type:tls) upstreams
- `independent_cache` if on older version (auto in 1.14+)
- Block QUIC/STUN/port 853 to prevent DNS leaks
- `action: reject` for bittorrent protocol
- No FakeIP if strict DNS audit is needed (use real DNS + geosite rules)
- Separate DNS server tags for proxy/direct resolution
- `sniff_override_destination: true` for strict domain-based routing

## Config Patterns by Platform

### PC Client (Windows/Linux/macOS)

```json
{
  "log": { "level": "info", "timestamp": true },
  "dns": {
    "servers": [
      { "tag": "dns-remote", "type": "tls", "server": "8.8.8.8", "detour": "proxy" },
      { "tag": "dns-direct", "type": "https", "server": "dns.alidns.com", "detour": "direct", "domain_resolver": "dns-local" },
      { "tag": "dns-local", "type": "local" },
      { "tag": "dns-fake", "type": "fakeip", "inet4_range": "198.18.0.0/15" }
    ],
    "rules": [
      { "rule_set": "geosite-cn", "server": "dns-direct" },
      { "query_type": ["A", "AAAA"], "server": "dns-fake" },
      { "server": "dns-direct" }
    ],
    "strategy": "prefer_ipv4"
  },
  "inbounds": [
    { "type": "mixed", "tag": "mixed-in", "listen": "127.0.0.1", "listen_port": 7890 },
    { "type": "tun", "tag": "tun-in", "address": ["172.19.0.1/30"],
      "mtu": 9000, "auto_route": true, "strict_route": true,
      "auto_redirect": true, "stack": "system" }
  ],
  "outbounds": [
    { "type": "selector", "tag": "proxy", "outbounds": ["auto", "node1"],
      "default": "auto" },
    { "type": "urltest", "tag": "auto", "outbounds": ["node1"],
      "url": "https://www.gstatic.com/generate_204", "interval": "5m", "tolerance": 50 },
    { "type": "direct", "tag": "direct" }
  ],
  "route": {
    "rule_set": [
      { "type": "remote", "tag": "geoip-cn", "format": "binary",
        "url": "https://github.com/SagerNet/sing-geoip/raw/rule-set/geoip-cn.srs",
        "download_detour": "direct", "update_interval": "7d" },
      { "type": "remote", "tag": "geosite-cn", "format": "binary",
        "url": "https://github.com/SagerNet/sing-geosite/raw/rule-set/geosite-geolocation-cn.srs",
        "download_detour": "direct", "update_interval": "7d" }
    ],
    "rules": [
      { "action": "sniff" },
      { "protocol": "dns", "action": "hijack-dns" },
      { "rule_set": "geosite-cn", "outbound": "direct" },
      { "rule_set": "geoip-cn", "outbound": "direct" },
      { "ip_is_private": true, "outbound": "direct" },
      { "protocol": "bittorrent", "outbound": "direct" }
    ],
    "default_domain_resolver": "dns-direct",
    "auto_detect_interface": true,
    "final": "proxy"
  },
  "experimental": {
    "clash_api": { "external_controller": "127.0.0.1:9090", "external_ui": "dashboard", "secret": "", "default_mode": "Rule" },
    "cache_file": { "enabled": true, "path": "cache.db", "store_fakeip": true }
  }
}
```

### Server Side

Server configs are minimalist — just one protocol inbound + `direct` outbound:

```json
{
  "log": { "level": "info" },
  "inbounds": [{
    "type": "hysteria2", "tag": "hysteria-in",
    "listen": "::", "listen_port": 443,
    "up_mbps": 100, "down_mbps": 100,
    "tls": { "enabled": true, "server_name": "example.com",
      "acme": { "domain": ["example.com"], "email": "admin@example.com" } },
    "users": [{ "password": "your-password" }]
  }],
  "outbounds": [{ "type": "direct", "tag": "direct" }]
}
```

**Note:** For 1.14+, `tls.acme` → `tls.certificate_provider` (see migrating-sing-box-configs skill).

Common server protocols from community examples at `https://github.com/SagerNet/sing-box-examples`:
Shadowsocks, Trojan, VMess, VLESS (Vision+REALITY, gRPC, HTTP2), Hysteria2, TUIC, ShadowTLS, Naive.

## DNS Strategy Quick Reference

| Strategy | Use case | Config pattern |
|---|---|---|
| **Real DNS only** | Privacy focus, no FakeIP complexity | Remote + direct DNS servers, geosite rules for split |
| **FakeIP** | PC client, zero DNS leak | Add fakeip server, rule: A/AAAA→fakeip |
| **Redirect (legacy)** | Old Android clients | `address_resolver` on remote servers |
| **Local DNS only** | Server side | `type: local`, or none if not needed |

DNS server types (1.12+): `local`, `udp`, `tcp`, `tls`, `https`, `quic`, `h3`, `dhcp`, `fakeip`, `hosts`.

## TUN Stack Choice

| Stack | Platform | Notes |
|---|---|---|
| `system` | Linux, Windows | Best performance, native TUN |
| `gvisor` | Android, cross-platform | Pure Go userspace stack, more compatible |
| `mixed` | Windows | system TCP + gVisor UDP, good Windows default |

TUN addresses: `["172.19.0.1/30"]` (IPv4 only), `["172.19.0.1/30", "fdfe:dcba:9876::1/126"]` (dual-stack).

## Version-Specific Adjustments

After generating, ensure syntax matches the target version. Key differences:

- **1.14+**: `certificate_providers[]`, `store_dns` not `store_rdrc`, remove `independent_cache`, DNS rules use `evaluate`+`match_response` for GeoIP in DNS
- **1.12+**: DNS servers use `type`+`server`, `domain_resolver` not `domain_strategy`, `default_domain_resolver` in route
- **1.11+**: Route rule `action: "sniff"`/`"hijack-dns"`/`"reject"` instead of legacy inbound fields and special outbounds
- **1.10+**: TUN `address` merged array (not `inet4_address`/`inet6_address`)
- **1.8+**: `rule_set` not `geoip`/`geosite` in route rules

**REQUIRED SUB-SKILL:** Use `migrating-sing-box-configs` skill for detailed migration steps if the version doesn't match.

## Clashtui Template Generation

When the user wants a clashtui template, use `${...}` placeholders for dynamic outbound expansion:

| Placeholder | Meaning |
|---|---|
| `${PPG.<group>}` | All proxy-provider names in a group |
| `${PGG.<name>}` | All generated proxy-group tags matching `<name>` |
| `expand_group_with: ["${PPG.<group>}"]` | Expand one outbound copy per provider in the group |

Template pattern — place these in `outbounds`:

```json
{ "type": "selector", "tag": "Entry", "outbounds": ["${PGG.Auto}", "${PPG.pvd}"], "default": "Auto-pvd" },
{ "type": "urltest", "tag": "Auto", "expand_group_with": ["${PPG.pvd}"],
  "url": "https://www.gstatic.com/generate_204", "interval": "5m", "tolerance": 50 }
```

Reference full template from the clashtui project source code (e.g. `src/functions/file/template/testdata/singbox_common_tpl.json`)

Templates go in `sing-box/templates/`, proxy-providers in `sing-box/template_proxy_providers`.

## Mandatory Verification

After writing the config, run:

```bash
sing-box check -c /path/to/config.json
```

If `sing-box` binary is not found, ask the user to provide the path or install it. **Never skip verification.**

If check fails:
1. Read the error message — sing-box reports the exact deprecated field
2. Fix the reported issue directly
3. Re-run check until it passes

## When You Don't Know Something

Ask the user for:
1. **sing-box docs repo** (local clone or `https://github.com/SagerNet/sing-box`)
2. **Official website** `https://sing-box.sagernet.org` — use `WebFetch` for current docs
3. **Community examples** at `https://github.com/SagerNet/sing-box-examples` for reference configs

Never guess config field names. Look them up in:
- Official site: `https://sing-box.sagernet.org/configuration/`
- Community examples: `https://github.com/SagerNet/sing-box-examples`

## Common Mistakes

- **Generating without asking questions first.** The user's needs determine the entire config structure.
- **Using deprecated fields.** Check the version and apply migrations. `domain_strategy`, `sniff` on inbound, `address` with URI prefixes in DNS, `geoip`/`geosite` in route — all deprecated in recent versions.
- **Forgetting `default_domain_resolver`** in 1.12+ configs with TUN.
- **Wrong TUN stack.** `gvisor` for Android, `system` for Linux/Windows.
- **Missing `domain_resolver` on remote DNS servers.** DoH servers need `domain_resolver` pointing to a bootstrap DNS server for initial resolution.
- **Forgetting `experimental.cache_file.enabled: true`** when using `rule_set` or `clash_api`.

## Red Flags

- Generating a config before asking the 4 mandatory questions — STOP
- Not running `sing-box check` after writing — the config is unverified
- Guessing field names instead of looking up docs — ask for docs URLs
- Ignoring version differences — always adapt syntax to the target version
