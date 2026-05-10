## Context

demotui's Proxies tab renders a tree of proxy nodes (Folder / Link / File). The `/proxies` API already returns `proxy_type`, `delay` (via `history`), and `test_url` for each proxy. The `test_proxy_delay()` and `test_group_delay()` API wrappers already exist in `src/functions/restful/proxies.rs`. What's missing is (1) Link nodes surfacing the target's type/delay in the render, and (2) keyboard shortcuts to trigger delay tests from within the TUI.

The `mod_agent!` macro + `ChordHandler` already support adding new key bindings (`t` as a single press, `a t` as a multi-key chord) without architectural changes. The `FutureSet` async task system already supports spawning background I/O that updates component state via `wrapper()` callbacks.

## Goals / Non-Goals

**Goals:**
- Link nodes display `[vmess]`/`[ss]` type tag and `150ms` delay from the proxy they reference
- `t` on a Folder triggers `test_group_delay()` for that group, with results reflected after re-fetch
- `t` on a File triggers `test_proxy_delay()` for that single node, with result displayed immediately
- `t` on a Link triggers `test_proxy_delay()` for the single node the Link points to
- `a t` chord triggers delay test for ALL nodes, with visual progress indication
- Delay values are non-blocking — the UI remains responsive during tests

**Non-Goals:**
- Test URL configuration (uses existing proxy-level `test_url` from API, falls back to a hardcoded default)
- History graphs or per-node URL selection UI
- Cancelling an in-progress delay test mid-flight (beyond the CancellationToken already present)
- Persisting test URL preferences

## Decisions

### 1. Link type/delay: populate during tree rebuild

In `ProxyTree::rebuild_from_proxies()`, when creating a Link `NodeItem` for a child that is a group, look up the child's full `Proxy` from the `proxies` IndexMap to get `proxy_type` and `history.last().delay`.

**Rationale:** The `proxies` map is already in scope. File nodes already do this lookup (lines 232–238). Consistency with File nodes is the strongest argument. No new data structures needed.

**Alternative considered:** Separate API lookup per Link. Rejected — wasteful (the data is already in-memory from the same `/proxies` response).

### 2. `t` key: context-sensitive dispatch

`t` is added as a single `KeyCode::Char('t')` entry in `mod_agent!`. In `dispatch_key()`, the handler checks the currently selected node type:

- **Folder**: wrap `test_group_delay(name, url, timeout)` in a `FutureSet` task. On completion, re-fetch `/proxies` and rebuild tree.
- **File**: wrap `test_proxy_delay(name, url, timeout)` in a `FutureSet` task. On completion, store the delay result in a `HashMap<String, u64>` and inject it into the tree on the next render cycle, OR re-fetch `/proxies` to get the full picture (consistent with Folder).
- **Link**: same as File, using the Link target name.

Hybrid approach: per-node (File/Link) returns delay synchronously → update tree directly in the `wrapper` callback without re-fetching. Group test is fire-and-forget → re-fetch after a short delay.

**Rationale:** Per-node delay is synchronous (the API returns `{ "delay": 150 }` immediately), so we can display it instantly. Group delay is asynchronous (the server triggers tests, results appear later in `/proxies`), so re-fetch is required.

**Alternative considered:** Always re-fetch `/proxies` after any test. Rejected — wasteful for single-node tests that already have the result.

### 3. `a t` chord: batch test all nodes

Added as `([KeyCode::Char('a'), KeyCode::Char('t')], Key::TestAllDelay, "Test all delay")` in `mod_agent!`. The `a` prefix is already part of the existing chord system (`a s`, `a f`, `a e`).

Implementation: spawn one background task that iterates all Folders and Files at root level. For each Folder, call `test_group_delay`. For each standalone File, call `test_proxy_delay`. Track progress with an `AtomicUsize`. When all complete, re-fetch `/proxies` once and rebuild.

**Rationale:** Avoids flooding the event loop with N parallel `FutureSet` tasks. One batched task is simpler to track and notify. The `AtomicUsize` provides a coarse progress counter that `render()` can display as "Testing 3/12..." in the status area.

**Alternative considered:** N parallel tasks, each updating one node. Rejected — more complex bookkeeping, harder to know when "all done."

### 4. Test URL source

Each proxy may have a `test_url` field from the Mihomo API. For testing, use that if present. If absent, use a hardcoded default: `https://www.gstatic.com/generate_204`. This default is the standard Mihomo latency test URL and is widely used across yacd/metacubexd/mihomo-tui.

Timeout comes from `CONFIG.cfg_file.timeout` (which may be `None` → defaults to 5 seconds).

**Rationale:** No new config fields. Reuses existing infrastructure.

### 5. NodeItem delay field

`NodeItem` already has `delay: Option<u64>`. For per-node tests, the wrapper callback directly sets `node.delay`. For group/all tests, the re-fetch rebuilds the entire tree (which reads `history.last()` for each node from fresh API data).

**Rationale:** No struct changes needed. The `delay` field and `history` data are already wired to the render pipeline.

### 6. Visual feedback

- **Per-node test**: The `error` field on the Proxies component is set to `"Testing {node_name}..."` before the task spawns, cleared on completion. On error, set to error message.
- **Batch test**: Use `error` field set to `"Testing {done}/{total}..."`. Updated by the `wrapper` callback after each completed test within the batch.
- **No spinner/overlay**: Avoids PopUp API for non-blocking status. Simple text in the component's status area is sufficient.

## Risks / Trade-offs

- **Race condition**: If user presses `t` rapidly on multiple nodes, the re-fetch may overwrite inline delay updates. Mitigation: per-node tests write directly to `NodeItem.delay` (not via re-fetch), so they survive re-fetches. Group tests are inherently debounced by the re-fetch cycle.
- **Large groups**: `a t` on a config with 200+ proxies may take 5–10 seconds. Mitigation: the Mihomo API has a 5s default timeout per node, but `test_group_delay` is non-blocking server-side — the server handles parallelism. The UI remains responsive.
- **API errors**: If a node's test URL is misconfigured, `test_proxy_delay` returns an error. Mitigation: `tri!(, or_cancel)` silently swallows errors without breaking the UI. The error field can show the first failure.
