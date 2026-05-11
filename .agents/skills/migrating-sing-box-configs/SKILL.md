---
name: migrating-sing-box-configs
description: Use when migrating sing-box configuration files between versions, when `sing-box check` reports deprecated fields or validation errors after an upgrade, or when a config written for an older sing-box version fails to work with a newer version.
---

# Migrating Sing-Box Configs

## Overview

Sing-box deprecates and removes configuration fields across versions. This skill covers the migration patterns for versions 1.8.0 through 1.14.0, with a mandatory `sing-box check` verification step after each migration.

## Core Workflow

1. Identify the current config version and the target version
2. Apply migrations sequentially from old to new (e.g. 1.8 → 1.10 → 1.11 → 1.12 → 1.14)
3. After each migration step, run `sing-box check -c <config>` to verify
4. If `sing-box check` fails, fix the reported errors before proceeding

## Mandatory Verification

After every migration change, run:

```bash
sing-box check -c /path/to/config.json
```

**Never skip this step.** A passing `sing-box check` is the only reliable indicator that the config is valid for the target version.

**If the binary for the target version is not installed**, install it first or use `-D` to specify a different working directory. Ask the user which version they are targeting and how to obtain the correct binary.

## When the Skill Doesn't Know a Configuration

If encountering a config field or migration not covered here, ask the user for:

1. The **sing-box docs repository** URL (e.g. a local clone of `https://github.com/SagerNet/sing-box` on the `docs` branch)
2. The **official website** URL (`https://sing-box.sagernet.org`) for the current documentation
3. The specific **source and target versions** involved

Use `WebFetch` against the official site (`https://sing-box.sagernet.org/migration/`) for the latest migration guide, and `https://sing-box.sagernet.org/configuration/` for current config reference.

## Migration Reference

### 1.14.0 → current

#### Inline ACME to certificate provider

`tls.acme` → `tls.certificate_provider` (inline) or `certificate_providers[]` top-level array with `tls.certificate_provider: "<tag>"`.

```json
// Before
{ "tls": { "enabled": true, "acme": { "domain": ["example.com"], "email": "a@b.com" } } }

// After (inline)
{ "tls": { "enabled": true, "certificate_provider": { "type": "acme", "domain": ["example.com"], "email": "a@b.com" } } }

// After (referenced)
{ "certificate_providers": [{ "type": "acme", "tag": "my-cert", "domain": ["example.com"], "email": "a@b.com" }],
  "tls": { "enabled": true, "certificate_provider": "my-cert" } }
```

#### Address filter fields to response matching

DNS rules using `rule_set` with `ip_cidr` items → wrap with `evaluate` action + `match_response: true`.

```json
// Before: DNS rule referencing geoip rule-set
{ "rule_set": "geoip-cn", "action": "route", "server": "local" }

// After: add evaluate action before the rule
{ "action": "evaluate", "server": "remote" },
{ "match_response": true, "rule_set": "geoip-cn", "action": "route", "server": "local" }
```

#### DNS: remove `independent_cache`

Simply delete the field — DNS cache now always keys by transport name.

#### DNS: `store_rdrc` → `store_dns`

In `experimental.cache_file`: rename `store_rdrc: true` → `store_dns: true`.

#### DNS: `ip_version` / `query_type` behavior

These fields now apply on every DNS rule evaluation (not just client queries). They are incompatible with legacy address filter fields in the same config. Migrate to `evaluate` + `match_response` first (see above).

### 1.12.0 → 1.13+

#### DNS server address → type

The `address` field with URI scheme prefixes is replaced by `type` + `server`:

