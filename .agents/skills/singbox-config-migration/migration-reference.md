# Sing-Box Migration Reference

Version-by-version migration rules extracted from https://sing-box.sagernet.org/migration/ and practical experience.
Sing-box version: 1.14.0 (current latest as of this writing).

## 1.8.0 Changes

### Cache file from Clash API to independent options
```json
// OLD
{"experimental":{"clash_api":{"cache_file":"cache.db","store_fakeip":true}}}
// NEW
{"experimental":{"cache_file":{"enabled":true,"path":"cache.db","store_fakeip":true}}}
```

### GeoIP → rule-sets (FATAL in 1.12.0)
```json
// OLD
{"route":{"rules":[{"geoip":"cn","outbound":"direct"}]}}
// NEW
{"route":{"rules":[{"rule_set":"geoip-cn","outbound":"direct"}],"rule_set":[{"tag":"geoip-cn","type":"remote","format":"binary","url":"https://github.com/SagerNet/sing-geoip/releases/latest/download/geoip-cn.srs","download_detour":"DIRECT"}]}}
```

### Geosite → rule-sets (FATAL in 1.12.0)
```json
// OLD (DNS rule)
{"dns":{"rules":[{"geosite":["category-ads-all"],"server":"dns_block"}]}}
// NEW
{"dns":{"rules":[{"rule_set":["geosite-category-ads-all"],"action":"predefined"}]}}
// Must also add to route.rule_set:
{"route":{"rule_set":[{"tag":"geosite-category-ads-all","type":"remote","format":"binary","url":"https://github.com/SagerNet/sing-geosite/releases/latest/download/geosite-category-ads-all.srs","download_detour":"DIRECT"}]}}
```

## 1.10.0 Changes

### TUN address field merge
```json
// OLD
{"inbounds":[{"type":"tun","inet4_address":"172.19.0.1/30"}]}
// NEW
{"inbounds":[{"type":"tun","address":["172.19.0.1/30"]}]}
```

## 1.11.0 Changes

### Legacy special outbounds → rule actions
Block outbound is no longer a special type. If selectors reference `REJECT`:
```json
// ADD this outbound explicitly:
{"type":"block","tag":"REJECT"}
```

### Legacy inbound fields → route rules (FATAL in 1.13.0)
```json
// OLD: sniff on tun inbound
{"inbounds":[{"type":"tun","sniff":true,"tag":"tun-in"}]}
// NEW: remove from inbound, add route rule
{"inbounds":[{"type":"tun","tag":"tun-in"}]}
// Add at TOP of route.rules:
{"route":{"rules":[{"inbound":"tun-in","action":"sniff"},...]}}
```

## 1.12.0 Changes

### Legacy DNS server formats → new type+server format
```json
// OLD
{"dns":{"servers":[
  {"address":"tls://1.1.1.1","address_resolver":"dns_resolver"},
  {"address":"h3://dns.alidns.com/dns-query","address_resolver":"dns_resolver"},
  {"address":"223.5.5.5","detour":"DIRECT"},
  {"address":"fakeip","tag":"dns_fakeip"},
  {"address":"rcode://success","tag":"block"}
]}}
// NEW
{"dns":{"servers":[
  {"type":"tls","server":"1.1.1.1","domain_resolver":"dns_resolver"},
  {"type":"h3","server":"dns.alidns.com","domain_resolver":"dns_resolver"},
  {"type":"udp","server":"223.5.5.5"},
  {"type":"fakeip","tag":"dns_fakeip","inet4_range":"198.18.0.0/15"}
  // rcode:// server REMOVED — handled by DNS rule action instead
]}}
```

