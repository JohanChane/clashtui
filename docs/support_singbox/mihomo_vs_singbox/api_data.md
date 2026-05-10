# mihomo vs sing-box: API Data JSON Format Comparison

## 1. `/configs` — Running Config

### `GET /configs`

```jsonc
// mihomo
{
  "mode": "rule",
  "tun": { "enable": true, "stack": "System" },
  "log-level": "info",
  "bind-address": "*:7890",
  "allow-lan": true,
  "ipv6": false,
  "global-client-fingerprint": "chrome",
  "tcp-concurrent": true,
  "global-ua": "...",
  "dns": "...",
  "geodata-mode": true,
  "unified-delay": true,
  "geo-auto-update": true,
  "geo-update-interval": 24,
  "find-process-mode": "always"
}

// sing-box
{
  "mode": "rule"
  // No other fields exposed via clash_api
}
```

sing-box only exposes `mode`. All other mihomo config fields (`tun`, `log-level`, `allow-lan`, `ipv6`, `dns`, etc.) are **not returned** — they live in sing-box's native JSON config and are not accessible via the `clash_api` REST layer.

### `PATCH /configs` (partial update)

| Field | mihomo | sing-box |
|---|---|---|
| `mode` | Patchable | Patchable |
| `log-level` | Patchable | **Not patchable** |
| `tun.enable` | Patchable | **Not patchable** |
| Others | Patchable | **Not patchable** |

Example: switching to Direct mode — identical for both:

```json
// Request body (both cores)
{"mode": "direct"}
```

---

## 2. `/proxies` — Proxy Tree

Both cores return **near-identical** JSON. The response is a flat map of proxy name → proxy object:

```jsonc
{
  "proxies": {
    "GLOBAL": {
      "type": "Selector",          // mihomo: "Selector" | sing-box: "Selector"
      "now": "Proxy",              // currently selected child
      "all": ["DIRECT", "Proxy", "REJECT"],
      "name": "GLOBAL",
      "udp": true,
      "history": [{"delay": 45}]
    },
    "Auto": {
      "type": "URLTest",           // mihomo: "URLTest" | sing-box: "URLTest"
      "now": "node-hk-01",
      "all": ["node-hk-01", "node-hk-02"],
      "name": "Auto",
      "udp": true,
      "history": [{"delay": 123}]
    },
    "node-hk-01": {
      "type": "Vless",
      "name": "node-hk-01",
      "udp": true,
      "history": [{"delay": 50}]
    },
    "DIRECT": {
      "type": "Direct",
      "name": "DIRECT",
      "udp": true
    }
  }
}
```

**Differences:**
- `Proxy.proxy_type` — same values across both: `Selector`, `URLTest`, `Fallback`, `LoadBalance`, `Direct`, `Reject`, `Vless`, `Vmess`, `Shadowsocks`, `Trojan`, etc.
- `Proxy.alive` — present in mihomo, may be missing or `true` in sing-box
- `Proxy.hidden` — present in mihomo, may be missing or `false` in sing-box
- `Proxy.extra` — extended delay info for each child proxy, present in mihomo, may be missing in sing-box

**Verdict**: Drop-in compatible. Existing `ProxiesResponse` / `Proxy` structs work for both.

---

## 3. `/connections` — Active Connections

This is where the format **differs meaningfully**.

### mihomo response

```
GET /connections
```

```jsonc
{
  "downloadTotal": 1234567890,     // cumulative bytes (u64)
  "uploadTotal": 9876543210,       // cumulative bytes (u64)
  "connections": [
    {
      "id": "abc-123",
      "metadata": {
        "network": "tcp",
        "type": "http",            // ← PRESENT (connection type)
        "host": "google.com",
        "process": "chrome",
        "processPath": "/usr/bin/chrome",
        "sourceIP": "192.168.1.5",
        "sourcePort": "54321",
        "remoteDestination": "google.com",
        "destinationPort": "443",
        "destinationIP": "142.250.80.46",
        "sniffHost": "google.com"
      },
      "upload": 123456,            // per-connection bytes
      "download": 654321,
      "start": "2026-05-06T12:00:00Z",
      "chains": ["Proxy", "Auto", "node-hk-01"],
      "rule": "DOMAIN-SUFFIX",
      "rulePayload": "google.com"
    }
  ]
}
```

### sing-box response

```
GET /connections
```

```jsonc
{
  "connections": [                 // ← NO downloadTotal / uploadTotal
    {
      "id": "abc-123",
      "metadata": {
        "network": "tcp",
                                   // ← NO "type" field
        "host": "google.com",
        "process": "chrome",
        "processPath": "/usr/bin/chrome",
        "sourceIP": "192.168.1.5",
        "sourcePort": "54321",
        "remoteDestination": "google.com",
        "destinationPort": "443",
        "destinationIP": "142.250.80.46",
        "sniffHost": "google.com"
      },
      "upload": 123456,
      "download": 654321,
      "start": "2026-05-06T12:00:00Z",
      "chains": ["Proxy", "node-hk-01"],
      "rule": "domain_suffix",
      "rulePayload": "google.com"
    }
  ]
}
```

### Field comparison table

