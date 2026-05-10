## Why

demotui currently only supports mihomo as the backend proxy core. Users who prefer sing-box cannot use demotui for proxy management, connection monitoring, service control, or status display. The `docs/support_singbox/singbox_support.md` analysis confirms sing-box's `clash_api` is a near-complete clone of mihomo's REST API — supporting sing-box is feasible and follows the proven v2rayn multi-core pattern.

## What Changes

- **Install paths**: Create `/opt/clashtui/{mihomo,sing-box}/` separation and `~/.config/clashtui/{mihomo,sing-box}/` config directories per core
- **Core type selection**: Add `CoreType` enum (`Mihomo`/`Singbox`) to config with per-core binary paths, config paths, and service names
- **Profile import for sing-box**: Support importing and updating sing-box profiles (note: sing-box does not support proxy-provider)
- **Proxies selection panel**: Extend the Proxies tab to work with sing-box's `/proxies` API (drop-in compatible)
- **Connection panel**: Extend the Connections tab handling sing-box's missing metadata fields (`type`, `nsMode`)
- **Settings panel**: Extend the Settings tab — sing-box has limited config PATCH support (mode only)
- **SrvCtl panel**: Extend the Service Control tab with sing-box aware service operations (different binary, flags, service name)
- **Status panel**: Extend the Status tab with sing-box WebSocket-based traffic statistics
- **Config generation**: Create `src/functions/config_gen_singbox/` for generating sing-box JSON config from profile data
- **Traffic stats**: Add WebSocket traffic client for sing-box (push model) alongside existing mihomo poll model

## Capabilities

### New Capabilities
- `singbox-install-paths`: Install directory (`/opt/clashtui/sing-box/`) and config directory (`~/.config/clashtui/sing-box/`) support with core type switching
- `singbox-profile-management`: Import, update, and manage sing-box profiles (no proxy-provider support; profiles contain explicit outbounds)
- `singbox-config-generation`: Generate native sing-box JSON config from demotui's proxy/profile data model (outbounds, route rules, DNS, inbounds)
- `singbox-traffic-stats`: WebSocket-based traffic statistics client for sing-box, normalizing to the same internal traffic model as mihomo

### Modified Capabilities
- `proxy-selection`: Proxy selector switching must work against sing-box's `/proxies` API in addition to mihomo
- `connection-management`: Connection display and termination must handle sing-box's reduced metadata fields (missing `type`, `nsMode`)
- `service-control`: Service start/stop/restart operations must dispatch by core type with correct binary path, CLI flags, and service name
- `mode-switching`: sing-box only supports `mode` PATCH via REST API — no other config fields are patchable
- `log-level-switching`: sing-box does not support log level changes via REST API; must use config file + SIGHUP reload
- `profile-serialization`: Profile data model must support a sing-box profile type distinct from mihomo profiles

## Impact

- **Config**: `src/config/core.rs` — new `CoreType` enum, per-core paths, service names
- **REST client**: `src/functions/restful/` — generalize controller URL selection by core type; add `traffic_ws.rs` for sing-box WebSocket
- **Service control**: `src/functions/command.rs` — add sing-box service dispatch, config validation via `sing-box check`
- **Config generation**: New `src/functions/config_gen_singbox/` module
- **TUI tabs**: `src/tui/tab/proxies/`, `src/tui/tab/connections/`, `src/tui/tab/settings/`, `src/tui/tab/srvctl/`, `src/tui/tab/status/` — core-type-aware dispatch
- **Profile management**: `src/tui/tab/files/` — profile/template tab extended for sing-box profiles
- **Documentation**: `docs/support_singbox/` — detailed comparison docs (`cmd.md`, `config.md`, `api_data.md`) and implementation guide (`singbox_support.md`)