Key mappings:
- `address: "tls://X"` → `type: "tls", server: "X"`
- `address: "https://X/dns-query"` → `type: "https", server: "X"`
- `address: "h3://X/dns-query"` → `type: "h3", server: "X"`
- `address: "quic://X"` → `type: "quic", server: "X"`
- `address: "1.2.3.4"` (bare IP) → `type: "udp", server: "1.2.3.4"`
- `address: "tcp://X"` → `type: "tcp", server: "X"`
- `address: "local"` → `type: "local"`
- `address: "dhcp://auto"` → `type: "dhcp"`
- `address: "fakeip"` → `type: "fakeip"` (move `inet4_range` here from top-level `dns.fakeip`)
- `address: "rcode://success"` → REMOVE server, use DNS rule: `{"action":"predefined"}`
- `address: "rcode://refused"` → REMOVE server, use DNS rule: `{"action":"predefined","rcode":"REFUSED"}` (check valid rcodes)

**Also**: `address_resolver` → `domain_resolver` everywhere.

### Legacy DNS fakeip options (FATAL in 1.14.0)
```json
// OLD
{"dns":{"fakeip":{"enabled":true,"inet4_range":"198.18.0.0/15"}}}
// NEW: move into fakeip DNS server
// The top-level dns.fakeip block is REMOVED entirely.
// The inet4_range goes into the fakeip-type DNS server.
```

### Outbound DNS rule items → domain_resolver
```json
// OLD
{"dns":{"rules":[{"outbound":["any"],"server":"dns_resolver"}]}}
// NEW: remove the rule entirely, add to route:
{"route":{"default_domain_resolver":"dns_resolver"}}
```

### domain_strategy on outbound → domain_resolver
```json
// OLD (on outbound dial fields)
{"domain_strategy":"prefer_ipv4"}
// NEW: use domain_resolver on the outbound or route.default_domain_resolver
```

## 1.14.0 Changes

### independent_cache removal
```json
// OLD
{"dns":{"independent_cache":true}}
// NEW: remove the field entirely
```

### disable_cache on DNS rules with action:predefined
The `disable_cache` field is not valid on DNS rules that use `action: "predefined"`. Remove it.

### store_rdrc → store_dns
```json
// OLD
{"experimental":{"cache_file":{"enabled":true,"store_rdrc":true}}}
// NEW
{"experimental":{"cache_file":{"enabled":true,"store_dns":true}}}
```

### download_detour deprecated
On remote rule_sets, `download_detour` still works but is deprecated. Prefer `http_client` going forward, but `download_detour` is fine for 1.14.x.

## Error Message → Fix Quick Reference

| Error Message (partial) | Fix |
|---|---|
| `legacy DNS fakeip options is deprecated` | Move `dns.fakeip` into fakeip server, remove top-level block |
| `geosite database is deprecated` | Replace with `rule_set`, add `route.rule_set` entries |
| `geoip database is deprecated` | Replace with `rule_set`, add `route.rule_set` entries |
| `legacy inbound fields are deprecated` | Remove `sniff` from inbound, add sniff route rule |
| `unknown field "rule_set"` (top-level) | Move to `route.rule_set` |
| `unknown transport type: http3` | Use `h3` |
| `unknown field "disable_cache"` | Remove from DNS rules with `action: "predefined"` |
| `unknown field "independent_cache"` | Remove from `dns` |
| `missing default_domain_resolver` | Add `route.default_domain_resolver` |
| `dependency[REJECT] not found` | Add `{"type":"block","tag":"REJECT"}` to outbounds |
| `dependency[REJECT] not found` | Add `{"type":"block","tag":"REJECT"}` to outbounds |
| `detour to an empty direct outbound` | Remove `detour: "DIRECT"` from DNS servers |
| `unknown rcode: SUCCESSFUL` | Remove `rcode` from predefined DNS rule |
| `initialize router: parse rule-set[X]` (file not found) | Change rule_set type to `remote` with SRS URL, or download the SRS file |
| `unknown field "domain_resolver"` on DNS rule | `domain_resolver` goes on DNS servers (replaces `address_resolver`), not on DNS rules |
