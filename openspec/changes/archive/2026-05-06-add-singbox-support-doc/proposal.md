## Why

demotui currently only supports mihomo as its proxy backend. sing-box is a popular, actively developed alternative proxy core with a fundamentally different configuration format, CLI interface, and partially-compatible REST API (via its built-in `clash_api` experimental module). Without documentation of these differences and a phased plan to bridge them, adding sing-box support is infeasible. This change creates the research document that maps the gap between the two cores, drawing on v2rayn's existing compatibility approach and upstream documentation, and concludes with a concrete phased implementation plan.

## What Changes

- New documentation file `docs/singbox_support.md` covering:
  - API surface differences (traffic stats, proxy management, connection management, config endpoints) between sing-box's `clash_api` and native mihomo API
  - Configuration format differences (JSON vs YAML, structure, protocol representation, routing, DNS, TUN)
  - CLI/bin command differences (flags, subcommands, test/validate, service management)
  - How v2rayn handles these differences — its compatibility layer design decisions (what it unified, what it kept separate, and why)
  - A phased plan to add sing-box support to demotui, from minimal viable (launch + basic API) through full feature parity

## Capabilities

### New Capabilities

- `singbox-support-doc`: Research and analysis document mapping API, configuration, and CLI differences between sing-box and mihomo, with a phased implementation plan for gradual sing-box support in demotui.

### Modified Capabilities

None — this change adds documentation only; no existing code behavior changes.

## Impact

- New file: `docs/singbox_support.md`
- No code changes
- No API changes
- No dependency changes
- References: v2rayn `docs/core_api.md` and `docs/design_zh.md`, mihomo wiki (wiki.metacubex.one), sing-box docs (sing-box.sagernet.org)
