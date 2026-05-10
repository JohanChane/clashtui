## Context

demotui currently targets mihomo as its sole backend proxy core. The `docs/support_singbox/singbox_support.md` analysis (2026-05-06) confirmed that sing-box's `clash_api` is a near-complete clone of mihomo's REST API for proxy and connection endpoints, making sing-box support architecturally feasible. v2rayn's proven multi-core pattern demonstrates per-core config generation with a shared REST client — an approach directly applicable to demotui.

### Current Architecture
- `config::CONFIG` singleton holds a single `external_controller`, `basic_clash_config.yaml`, `clash_bin_path`, `clash_service_name` — all mihomo-specific
- `restful::utils::request()` hardcodes controller URL to `CONFIG.external_controller`
- `command.rs` hardcodes binary paths and CLI flags for mihomo
- Traffic stats use poll-based `/connections` with per-connection byte differencing
- Profile selection writes YAML, merges with `basic_clash_config.yaml`, calls `/configs` reload

### Constraints
- Backward compatibility: existing mihomo users must not break
- Config directory must remain the same (optional sing-box sub-directory)
- All 6 existing tabs must function for both core types
- sing-box does NOT support proxy-provider — profiles must contain explicit outbounds
- sing-box's `clash_api` is read-only for most config fields — only `mode` is patchable

## Goals / Non-Goals

**Goals:**
- Add `CoreType` enum (`Mihomo`, `Singbox`) to config with per-core paths and service names
- Route REST API calls to the correct controller URL based on active core type
- Implement sing-box service control (start/stop/restart) with correct binary and CLI flags
- Generate sing-box-native JSON config from profile data (separate config generator module)
- Display proxies, connections, settings, and service control for sing-box
- Implement WebSocket-based traffic stats collection for sing-box
- Support importing and updating sing-box profiles

**Non-Goals:**
- Cross-core profile sharing (mihomo profiles remain mihomo, sing-box profiles remain sing-box)
- In-process config hot-reload (sing-box uses SIGHUP — external mechanism)
- Feature parity for sing-box-missing API endpoints (logs, rules, DNS query — inform user)
- GUI-based config editing for sing-box JSON schema
- Concurrent core running (only one core active at a time)

## Decisions

### Decision 1: Per-Core Config Generation (No Shared Abstraction)

**Choice**: Create a completely separate `src/functions/config_gen_singbox/` module for sing-box JSON config generation, with no shared trait or interface with mihomo YAML profile handling.

**Rationale**: Following v2rayn's documented design choice — YAML vs JSON schemas are fundamentally different. A shared abstraction would be leaky. The number of cores is small (2), so the cost of duplication is lower than the cost of a bad abstraction.

**Alternatives considered**:
- Shared `CoreConfig` trait — rejected because mihomo config is implicit (profile YAML = near-final config) while sing-box requires full structural transformation
- Unified intermediate representation — rejected because the transformation from profile to sing-box JSON is a one-way mapping with no reuse

### Decision 2: Shared REST API Client with Core-Type Routing

**Choice**: Keep the existing `restful::utils::request()` function but make the controller URL core-type-aware. Both mihomo and sing-box use the same `request()` function. The active core type determines which `external_controller` to use.

**Rationale**: sing-box's `clash_api` is a near-perfect clone for `/proxies`, `/connections`, `/version`, and connection termination. The response JSON structures are identical. Only the traffic stats endpoint differs in transport (WebSocket vs poll).

**Alternatives considered**:
- Per-core REST client modules — rejected as unnecessary duplication for identical endpoints
- Trait-based REST client — rejected as over-engineering for a 2-core system

### Decision 3: WebSocket Traffic Stats Client (Separate Transport, Same Model)

**Choice**: Implement `src/functions/restful/traffic_ws.rs` as a sing-box-specific WebSocket client that connects to `ws://{controller}/traffic`, parses `{up, down}` JSON messages, computes speed deltas, and writes to a shared `TrafficStats` struct.

**Rationale**: The transport difference (WebSocket push vs REST poll) is fundamental. A shared abstraction at the transport level would be forced. However, both should normalize to the same `TrafficStats` output model so TUI display code is core-type-agnostic.

