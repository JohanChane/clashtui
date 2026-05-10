## 1. Config Schema: Core Type & Per-Core Paths

- [ ] 1.1 Add `CoreType` enum (`Mihomo`, `Singbox`) to `src/config/core.rs` with serde support (`mihomo`/`singbox` YAML string mapping)
- [ ] 1.2 Add `core_type: CoreType` field to `ConfigFile` with default `Mihomo` for backward compatibility
- [ ] 1.3 Add `SingboxBasic` struct to `src/config/core.rs` with `singbox_bin_path`, `singbox_config_dir`, `singbox_config_path` fields
- [ ] 1.4 Add `singbox: SingboxBasic` field to `ConfigFile` with sensible defaults (`/usr/bin/sing-box`, `~/.config/clashtui/sing-box`)
- [ ] 1.5 Add `singbox_service_name` and `singbox_is_user` fields to `Service` struct in `src/config/core.rs`
- [ ] 1.6 Add `singbox_external_controller` and `singbox_secret` to `Config` in `src/config.rs`
- [ ] 1.7 Update `Config::load()` to read sing-box config fields and determine active controller URL by core type
- [ ] 1.8 Verify: existing mihomo users are unaffected — `cargo check` passes

## 2. REST API Client: Core-Type Dispatch

- [ ] 2.1 Make `request()` in `src/functions/restful/utils.rs` use the active controller URL (dispatch on `CONFIG.core_type`)
- [ ] 2.2 Add `get_controller_url()` helper in `utils.rs` that returns the correct URL based on core type
- [ ] 2.3 Make `ConnMetaData` fields `ctype` (connection type) and `nsMode` `Option<String>` for sing-box compatibility in `src/functions/restful.rs`
- [ ] 2.4 Update `ConnInfo` deserialization to handle sing-box's missing total fields gracefully (default to 0)
- [ ] 2.5 Update `ClashConfig` in `config_struct.rs` to handle sing-box's missing fields (`tun`, `geodata_mode`, etc.) by making them `Option`
- [ ] 2.6 Update `ClashConfig::build()` to show "N/A" for sing-box-unavailable fields
- [ ] 2.7 Verify: `GET /proxies` and `GET /connections` work against sing-box — `cargo check` passes

## 3. Service Control: Per-Core Dispatch

- [ ] 3.1 Add `service_name()` / `bin_path()` / `config_path()` helpers dispatching on `COre_type` in `src/functions/command.rs`
- [ ] 3.2 Update `svc_operation()` to use the correct service name and binary path based on core type
- [ ] 3.3 Add `test_singbox_config(path)` function running `{singbox_bin} check -c {path}` in `src/functions/command.rs`
- [ ] 3.4 Update `test_config()` to dispatch to mihomo or sing-box validation based on core type
- [ ] 3.5 Add `reload_singbox_config()` function sending SIGHUP to sing-box process in `src/functions/command.rs`
- [ ] 3.6 Verify: `sing-box check` is called for singbox core, `systemctl start sing-box` works — manual test with `cargo run`

## 4. Proxies Tab: Singbox API Compatibility

- [ ] 4.1 Verify proxy tree rendering works with sing-box `/proxies` response (drop-in compatible — verify with sample data)
- [ ] 4.2 Verify selector switching (`PUT /proxies/{name}`) works against sing-box controller
- [ ] 4.3 Verify delay testing (`GET /proxies/{name}/delay`) works against sing-box controller
- [ ] 4.4 Handle any sing-box-specific proxy type naming differences in `Proxy` struct deserialization
- [ ] 4.5 Verify: proxy tree displays, selection works, delay tests show results — manual test with sing-box running

## 5. Connections Tab: Singbox Metadata Handling

- [ ] 5.1 Update `Connections` content in `src/tui/tab/connections.rs` to handle `Option<String>` for `ctype`
- [ ] 5.2 Update `make_display_rows()` to show "N/A" or empty string when `ctype` is None
- [ ] 5.3 Handle sing-box's empty `processPath` and missing `process` fields in display row generation
- [ ] 5.4 Verify: connections display correctly with sing-box data — manual test with sing-box running

## 6. Settings Tab: Singbox-Limited Config

- [ ] 6.1 Add core-type awareness to Settings tab in `src/tui/tab/settings.rs` (read `CONFIG.core_type`)
- [ ] 6.2 Make "Switch Log Level" greyed out or show "Unavailable for sing-box" when `core_type` is Singbox
- [ ] 6.3 Mark non-patchable settings (tun, allow_lan, ipv6, etc.) as read-only display for singbox
- [ ] 6.4 Display "N/A" for singbox-unavailable config fields in `ClashConfig::build()`
- [ ] 6.5 Verify: Settings tab renders correctly for singbox, mode switching works, log level is marked unavailable

## 7. Service Control (SrvCtl) Tab: Singbox Operations

- [ ] 7.1 Add core-type display to SrvCtl tab header (`core: Mihomo` / `core: sing-box`)
- [ ] 7.2 Update `SrvCtlContent` to read and display per-core service name, binary path
- [ ] 7.3 Update `spawn_status_check()` to poll the correct service name based on core type
- [ ] 7.4 Verify: SrvCtl tab shows sing-box service name, start/stop/restart works