| Old `address` | New `type` | New `server` |
|---|---|---|
| `"local"` | `"local"` | (none) |
| `"tcp://1.1.1.1"` | `"tcp"` | `"1.1.1.1"` |
| `"1.1.1.1"` | `"udp"` | `"1.1.1.1"` |
| `"tls://1.1.1.1"` | `"tls"` | `"1.1.1.1"` |
| `"https://dns.google/dns-query"` | `"https"` | `"dns.google"` |
| `"quic://1.1.1.1"` | `"quic"` | `"1.1.1.1"` |
| `"h3://1.1.1.1/dns-query"` | `"h3"` | `"1.1.1.1"` |
| `"dhcp://auto"` | `"dhcp"` | (none) |
| `"dhcp://en0"` | `"dhcp"` | `"interface": "en0"` |
| `"fakeip"` | `"fakeip"` | (none; inet4_range/inet6_range move into server) |
| `"rcode://refused"` | (removed) | Use `action: "predefined"` with `rcode` in a DNS rule |

Additional changes:
- `address_resolver` → `domain_resolver` on the server
- `strategy` on server → `strategy` on DNS rule, or `dns.strategy` as default
- `client_subnet` on server → `client_subnet` on DNS rule

#### Outbound DNS rules → domain_resolver

Remove `dns.rules` entries with `outbound` field. Add `domain_resolver` to the outbound's dial fields, or set `route.default_domain_resolver`.

```json
// Before
{ "dns": { "rules": [{ "outbound": "any", "server": "local" }] } }

// After — per-outbound
{ "outbounds": [{ "type": "socks", "server": "example.org", "server_port": 2080,
    "domain_resolver": { "server": "local", "rewrite_ttl": 60 } }] }

// After — global default
{ "route": { "default_domain_resolver": { "server": "local" } } }
```

#### `domain_strategy` → `domain_resolver`

In outbound dial fields: remove `domain_strategy`, add `domain_resolver` with `strategy`.

```json
// Before
{ "domain_strategy": "prefer_ipv4" }

// After
{ "domain_resolver": { "server": "local", "strategy": "prefer_ipv4" } }
```

### 1.11.0 → 1.12+

#### Legacy special outbounds → rule actions

| Old | New |
|---|---|
| `"type": "block"` outbound + route rule | Route rule with `"action": "reject"` |
| `"type": "dns"` outbound + route rule | Route rule with `"action": "hijack-dns"` |

Remove the `block`/`dns` outbound entirely, use rule actions instead.

#### Legacy inbound fields → rule actions

Move `sniff`, `sniff_timeout`, `domain_strategy` from inbound config into route rules keyed by `inbound` tag.

```json
// Before
{ "inbounds": [{ "type": "mixed", "sniff": true, "sniff_timeout": "1s", "domain_strategy": "prefer_ipv4" }] }

// After
{ "inbounds": [{ "type": "mixed", "tag": "in" }],
  "route": { "rules": [
    { "inbound": "in", "action": "resolve", "strategy": "prefer_ipv4" },
    { "inbound": "in", "action": "sniff", "timeout": "1s" }
  ] } }
```

#### Destination override → route options

Move `override_address` / `override_port` from `direct` outbound to route rules with `action: "route-options"`.

```json
// Before
{ "outbounds": [{ "type": "direct", "override_address": "1.1.1.1", "override_port": 443 }] }

// After
{ "route": { "rules": [{ "action": "route-options", "override_address": "1.1.1.1", "override_port": 443 }] } }
```

#### WireGuard outbound → endpoint

Move from `outbounds[{type:"wireguard"...}]` to `endpoints[{type:"wireguard"...}]`. Key field renames:

| Old (outbound) | New (endpoint) |
|---|---|
| `server` / `server_port` | `peers[].address` / `peers[].port` |
| `local_address` | `address` |
| `peer_public_key` | `peers[].public_key` |
| `pre_shared_key` | `peers[].pre_shared_key` |
| `reserved` | `peers[].reserved` |
| `system_interface` | `system` |
| `interface_name` | `name` |
| `mtu` | `mtu` |

### 1.10.0 → 1.11+

#### TUN address merge

Merge `inet4_address`/`inet6_address` → `address`, same for `route_address` and `route_exclude_address`.

