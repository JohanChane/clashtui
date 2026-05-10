---
name: singbox-config-migration
description: Use when migrating sing-box JSON configuration files to be compatible with the current installed sing-box version. Triggered by errors from `sing-box check`, `sing-box run`, or systemd service failures related to deprecated/removed config fields.
license: MIT
metadata:
  author: demotui
  version: "1.0"
---

# Sing-Box Config Migration

## Overview

Migrate sing-box JSON configs to the current installed version by iteratively running `sing-box check` and fixing errors one by one until clean.

**Core principle:** Never guess — run `sing-box check` after every change, let the binary tell you what's wrong next.

## When to Use

- `sing-box check -c config.json` reports FATAL errors about deprecated/removed fields
- sing-box service fails with `start service: ...` errors referencing config issues
- User asks to "migrate", "fix", or "update" a sing-box config for a newer version

## Workflow

### Step 1: Detect Versions

```bash
sing-box version          # installed version (e.g., 1.13.11)
```

Estimate config's target version by inspecting for deprecated patterns (see [migration reference](./migration-reference.md)). Pattern → version mapping:

| Pattern in config | Config version ≤ |
|---|---|
| `dns.fakeip` at top level (not in server) | 1.11.x |
| `dns.servers[].address` as URL string | 1.11.x |
| `geoip` / `geosite` in any rule | 1.11.x |
| `sniff: true` on inbound | 1.10.x |
| `inet4_address` on tun inbound | 1.9.x |
| `rcode://` in DNS server address | 1.11.x |
| `independent_cache: true` in DNS | 1.13.x |
| `domain_strategy` on outbound | 1.11.x |
| `disable_cache` in DNS rules | 1.13.x |

### Step 2: Parse the Config

Read the JSON. Identify all version-sensitive fields. Build a list of likely issues before starting.

### Step 3: Iterate with `sing-box check`

```
LOOP:
  1. Run: sing-box check -c /path/to/config.json
  2. If exit 0 AND no ERROR/WARN → DONE
  3. Parse the error message → identify deprecated field
  4. Look up migration in [migration-reference.md](./migration-reference.md)
  5. If not found in reference, fetch: https://sing-box.sagernet.org/migration/
  6. Apply the ONE fix for that specific error
  7. REPEAT
```

**Critical: Fix ONE error at a time.** The next error often changes after each fix. Batch-fixing without checking creates cascading issues.

### Step 4: Verify No Warnings

After all FATAL errors are resolved, run once more and check for ERROR-level warnings. These become FATAL in the next sing-box version — fix them proactively.

## Key Migration Rules (embedded)

See [migration-reference.md](./migration-reference.md) for the full version-to-version reference.

Quick fixes for the most common errors:

| Error keyword | Fix |
|---|---|
| `legacy DNS fakeip options` | Move `dns.fakeip` into server: `{"type":"fakeip","tag":"dns_fakeip","inet4_range":"198.18.0.0/15"}` |
| `geosite database is deprecated` | Replace `"geosite":[...]"` with `"rule_set":[...]"`, add `route.rule_set` with remote SRS URLs |
| `geoip database is deprecated` | Replace `"geoip":"cn"` with `"rule_set":"geoip-cn"`, add remote rule_set |
| `legacy inbound fields` | Remove `sniff` from inbound, add route rule: `{"inbound":"tun-in","action":"sniff"}` |
| `unknown field "rule_set"` at top level | Move `rule_set` under `route.rule_set` |
| `unknown transport type` | Use `h3` not `http3`, `tls` not `tls://`, `udp` not bare IP |
| `unknown field "disable_cache"` | Remove `disable_cache` from DNS rules with `action:"predefined"` |
| `unknown field "independent_cache"` | Remove `independent_cache` from `dns` |
| `missing default_domain_resolver` | Add `"default_domain_resolver":"dns_resolver"` to `route` |
| `dependency[REJECT] not found` | Add `{"type":"block","tag":"REJECT"}` to outbounds |
| `detour to an empty direct outbound` | Remove `"detour":"DIRECT"` from DNS servers |
| `unknown rcode: SUCCESSFUL` | Remove `rcode` field from DNS rule with `action:"predefined"` |
| `unknown field "domain_resolver"` on DNS server | Field name is `domain_resolver` (replaces `address_resolver`) |

## Common Pitfalls

1. **`rule_set` key placement**: It goes under `route.rule_set`, NOT top-level `rule_set`
2. **Geosite SRS filenames**: `geosite-category-ads-all.srs`, `geosite-geolocation-!cn.srs` (the `!` is literal)
3. **DNS server type names**: `h3` (not `http3`), `tls` (not `tls://`), `udp` (not bare address)
4. **Predefined DNS rule**: Rcode `rcode://success` must become `"action":"predefined"` in the rule, and the server entry removed
5. **After adding rule_sets**: The first `sing-box run` will download SRS files and cache them — initial startup may be slow

## SRS Download URLs

```
https://github.com/SagerNet/sing-geoip/releases/latest/download/geoip-cn.srs
https://github.com/SagerNet/sing-geosite/releases/latest/download/geosite-category-ads-all.srs
https://github.com/SagerNet/sing-geosite/releases/latest/download/geosite-geolocation-!cn.srs
```