| Field | mihomo | sing-box | Impact on demotui |
|---|---|---|---|
| `downloadTotal` / `uploadTotal` | **Present** | **Missing** | Traffic totals unavailable via this endpoint; use `/traffic` WebSocket instead |
| `connections[].metadata.type` | `"http"`, `"socks"`, etc. | **Missing** | Make `ctype` `Option<String>`, display "N/A" when absent |
| `connections[].metadata.host` | Present | May be **empty** | Fall back to `destinationIP:destinationPort` |
| `connections[].metadata.process` | Present | Present | Same |
| `connections[].metadata.processPath` | Present | May be **empty** | Same |
| `connections[].metadata.network` | Present | Present | Same |
| `connections[].metadata.sourceIP` / `.sourcePort` | Present | Present | Same |
| `connections[].metadata.destinationIP` / `.destinationPort` | Present | Present | Same |
| `connections[].metadata.sniffHost` | Present | Present | Same |
| `connections[].rule` | `"DOMAIN-SUFFIX"`, `"MATCH"`, etc. | `"domain_suffix"`, `"outbound"`, etc. | CSS-style vs underscore naming; both present |
| `connections[].rulePayload` | Present | Present | Same |
| `connections[].chains` | Present | Present | Same |
| `connections[].upload` / `.download` | Present (per-connection bytes) | Present | Same |

### Required code changes

In `src/functions/restful.rs`:

```rust
// Before (mihomo-only):
pub struct ConnMetaData {
    pub network: String,
    pub ctype: String,              // will fail deser on sing-box
    // ...
}

// After (both cores):
pub struct ConnMetaData {
    pub network: String,
    #[serde(rename = "type", default)]  // default to "" for sing-box
    pub ctype: String,                  // or change to Option<String>
    // ...
}
```

---

## 4. `/traffic` — Traffic Statistics

This is the **most significant difference** — completely different transport and message format.

### mihomo

Uses per-connection byte differencing from `GET /connections`. No dedicated traffic endpoint needed (though WebSocket `/traffic` also works on mihomo as an alternative).

`ConnInfo` struct:
```rust
pub struct ConnInfo {
    pub download_total: u64,  // ← cumulative bytes from all connections
    pub upload_total: u64,
    pub connections: Option<Vec<Conn>>,
}
```

### sing-box

**WebSocket-only** push model at `ws://{controller}/traffic`:

```
WebSocket → ws://127.0.0.1:9090/traffic
```

Each message is a JSON text frame:
```json
{"up": 123456789, "down": 987654321}
```

- `up` / `down`: cumulative byte counters (u64) — NOT speed
- No proxy/direct split — only global totals
- Client must compute speed delta: `(new_up - prev_up) / time_interval`

### Normalized internal model (both cores → same struct)

```rust
pub struct TrafficStats {
    pub total_up: u64,
    pub total_down: u64,
    pub speed_up: u64,    // computed delta
    pub speed_down: u64,  // computed delta
}
```

| | mihomo | sing-box |
|---|---|---|
| Source | `ConnInfo.download_total` / `.upload_total` | WebSocket `{"up", "down"}` |
| Transport | REST poll (HTTP GET every 1s) | WebSocket push (persistent connection) |
| Unit | Cumulative bytes | Cumulative bytes |
| Proxy/Direct split | Yes (per-connection metadata) | No (global totals only) |
| Speed calculation | `(this_total - prev_total) / 1s` | `(this_msg - prev_msg) / msg_interval` |

---

## 5. `/version` — Core Version

Identical endpoint, different values:

```json
// mihomo
{"meta": true, "version": "v1.1.1"}

// sing-box
{"version": "1.2.3"}
```

Both return a `version` field. mihomo additionally has `meta: true`. The existing `control::version()` function reads the response as a raw string and works for both.

---

## 6. `/proxies/{name}/delay` — Delay Test

Identical format for both cores:

```
GET /proxies/node-hk-01/delay?url=https://www.gstatic.com/generate_204&timeout=5000
```

```json
{"delay": 123}
```

`delay` is milliseconds. A value of `0` means the test failed (treated as unreachable). Both cores use the same query parameters (`url`, `timeout`). Drop-in compatible.

---

## 7. Summary: Compatibility Matrix

| Endpoint | Method | Compatible? | Action needed |
|---|---|---|---|
| `/proxies` | GET | Yes | None |
| `/proxies/{name}` | GET/PUT | Yes | None |
| `/proxies/{name}/delay` | GET | Yes | None |
| `/connections` | GET | **Partial** — `ctype` missing, no totals | Make `ctype` `Option`; use WS for traffic |
| `/connections/{id}` | DELETE | Yes | None |
| `/connections` | DELETE | Yes | None |
| `/configs` | GET | **Different** — only `mode` returned | Handle missing fields gracefully |
| `/configs` | PATCH | **Limited** — only `mode` patchable | Mark other settings as unavailable in TUI |
| `/configs` | PUT | **Missing** in sing-box | Use SIGHUP for config reload |
| `/restart` | POST | **Missing** in sing-box | Use systemctl stop/start |
| `/version` | GET | Yes | None |
| `/traffic` | GET/WS | **Different transport** | New WebSocket client needed |

The bottom line: `/proxies` and connection CRUD are drop-in compatible. The two breaking areas are **connection metadata** (missing `ctype` field) and **traffic stats** (WebSocket push vs REST poll).