**Alternatives considered**:
- Poll WebSocket in the same 1-second `after_sync()` cycle — rejected because WebSocket is push-based; polling defeats the purpose
- Rewrite mihomo stats to also use WebSocket — rejected as unnecessary refactoring

### Decision 4: Per-Core Config Field in ConfigFile

**Choice**: Add a `core_type: CoreType` field to `ConfigFile` (YAML key `core-type`, defaults to `mihomo`). Add optional `singbox` subsection with per-core paths. The `Basic` struct gets sibling sing-box fields.

**Rationale**: Minimal config schema change. Users who don't use sing-box see no difference. The fields mirror existing mihomo fields with `singbox_` prefix, keeping the pattern consistent.

**Alternatives considered**:
- Separate `core` config subsection with nested `mihomo` / `singbox` — rejected as over-nested for 2 cores
- Separate config file for sing-box — rejected as adding complexity without benefit

### Decision 5: Core-Type Enum Dispatch in Tabs (Minimal)

**Choice**: Tabs remain core-type-agnostic at the content level. Core-type dispatch happens in the REST/command layer. The only tab-level change is displaying `core_type` in UI headers/footers.

**Rationale**: The TUI content structs (`Proxies`, `Connections`, etc.) call functions like `restful::proxies::fetch_proxies()`. These functions already return the same data structures regardless of core. Adding core-type awareness to every tab would be invasive.

**Alternatives considered**:
- Per-core tab implementations — rejected as massive duplication for identical UI
- Core-type parameter in every REST call — rejected as noisy; better to set once at the controller URL level

### Decision 6: Profile Type for sing-box

**Choice**: Add `ProfileType::Singbox` to the `ProfileType` enum and a `singbox` profile directory. sing-box profiles are JSON files (not YAML). `no_pp` is ignored for sing-box (sing-box has no proxy-provider concept).

**Rationale**: sing-box profiles must be structurally different from mihomo profiles (JSON format, no proxy-provider). Using a separate profile type makes the distinction explicit rather than implicit.

**Alternatives considered**:
- Reuse `ProfileType::File` with file extension detection — rejected as fragile and loses type safety

## Risks / Trade-offs

- **[Risk] sing-box API compatibility changes**: sing-box's `clash_api` is experimental and may change. → **Mitigation**: Wrap all sing-box-specific API expectations in the `restful` layer; if sing-box changes, only the REST client needs updating.
- **[Risk] WebSocket connection stability**: WebSocket may disconnect during sing-box runtime. → **Mitigation**: Implement reconnection with exponential backoff in the traffic client; log disconnects to error display.
- **[Risk] Profile JSON validation**: Generated sing-box JSON may fail `sing-box check`. → **Mitigation**: Run `sing-box check` before writing config, show validation errors to user in TUI.
- **[Trade-off] Limited Settings tab for sing-box**: Only `mode` can be changed via REST PATCH. Other settings require config file edit + SIGHUP. → Accept reduction and document in UI.
- **[Trade-off] No proxy-provider**: sing-box profiles must include all outbounds explicitly. Profile updates download the full JSON. → Accept as sing-box design limitation; inform user in profile management.

## Migration Plan

1. **Add config fields** (`CoreType`, `singbox_*` paths) with defaults — no migration needed for existing users
2. **Create install directories**: `/opt/clashtui/sing-box/` and `~/.config/clashtui/sing-box/` — manual setup, guided by documentation
3. **Profile migration**: Users manually re-import profiles as `ProfileType::Singbox` — no automatic mihomo→sing-box conversion
4. **Rollback**: Set `core-type: mihomo` in config.yaml to revert to mihomo-only mode

## Open Questions

- Should `core_type` switching be a runtime action (in SrvCtl tab) or require restart? → Leaning toward config-only (requires demotui restart) to avoid state inconsistency
- Should sing-box profiles be stored as `.json` in a separate directory, or use the same `profile_yamls/` dir? → Separate `profile_jsons/` directory to avoid format confusion
