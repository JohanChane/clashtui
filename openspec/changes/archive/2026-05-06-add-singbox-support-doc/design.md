## Context

demotui currently interfaces exclusively with mihomo proxy cores. It assumes a mihomo binary (`/usr/bin/mihomo`), a YAML config directory, and the mihomo Clash REST API at `external_controller`. sing-box is a separate proxy core with a JSON-native config format, different CLI flags, and a partial Clash REST API compatibility layer (enabled via `experimental.clash_api` in its config). v2rayn — the reference TUI/GUI project — has already solved multi-core support including sing-box, and its approach (documented in `docs/core_api.md` and `docs/design_zh.md`) provides a proven design pattern: parallel config generation services per-core, a unified REST API client (`ClashApiManager`), and per-core traffic stat services.

This change is documentation-only: research the gaps, document them, propose a phased implementation plan. No code is written.

### Current demotui Backend Integration Points

| Integration Point | File(s) | What It Uses |
|---|---|---|
| REST API client | `src/functions/restful.rs`, `restful/*.rs` | mihomo REST API at `external_controller` |
| Traffic stats | `restful.rs` → `ConnInfo.download_total/upload_total` via `GET /connections` | Poll-based, total-only |
| Proxy data | `restful/proxies.rs` → `ProxiesResponse` from `GET /proxies` | Full proxy tree with delays |
| Connection data | `restful.rs` → `ConnInfo` from `GET /connections` | Active connections with metadata |
| Config update | `restful.rs` → `PATCH /configs`, `PUT /configs` | Mode, log level, etc. |
| Service control | `functions/command.rs`, `command/linux.rs` | systemd/openrc/nssm for mihomo |
| Config validation | `command.rs:test_config()` | `{clash_bin_path} -t -d {dir} -f {path}` |
| Config paths | `config/core.rs:Basic` | `clash_bin_path`, `clash_config_dir`, `clash_config_path` |
| Profile/template | `tui/tab/files.rs`, `functions/file/` | Serialize to mihomo YAML format |

### Reference Sources

1. **v2rayn `docs/core_api.md`** — Detailed sing-box vs mihomo API comparison, data formats, traffic stat flow
2. **v2rayn `docs/design_zh.md`** — Multi-core architecture: unified `ProfileItem` model, per-core config generators, process management
3. **mihomo wiki** (wiki.metacubex.one) — Authoritative mihomo API and config reference
4. **sing-box docs** (sing-box.sagernet.org) — Authoritative sing-box API, config, and CLI reference

## Goals / Non-Goals

**Goals:**
- Produce a single reference document (`docs/singbox_support.md`) that comprehensively maps sing-box/mihomo differences across API, config, and CLI
- Document v2rayn's architecture decisions around these differences (what was unified, what was kept separate, and why)
- Provide a phased implementation plan (3-4 phases) for gradually adding sing-box support to demotui
- Make the document AI-friendly — structured, scannable, with comparison tables and code snippets for each difference area
- Use the official upstream docs and v2rayn as authoritative references

**Non-Goals:**
- Write any code
- Produce exhaustive protocol-by-protocol config mapping (only representative examples needed)
- Cover Xray differences beyond what's needed for contrast in the v2rayn comparison section
- Design a full compatibility layer (the plan identifies what needs building, not how to build every function)

## Decisions

### 1. Document structure: comparison-first, plan-last

The document opens with a structured comparison of the three difference domains (API, config, CLI), using comparison tables with **api endpoint**, **config key**, or **CLI flag** as primary key. Each domain section covers "what mihomo does", "what sing-box does", "difference summary", and "v2rayn's approach". The phased plan comes last, referencing the comparisons above.

**Rationale**: The comparison tables directly answer "can I reuse this?" for each integration point. The plan then maps these comparisons into concrete phases. v2rayn's `core_api.md` uses this same structure for APIs and it proved effective.

**Alternative considered**: Narrative diff format (e.g., "sing-box differs from mihomo in X ways..."). Rejected because comparison tables are more scannable for AI and developers looking up specific endpoints/keys.

### 2. Phased plan granularity: 4 phases

The implementation plan has four phases:
1. **Phase 0: Research & Infrastructure** — what must exist before sing-box can be touched (config schema for sing-box paths, binary discovery)
2. **Phase 1: Launch & Basic API** — start sing-box, use existing REST API tabs (proxies, connections) with minimal changes
3. **Phase 2: Config & Traffic** — sing-box-native config generation, traffic stats, service control differences
4. **Phase 3: Feature Parity** — any remaining gaps (TUN mode, profile management, speed test nuances)

**Rationale**: Follows v2rayn's implicit layering (process first, then API, then config generation). Phase 0 is demotui-specific prerequisite work. Each phase produces a testable increment.

**Alternative considered**: Single "big bang" implementation phase. Rejected because the scope is too large and the risks of API incompatibility require iterative verification.

### 3. v2rayn section: focus on design choices, not code

The document describes **what** v2rayn decided (wrap sing-box's traffic WebSocket into a unified stat service, use identical ClashApiManager for both cores, keep per-core config generators separate) rather than **how** (specific C# classes). This keeps the document language-agnostic and actionable for Rust.

**Rationale**: v2rayn's design patterns translate directly to Rust patterns (parallel mod files instead of partial classes, enum dispatch instead of `is RunningCore(sing_box)`).

### 4. No compatibility layer recommendation until comparison is complete

The document does not pre-decide whether demotui should use a compatibility layer (like v2rayn) or diverge. The comparison tables make the trade-off visible per integration point, and the phased plan includes compatibility layer decisions as explicit choices in each phase.

**Rationale**: v2rayn could use a unified `ClashApiManager` because sing-box's `clash_api` is a near-perfect clone. But for config generation, v2rayn kept separate services (`CoreConfigSingboxService` vs `CoreConfigClashService`). The document should present both approaches and let implementers decide per domain.

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| sing-box `clash_api` may not be a complete mirror of mihomo API — undocumented differences cause runtime failures in proxy selection, connection display, or speed tests | Comparison tables document every known endpoint difference with notes on untested areas; phased plan defers risky endpoints to later phases |
| Document becomes stale as sing-box/mihomo evolve | Include upstream doc URLs prominently for each section; note last-verified dates |
| Phase plan over-reaches or under-reaches (missing critical steps) | Reference v2rayn's implementation path as a proven baseline; each phase ends with a "verification checkpoint" listing what must work before proceeding |
