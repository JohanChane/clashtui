## Context

The Proxies tab is a single 857-line file (`src/tui/tab/proxies.rs`) that mixes tree data structures, rendering, key dispatch, async API calls, and tests. The codebase follows a multi-file module convention (e.g., `src/cli.rs` re-exports from `src/cli/`, `src/tui.rs` re-exports from `src/tui/`), but `proxies.rs` was never split. This makes it harder to navigate and extend.

Specific correctness issues:
1. **0ms display**: `DelayRecord::deserialize` falls back to `unwrap_or(0)` for missing `delay` fields. Mihomo returns `0` for failed/timeout tests. The code pushes these `0` values into history and displays `0ms` — misleading the user into thinking the proxy has sub-millisecond latency.
2. **Spinner hang on `a t`**: The `TestAllDelay` completion wrapper clears `content.error` but forgets to clear `content.testing_since`, so the spinner animation runs forever.
3. **Unnecessary 2‑second sleep**: `TestDelay` for Folder types has a `tokio::time::sleep(2s)` after `test_group_delay()` returns. The API responds synchronously; the sleep is wasteful.
4. **No user-configurable test URL**: `DEFAULT_TEST_URL` is a compile-time constant. metacubexd lets users set their own.

## Goals / Non-Goals

**Goals:**
- Filter zero-delay values at the API boundary so they never enter history or display as `0ms`
- Fix the `a t` spinner hang by clearing `testing_since` on completion
- Remove the unnecessary 2‑second sleep in per-group `t` test
- Split `proxies.rs` into a multi-file module following the project convention
- Add a `test_url` config field so users can override the default

**Non-Goals:**
- Changing the delay API endpoints or request format
- Adding real-time WebSocket streaming for delays
- Implementing delay trend charts (like metacubexd's sparkline)
- Adding node scoring or recommendations
- Merging/removing the duplicated `proxy-speedtest` spec (separate cleanup)

## Decisions

### D1: Filter zero delays at the API layer, not the UI layer

**Rationale**: `test_group_delay()` and `test_proxy_delay()` return raw Mihomo responses. Filtering zero values there means the `Proxies` content never sees them — no history pollution, no `0ms` display, no zero values in sort ordering.

**Alternatives considered**:
- *Filter at display time only*: Still pushes zeros to history; sort-by-delay would put failed nodes at the top (0 < any real delay), which is wrong.
- *Store zero as a separate enum variant* (e.g., `Delay::Timeout`): Over-engineering for this use case; filtering is simpler.

**Implementation**: `test_group_delay()` returns `HashMap<String, u64>` → filter entries where `value == 0`. `test_proxy_delay()` returns `Result<Option<u64>>` → convert `Some(0)` to `None`. The `DelayRecord` deserializer stays unchanged (it reflects Mihomo's stored history, not our filtering).

### D2: Show "FAIL" in the UI for nodes with no valid delay

**Rationale**: `0ms` is objectively wrong and misleading. "FAIL" clearly communicates the test didn't complete. Color it red to distinguish from valid delays.

**Implementation**: In `NodeItem`, change `delay: Option<u64>` to store the parsed value. In the render path, for `None` delay display `"FAIL"` in red; for `Some(n)` display `"{n}ms"` in the existing latency color scheme.

### D3: Module split: `proxies/{mod, tree, content, render}.rs`

**Rationale**: Follows the project convention (`src/cli/`, `src/tui/`, `src/config/`). Separates concerns clearly:
- `mod.rs` — re-exports, `newtype_tab!`, `mod_agent!`, `Key` enum, `tri!` macro
- `tree.rs` — `ProxyTree`, `NodeItem`, `NodeType`, build/rebuild logic
- `content.rs` — `Proxies` struct, `dispatch_key`, `init`, `handle_key_event`, `after_sync`, `spawn_select_inline`
- `render.rs` — `render` implementation

Tests stay in their own file (convention: `#[cfg(test)] mod tests` or `tests/`).

**Alternatives considered**:
- *Keep single file*: Works but violates convention and makes future extensions harder.
- *Split differently* (e.g., UI vs logic): Would scatter related code across files; current split groups by "shape" (tree vs content vs display).

### D4: Config field `proxies.test_url` in YAML config

**Rationale**: metacubexd stores this in browser `localStorage`. For a TUI, YAML config is the natural equivalent. Default to `https://www.gstatic.com/generate_204` (same default, verified correct).

**Implementation**: Add `test_url: Option<String>` to `ConfigFile` (analogous to the existing `timeout: Option<u64>`). Pass through to the API functions.

## Risks / Trade-offs

- **Legitimate 0ms latency is impossible** → filtering `0` is safe. No real proxy has sub-millisecond TCP/TLS latency.
- **Filtering at API layer means `test_group_delay` doesn't return failed nodes at all** → callers that want to know "which nodes were tested and failed" lose that information. Currently no caller needs this. If needed later, return a `Result` type instead of raw `u64`.
- **Module split changes import paths** → `src/tui/tab/mod.rs` needs a `pub mod proxies;` change. Other files importing from `proxies.rs` (if any) need updates. Checked: only `src/tui/tab/mod.rs` and `src/tui/app.rs` reference the Proxies tab — no direct imports of the inner types.

## Open Questions

- Should failed nodes still appear in the tree with a "FAIL" marker, or should they be hidden? → Decision: show "FAIL" marker, keep visible. Hiding would prevent users from trying to test them again.
