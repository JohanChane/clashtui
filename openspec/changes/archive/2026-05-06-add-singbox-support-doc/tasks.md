## 1. Research & Source Material

- [x] 1.1 Read v2rayn `docs/core_api.md` and extract all sing-box/mihomo API comparison tables, data format differences, and traffic stat flow details
- [x] 1.2 Read v2rayn `docs/design_zh.md` and extract design rationale for per-core config generation, ClashApiManager unification, and process management
- [x] 1.3 Review sing-box `clash_api` documentation at sing-box.sagernet.org for current API compatibility status and any documented limitations
- [x] 1.4 Review mihomo REST API documentation at wiki.metacubex.one for current endpoint reference
- [x] 1.5 Review sing-box CLI reference (`sing-box help`, `sing-box check --help`, `sing-box run --help`) for flag comparison with mihomo
- [x] 1.6 Review sing-box JSON config schema and mihomo YAML config schema for structural comparison across outbounds, routing, DNS, TUN, and API binding
- [x] 1.7 Check v2rayn `ECoreType` enum, `CoreInfoManager`, and sing-box-specific config templates for additional insight on binary management and config scaffolding

## 2. API Differences Section

- [x] 2.1 Create comparison table for all REST API endpoints used by demotui: traffic stats, proxy management (`/proxies`, `/proxies/{name}`, `/proxies/{name}/delay`), connection management (`/connections`, `/connections/{id}`), config management (`/configs`), health/restart (`/restart`, `/version`)
- [x] 2.2 Document sing-box's WebSocket `/traffic` endpoint vs mihomo's poll-based totals in `/connections` — pull vs push model, format differences, lack of proxy/direct split in both
- [x] 2.3 Document any field name differences in Clash API JSON responses (e.g., `type` → `network.type` differences, `nsMode` absence in sing-box, `process` field availability)
- [x] 2.4 Summarize API compatibility verdict: which endpoints are drop-in replacements, which need adapters, which are unavailable

## 3. Configuration Format Differences Section

- [x] 3.1 Compare top-level config structure: mihomo YAML (`proxies:`, `proxy-groups:`, `rules:`, `dns:`, `tun:`) vs sing-box JSON (`outbounds:`, `route:`, `dns:`, `experimental:`)
- [x] 3.2 Compare outbound/proxy node representation with representative snippet for a common protocol (e.g., VLESS+reality+ws) in both formats
- [x] 3.3 Compare routing rule representation: mihomo inline `rules:` strings vs sing-box `route.rules[]` objects and `route.rule_set[]` references
- [x] 3.4 Compare DNS configuration: mihomo `dns.nameserver:` vs sing-box `dns.servers[]`
- [x] 3.5 Compare TUN configuration: mihomo `tun.enable/stack` vs sing-box `inbounds[type=tun]`
- [x] 3.6 Compare API/clash_api binding: mihomo `external-controller`/`secret` vs sing-box `experimental.clash_api.external_controller`/`external_secret`

## 4. CLI/Bin Command Differences Section

- [x] 4.1 Compare config validation: `mihomo -t -d <dir> -f <file>` vs `sing-box check -c <file>`
- [x] 4.2 Compare run commands: `mihomo -d <dir> -f <file>` vs `sing-box run -c <file> -D <dir>`
- [x] 4.3 Compare version display: `mihomo -v` vs `sing-box version`
- [x] 4.4 Document any sing-box subcommands or lifecycle signals relevant to demotui's service control (start, stop, restart, signal handling)
- [x] 4.5 Note any sing-box binary naming conventions and platform-specific paths (Linux, macOS, Windows)

## 5. v2rayn Compatibility Approach Section

- [x] 5.1 Describe v2rayn's unified `ClashApiManager` pattern — why it works for both cores and what the one exception is (traffic stats)
- [x] 5.2 Describe v2rayn's per-core config generation architecture — parallel service classes, no shared abstraction, intentional separation
- [x] 5.3 Describe v2rayn's traffic stat abstraction — `StatisticsSingboxService` (WebSocket) vs `StatisticsManager` (poll-based) and how both feed the same UI model
- [x] 5.4 Describe v2rayn's approach to protocol support matrices — which protocols each core supports, how it gates validation
- [x] 5.5 Summarize v2rayn's design decisions that are applicable to demotui and decisions that are v2rayn-specific (C# partial classes pattern vs Rust mod pattern)

## 6. Phased Implementation Plan Section

- [x] 6.1 Define Phase 0 (Prerequisites): config schema changes in `config/core.rs` (sing-box binary path, config path, service name), sing-box binary detection, config dir awareness, `ECoreType` or equivalent enum addition
- [x] 6.2 Define Phase 1 (Launch & Basic API): start sing-box process, connect existing REST API client to sing-box's `clash_api`, verify proxies tab works, verify connections tab works (with field-difference notes), define verification checkpoints
- [x] 6.3 Define Phase 2 (Config & Traffic): sing-box-native JSON config generation (vs mihomo YAML), traffic stats integration (WebSocket vs poll), service control differences (systemd for sing-box), config validation adaptation, define verification checkpoints
- [x] 6.4 Define Phase 3 (Feature Parity): TUN mode, profile management, speed test nuances, any remaining gaps identified in comparison tables, define verification checkpoints
- [x] 6.5 For each phase, include a decision box: whether to use a compatibility layer (like v2rayn's ClashApiManager) or diverge, with rationale

## 7. Review & Finalize

- [x] 7.1 Cross-reference each spec scenario against the document to confirm coverage
- [x] 7.2 Verify all URLs to external docs are correct and accessible
- [x] 7.3 Add last-verified dates to each section for staleness tracking
- [x] 7.4 Run `openspec status --change "add-singbox-support-doc"` to confirm all artifacts are complete
