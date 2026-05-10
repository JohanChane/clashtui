# sing-box Support Analysis

> **Last verified**: 2026-05-06
> **References**:
> - [sing-box docs](https://sing-box.sagernet.org/) — authoritative sing-box reference
> - [mihomo docs](https://wiki.metacubex.one/) — authoritative mihomo reference
> - [v2rayn core_api.md](https://github.com/2dust/v2rayN) — API comparison & compat layer
> - [v2rayn design_zh.md](https://github.com/2dust/v2rayN) — multi-core architecture

This document analyzes the differences between **sing-box** and **mihomo** at an architectural level, referencing v2rayn's multi-core pattern, and presenting the phased implementation plan for demotui.

For detailed side-by-side comparisons in each domain, see the companion documents:

- [mihomo_vs_singbox/api_data.md](mihomo_vs_singbox/api_data.md) — REST API JSON format differences (endpoints, connection metadata, traffic stats)
- [mihomo_vs_singbox/config.md](mihomo_vs_singbox/config.md) — configuration format differences (basic config, proxy-provider, proxy-group, rules)
- [mihomo_vs_singbox/cmd.md](mihomo_vs_singbox/cmd.md) — CLI/bin command differences (validation, service control, config reload, log level)
- [clashtui_filetree.md](clashtui_filetree.md) — file tree layout for `/opt/clashtui/` and `~/.config/clashtui/`

---

## 1. REST API: Compatibility Summary

sing-box implements a **partial mihomo REST API compatibility layer** under `experimental.clash_api`. It is NOT a complete clone but is sufficient for proxy management, connection viewing, and mode switching.

The endpoint-by-endpoint comparison and JSON format examples are in [api_data.md](mihomo_vs_singbox/api_data.md). Here is the condensed compatibility verdict:

| Category | Verdict | Action |
|---|---|---|
| Proxy management (`/proxies`) | Drop-in compatible | Reuse existing REST client |
| Connection display (`/connections`) | Compatible with minor field nulls | Make `ctype` field `Option<String>` or default to empty string |
| Connection close | Drop-in compatible | Reuse existing REST client |
| Config read (`GET /configs`) | Structurally different | Needs sing-box-specific config deser |
| Config update (`PATCH /configs`) | Limited to `mode` only | Accept reduced functionality for sing-box |
| Config reload (`PUT /configs`) | Not available | Use SIGHUP |
| Restart | Not available | Use systemd/process restart |
| Traffic stats | Different model (WS vs poll) | New WebSocket client + speed delta calculation |
| Logs, rules, DNS query | Not available | Unavailable in sing-box (inform user) |

### Traffic Statistics: The Critical Difference

**mihomo**: provides `downloadTotal` / `uploadTotal` in the `/connections` response. demotui's `ConnInfo` struct (`src/functions/restful.rs:200`) parses these cumulative byte counters to derive speed.

**sing-box**: provides traffic via a **WebSocket-only** `/traffic` endpoint:
```json
{"up": 123456789, "down": 987654321}
```
- Units: **bytes** (cumulative)
- Mode: **push** (persistent WebSocket connection)
- No proxy/direct split — only total up/down
- Speed is computed as `(new - prev) / time_interval`

**v2rayn's approach**: `StatisticsSingboxService` wraps the WebSocket into a polling-like interface, computes speed deltas, and feeds the same `ServerSpeedItem` model as mihomo. Both sources must normalize to the same internal representation.

---

## 2. Configuration Format: Summary

For the full structural comparison (basic config, proxy-provider, proxy-group, rules, DNS, TUN), see [config.md](mihomo_vs_singbox/config.md).

Key architectural points:

- **Format**: mihomo uses YAML with dedicated sections (`proxies:`, `proxy-groups:`, `rules:`); sing-box uses JSON with a unified `outbounds[]` array + `route.rules[]`
- **Proxy-provider**: NOT supported in sing-box. Profiles must contain explicit outbounds.
- **TLS/Transport**: mihomo uses flat booleans + inline type-specific options; sing-box uses nested objects (`tls.enabled`, `transport.type`)
- **Routing**: mihomo uses inline string matchers (`DOMAIN-SUFFIX,google.com,Proxy`); sing-box uses structured JSON objects with `rule_set` references to external `.srs` binary files

---

## 3. CLI/Bin Commands: Summary

For the full command comparison (validation, service control, config reload, log level, permissions), see [cmd.md](mihomo_vs_singbox/cmd.md).

Key architectural points:

- **Style**: mihomo uses flat flags directly on binary (`-t -d -f`); sing-box uses subcommands (`check`, `run`, `version`)
- **Config validation**: `mihomo -t -d <dir> -f <file>` vs `sing-box check -c <file>`
- **Config reload**: mihomo uses REST API `PUT /configs`; sing-box uses `SIGHUP` signal
- **Service control**: Same systemd/OpenRC interface; different service names and binary paths

---

## 4. v2rayn's Compatibility Approach

v2rayn (Windows/macOS/Linux) already supports both mihomo and sing-box with full feature parity. Its architecture offers practical patterns for demotui.

### 4.1 Unified Clash API Client

v2rayn treats sing-box and mihomo as **identical REST API clients** via `ClashApiManager`:

```
if (IsRunningCore(ECoreType.sing_box) || IsRunningCore(ECoreType.mihomo)) {
    // Same ClashApiManager for both
    // proxies, connections, config endpoints — all shared
}
```

**The exception — traffic stats**: v2rayn has two separate stat services:
- `StatisticsSingboxService` — wraps sing-box's WebSocket `/traffic`
- Mihomo stats — pulled from `GET /connections` `downloadTotal`/`uploadTotal`

Both normalize to the same `ServerSpeedItem` model.

### 4.2 Per-Core Config Generation (No Shared Abstraction)

v2rayn **intentionally keeps config generation separate** for each core:

```
CoreConfigHandler.GenerateClientConfig():
  if (RunCoreType == sing_box):
    → CoreConfigSingboxService    (generates SingboxConfig JSON)
  elif (RunCoreType == mihomo):
    → CoreConfigClashService      (generates YAML)
```

**Design rationale**: Xray and sing-box have fundamentally different config schemas — a shared interface would be a leaky abstraction. The number of cores is small — the cost of abstraction exceeds the benefit.

### 4.3 Shared Intermediate Model

Despite separate config generation, v2rayn has a **shared intermediate model** (`ProfileItem`, `CoreConfigContext`):
- `ProfileItem`: unified proxy node representation (address, port, protocol, transport, TLS params)
- `CoreConfigContext`: resolved proxy graph, DNS settings, routing rules, validated core type

Both cores receive the same data; each config generator converts it to its native format.

### 4.4 Protocol Support Matrix

| Protocol | sing-box | mihomo |
|---|---|---|
| VMess | ✓ | ✓ |
| VLESS | ✓ | ✓ |
| Shadowsocks | ✓ | ✓ |
| Trojan | ✓ | ✓ |
| Hysteria2 | ✓ | ✓ |
| WireGuard | ✓ | ✓ |
| SOCKS | ✓ | ✓ |
| HTTP | ✓ | ✓ |
| TUIC | ✓ | ✓ |
| AnyTLS | ✓ | — |
| Naive | ✓ | — |
| SSR | — | ✓ |
| Snell | — | ✓ |
| MASQUE | — | ✓ |

### 4.5 What v2rayn Decided: Summary

| Decision | Rationale | Applicable to demotui? |
|---|---|---|
| Unified REST API client for both cores | `clash_api` is a near-clone | **Yes** |
| Separate config generators per core | Config schemas fundamentally different | **Yes** |
| Shared intermediate model | Single source of truth for proxy data | **Yes** |
| Per-core traffic stat services with unified output | Different transport, same callback | **Yes** |
| No shared process management abstraction | Small number of cores, same OS patterns | **Yes** |

---

## 5. Phased Implementation Plan for demotui

> **Note**: The official task breakdown is in `openspec/changes/support-singbox/tasks.md`. This section is the original planning document.

### Phase 0: Prerequisites & Config Schema

**Goal**: demotui can discover and configure a sing-box installation alongside mihomo.

**Install and config directory layout** (created by the `install` script):

See [clashtui_filetree.md](clashtui_filetree.md) for the full file tree layout with descriptions of each file/directory. Summary:

- `/opt/clashtui/` — TUI binary, core binaries, runtime configs, systemd units
- `~/.config/clashtui/{mihomo,sing-box}/` — per-core user config, database, logs, templates

| Task | File(s) | Description |
|---|---|---|
| Add core type enum | `src/config/core.rs` | `CoreType { Mihomo, Singbox }`, defaulting to `Mihomo` |
| Add sing-box config paths | `src/config/core.rs` | `singbox_bin_path` (default `/opt/clashtui/sing-box/sing-box`), `singbox_config_dir` (default `/opt/clashtui/sing-box/config`), `singbox_config_path` |
| Add sing-box service name | `src/config/core.rs` | `singbox_service_name` (default `"clashtui_singbox"`) |
| Add sing-box controller info | `src/config.rs` | `singbox_external_controller`, `singbox_secret` |

### Phase 1: Launch & Basic API

**Goal**: Start sing-box process and use demotui's existing REST API client to display proxies, connections, and perform node switching.

| Task | File(s) | Description |
|---|---|---|
| Select controller by core type | `src/functions/restful/utils.rs` | Dispatch mihomo or sing-box controller URL |
| Add sing-box service control | `src/functions/command.rs` | `test_singbox_config()`, per-core svc dispatch |
| Update SrvCtl tab | `src/tui/tab/srvctl.rs` | Show active core type, sing-box ops |
| Make `ConnMetaData.ctype` optional | `src/functions/restful.rs` | Handle sing-box's missing `type` field |

### Phase 2: Config Generation & Traffic Stats

**Goal**: Generate sing-box-native JSON config and display real-time traffic stats.

| Task | File(s) | Description |
|---|---|---|
| Config generator module | `src/functions/config_gen_singbox/` | `mod.rs`, `outbound.rs`, `route.rs`, `dns.rs`, `inbound.rs` |
| WebSocket traffic client | `src/functions/restful/traffic_ws.rs` | Connect to `ws://{controller}/traffic`, compute speed delta |
| Normalize traffic to shared model | `src/functions/restful.rs` | `TrafficStats { speed_up, speed_down }` for both cores |
| Profile/template for sing-box | `src/tui/tab/files.rs` | Singbox profile type in profile management |

### Phase 3: Feature Parity

| Task | Description |
|---|---|
| Config hot-reload via SIGHUP | `kill -HUP $(pidof sing-box)` |
| TUN mode parity | Map demotui `tun` settings to sing-box `inbounds[type=tun]` |
| Error message mapping | Map sing-box-specific errors to user-facing messages |
| Signal handling | Ensure `SIGINT`/`SIGTERM` works for sing-box via service control |

---

## 6. Compatibility Layer Decision Summary

| Integration Point | Use Shared Layer? | Rationale |
|---|---|---|
| REST API — proxies | **Yes** | `clash_api` is a near-perfect clone; same routes, same JSON format |
| REST API — connections | **Yes** | Same endpoint; make optional fields nullable for sing-box |
| REST API — config read/write | **Partial** | `mode` is shared; all other config keys differ |
| REST API — restart | **No** | sing-box has no `/restart`; use external process restart |
| Traffic stats | **No (separate transport, same model)** | WebSocket vs poll; different transport, same output model |
| Config generation | **No** | JSON vs YAML, fundamentally different schemas |
| Config validation | **No** | Different CLI flags and subcommands |
| Service control | **Yes (dispatch by core type)** | Same systemd/openrc interface, different service names |
| Process management | **Yes (dispatch by core type)** | Same lifecycle, different binary paths and flags |
