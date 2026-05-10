# singbox-support-doc Specification

## Purpose
TBD - created by archiving change add-singbox-support-doc. Update Purpose after archive.
## Requirements
### Requirement: Document covers API differences
The `docs/singbox_support.md` document SHALL include a section comparing the REST API surfaces of mihomo and sing-box (with `clash_api` enabled), covering all endpoints used by demotui: traffic statistics, proxy management, connection management, config management, and health checks. Each endpoint SHALL have a comparison table listing the method, path, request/response format, and any known behavioral differences or missing features.

#### Scenario: Traffic stats endpoint comparison
- **WHEN** a reader looks up traffic statistics API differences
- **THEN** the document SHALL compare mihomo's `GET /connections` `downloadTotal`/`uploadTotal` fields with sing-box's WebSocket `ws://.../traffic` stream, noting the pull vs push model difference and the lack of proxy/direct split in both

#### Scenario: Proxy group endpoint comparison
- **WHEN** a reader looks up proxy group API differences
- **THEN** the document SHALL compare `GET /proxies`, `PUT /proxies/{name}`, and `GET /proxies/{name}/delay` across both cores, noting that sing-box's `clash_api` mirrors mihomo for these endpoints but may have minor field differences

#### Scenario: Connection endpoint comparison
- **WHEN** a reader looks up connection management API differences
- **THEN** the document SHALL compare `GET /connections` and `DELETE /connections/{id}` across both cores, noting any sing-box-specific metadata field differences (e.g., missing `type` field, missing `nsMode` field)

### Requirement: Document covers configuration format differences
The document SHALL include a section comparing mihomo YAML configuration with sing-box JSON configuration, covering the structural paradigm (rules vs rule_set, outbound representation, DNS configuration, TUN configuration, and API port binding). Each config domain SHALL have a comparison table with representative snippets from both formats.

#### Scenario: Outbound/proxy config comparison
- **WHEN** a reader looks up how proxy nodes are configured
- **THEN** the document SHALL show side-by-side: mihomo's YAML `proxies:` list with typed sections vs sing-box's JSON `outbounds[]` array with `type` field, highlighting structural differences in transport, TLS, and multiplexing

#### Scenario: API/clash_api binding comparison
- **WHEN** a reader looks up how to enable the REST API
- **THEN** the document SHALL compare mihomo's `external-controller` and `secret` top-level YAML keys with sing-box's `experimental.clash_api.external_controller` and `experimental.clash_api.external_secret` nested JSON keys

#### Scenario: Routing/rules comparison
- **WHEN** a reader looks up routing rule differences
- **THEN** the document SHALL compare mihomo's `rules:` list (inline matchers) with sing-box's `route.rules[]` and `route.rule_set[]` (with external rule-set references), noting that mihomo rules are inline while sing-box supports both inline and external rule sets

### Requirement: Document covers CLI/bin command differences
The document SHALL include a section comparing CLI commands and flags between the `mihomo` binary and the `sing-box` binary, covering configuration validation, service modes (run vs run -D), version display, and any subcommands relevant to demotui's service control workflow.

#### Scenario: Config validation command comparison
- **WHEN** a reader looks up how to validate configuration
- **THEN** the document SHALL compare mihomo's `mihomo -t -d <dir> -f <file>` with sing-box's `sing-box check -c <file>`, noting flag name and positional differences

#### Scenario: Run command comparison
- **WHEN** a reader looks up how to run the core
- **THEN** the document SHALL compare mihomo's `mihomo -d <dir> -f <file>` with sing-box's `sing-box run -c <file> -D <dir>`, noting the subcommand requirement for sing-box and the working directory flag name difference (`-d` vs `-D`)

### Requirement: Document references v2rayn compatibility approach
The document SHALL include a section describing how the v2rayn project handles sing-box/mihomo differences, covering: its unified `ClashApiManager` pattern, its per-core config generation services, its traffic stat abstraction, and its explicit decision NOT to create a shared config abstraction between cores.

#### Scenario: v2rayn API compatibility layer description
- **WHEN** a reader looks up how v2rayn bridges the API gap
- **THEN** the document SHALL explain that sing-box's `clash_api` is a near-complete clone of mihomo's REST API, so v2rayn uses a single `ClashApiManager` for both cores, gated by `IsRunningCore(ECoreType.sing_box)`, and describe the one exception (traffic stats: WebSocket for sing-box vs poll-based for mihomo)

#### Scenario: v2rayn config generation separation description
- **WHEN** a reader looks up how v2rayn generates configs
- **THEN** the document SHALL explain that v2rayn keeps per-core config generation completely separate (`CoreConfigSingboxService` vs `CoreConfigClashService`) because the config schemas are fundamentally different (JSON vs YAML), and that this is an intentional design choice documented in their design docs

### Requirement: Document includes a phased implementation plan
The document SHALL include a phased plan (minimum 3 phases) for gradually adding sing-box support to demotui, with each phase listing: specific changes required, files affected, and verification checkpoints that must pass before proceeding to the next phase.

#### Scenario: Phase structure
- **WHEN** a reader looks up the implementation plan
- **THEN** the plan SHALL be organized into numbered phases (0 through 3+), with Phase 0 covering prerequisite infrastructure and config schema changes, and subsequent phases adding runnable sing-box integration at increasing levels of feature parity

#### Scenario: Each phase has verification checkpoints
- **WHEN** a reader reviews any phase
- **THEN** that phase SHALL include a verification section listing specific behaviors that must work (e.g., "sing-box binary starts and serves REST API on expected port", "proxy groups display correctly in the Proxies tab")

#### Scenario: Plan covers all integration points
- **WHEN** a reader reviews the complete plan
- **THEN** all integration points identified in the API, config, and CLI comparison sections SHALL map to at least one phase in the plan