## 8. Status Tab: Core-Aware Display

- [ ] 8.1 Update Status tab to show core type in header or first config line
- [ ] 8.2 Display sing-box version from `GET /version` (same endpoint, compatible response)
- [ ] 8.3 Verify: Status tab shows "core: sing-box" with version and relevant config

## 9. Profile Management: Singbox Profile Type

- [ ] 9.1 Add `Singbox` variant to `ProfileType` enum in `src/config/database.rs`
- [ ] 9.2 Add serde serialization for `ProfileType::Singbox` as `!Singbox` YAML tag
- [ ] 9.3 Add `profile_jsons` directory constant to `src/config/util.rs` `defs` module
- [ ] 9.4 Implement `import_singbox_profile()` in `src/functions/file/profile.rs` — validates JSON, copies to `profile_jsons/`
- [ ] 9.5 Implement `update_singbox_profile()` — downloads JSON from URL, skips proxy-provider resolution
- [ ] 9.6 Update `select()` in `src/functions/file/profile.rs` to dispatch sing-box profile selection to config generation flow
- [ ] 9.7 Update Profile tab in `src/tui/tab/files/profile.rs` to show singbox profiles with `[singbox]` tag
- [ ] 9.8 Disable `no_pp` toggle for singbox profiles (show "N/A" or skip)
- [ ] 9.9 Update `ProfileData` to include singbox-specific fields if needed
- [ ] 9.10 Verify: import singbox JSON profile, select it, config is generated and validated

## 10. Config Generation: Singbox JSON Config

- [ ] 10.1 Create `src/functions/config_gen_singbox/` module directory
- [ ] 10.2 Implement `mod.rs` with `generate_singbox_config(profile_data) -> serde_json::Value`
- [ ] 10.3 Implement `outbound.rs` — converts profile proxy nodes to sing-box `outbounds[]` (VLESS, VMess, SS, Trojan, etc.)
- [ ] 10.4 Implement `route.rs` — generates `route.rules[]` and `route.rule_set[]` from routing configuration
- [ ] 10.5 Implement `dns.rs` — generates `dns.servers[]` with FakeIP and fallback servers
- [ ] 10.6 Implement `inbound.rs` — generates TUN, mixed HTTP/SOCKS, and `clash_api` experimental config
- [ ] 10.7 Add `write_singbox_config()` to serialize generated JSON to `singbox_config_path`
- [ ] 10.8 Add `validate_singbox_config()` to run `sing-box check` before deploying
- [ ] 10.9 Register module in `src/functions/mod.rs`
- [ ] 10.10 Wire config generation into profile selection flow for singbox profiles
- [ ] 10.11 Verify: generated JSON passes `sing-box check`, sing-box starts with generated config

## 11. Traffic Stats: WebSocket Client

- [ ] 11.1 Create `src/functions/restful/traffic_ws.rs` — WebSocket client for `ws://{controller}/traffic`
- [ ] 11.2 Implement `SingboxTrafficClient` with connect, parse, reconnect logic
- [ ] 11.3 Parse `{"up": u64, "down": u64}` JSON messages from WebSocket stream
- [ ] 11.4 Compute speed deltas between consecutive messages (diff up/down, divide by interval)
- [ ] 11.5 Define shared `TrafficStats` struct in `src/functions/restful.rs` (`total_up`, `total_down`, `speed_up`, `speed_down`)
- [ ] 11.6 Populate `TrafficStats` from WebSocket client and make available to TUI via shared state
- [ ] 11.7 Wire `TrafficStats` into Status tab display (or dedicated traffic display area)
- [ ] 11.8 Implement reconnection with exponential backoff (1s, 2s, 4s, max 30s)
- [ ] 11.9 Register module in `src/functions/restful.rs`
- [ ] 11.10 Add WebSocket dependency to `Cargo.toml` (e.g., `tokio-tungstenite`)
- [ ] 11.11 Verify: traffic speeds display correctly in TUI when sing-box is running

## 12. Documentation & Wiring

- [ ] 12.1 Update `docs/support_singbox/singbox_support.md` front matter — add implementation overview and setup guide at the top
- [ ] 12.2 Add `docs/get_started.md` sing-box section — how to install sing-box alongside mihomo
- [ ] 12.3 Update `ConfigFile` defaults in code to be documented
- [ ] 12.4 Update `src/config/util.rs` `load_home_dir()` to support sing-box subdirectory
- [ ] 12.5 Run `cargo test` — all 81 existing tests still pass
- [ ] 12.6 Run `cargo check` — no warnings

## 13. Final Verification

- [ ] 13.1 Manual test: start sing-box, all 6 tabs display data correctly
- [ ] 13.2 Manual test: switch between mihomo and singbox profiles (restart demotui)
- [ ] 13.3 Manual test: proxy selection, connection closing, mode switching works for singbox
- [ ] 13.4 Manual test: `cargo run --release` runs correctly
- [ ] 13.5 `cargo test` passes with all tests