```json
// Before
{ "inet4_address": "172.19.0.1/30", "inet6_address": "fdfe:dcba:9876::1/126",
  "inet4_route_address": ["0.0.0.0/1"], "inet6_route_address": ["::/1"] }

// After
{ "address": ["172.19.0.1/30", "fdfe:dcba:9876::1/126"],
  "route_address": ["0.0.0.0/1", "::/1"] }
```

### 1.9.0 → 1.10

#### `domain_suffix` behavior change

Values not prefixed with `.` now match `(domain|.+\.domain)` instead of literal prefix. Add `.` prefix to preserve old behavior, or verify intent.

#### `process_path` on Windows

Format changed from device path (`\Device\HarddiskVolume1\...`) to Win32 path (`C:\...`). Update all `process_path` values accordingly.

### 1.8.0 → 1.9+

#### Clash API cache → independent cache_file

Move `experimental.clash_api.cache_file`, `cache_id`, `store_fakeip` into `experimental.cache_file`. Add `"enabled": true`.

```json
// Before
{ "experimental": { "clash_api": { "cache_file": "cache.db", "store_fakeip": true } } }

// After
{ "experimental": { "cache_file": { "enabled": true, "path": "cache.db", "store_fakeip": true } } }
```

#### GeoIP → rule-sets

Replace `route.geoip`, `route.rules[].geoip`, `route.rules[].source_geoip` with `rule_set` entries and `rule_set` rule references.

- `geoip: "private"` → `"ip_is_private": true`
- `geoip: "cn"` → `"rule_set": "geoip-cn"` + remote rule-set definition
- `source_geoip` → `rule_set_ipcidr_match_source: true` on the rule
- Use `sing-box geoip export` to convert custom GeoIP databases

Requires `experimental.cache_file.enabled: true` for rule-set caching.

#### Geosite → rule-sets

Replace `route.geosite`, `route.rules[].geosite` with `rule_set` references.

```json
// Before
{ "route": { "rules": [{ "geosite": "cn", "outbound": "direct" }],
    "geosite": { "download_detour": "proxy" } } }

// After
{ "route": { "rules": [{ "rule_set": "geosite-cn", "outbound": "direct" }],
    "rule_set": [{ "tag": "geosite-cn", "type": "remote", "format": "binary",
      "url": "https://raw.githubusercontent.com/SagerNet/sing-geosite/rule-set/geosite-cn.srs",
      "download_detour": "proxy" }] },
  "experimental": { "cache_file": { "enabled": true } } }
```

## Common Mistakes

- **Skipping intermediate versions.** Migrate step by step. A 1.8 config cannot jump directly to 1.14.
- **Not running `sing-box check` after each step.** Each migration introduces changes that depend on prior steps being correct.
- **Forgetting `experimental.cache_file.enabled: true`** when migrating to rule-sets.
- **Rcode server removal.** `rcode://refused` DNS servers are gone; replace with `action: "predefined"` DNS rules with `rcode` field. Alternatively, use a DNS rule with `action: "reject"`.
- **WireGuard endpoint address.** The endpoint's `address` is the interface's own address, NOT the outbound's `local_address`. The peer address goes into `peers[].address`.
- **Using `ENABLE_DEPRECATED_LEGACY_DNS_SERVERS=true` env var.** This masks real migration errors. Always fix the config instead.
- **Not tagging DNS servers.** When migrating to `domain_resolver`, DNS servers referenced by tag (e.g. `domain_resolver: "tcp-dns"`) must have a `tag` field set.

## Red Flags

- Editing a config without knowing the source version — ask the user first
- Migrating without `sing-box check` available — install the target binary first
- Encountering unknown fields — ask for docs URLs, do not guess
- `sing-box check` passes only with `ENABLE_DEPRECATED_*` env var — the config is still broken
- `domain_resolver` references a DNS server without a `tag` — add `tag` to the server
- Making changes without running `sing-box check` — every change must be verified
