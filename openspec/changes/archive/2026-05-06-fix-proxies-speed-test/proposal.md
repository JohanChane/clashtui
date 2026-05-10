## Why

The Proxies tab (tab 3) has multiple correctness and UX bugs in its delay testing: zero-delay values from failed tests are displayed as misleading `0ms`, the batch test (`a t`) spinner never stops, and the per-node `t` test wastes 2 seconds with an unnecessary sleep. These bugs erode trust in the displayed data and make debugging proxy connectivity frustrating.

## What Changes

- **Display "FAIL" instead of "0ms"** for proxies whose delay test returned 0 (timeout / unreachable). Zero-delay records are no longer pushed to history.
- **Fix `a t` spinner hang**: `testing_since` is now cleared on completion so the spinner animation stops.
- **Remove unnecessary 2‑second sleep** after group delay test — the API returns data synchronously.
- **Refactor `proxies.rs`** (857 lines) into a multi-file module (`src/tui/tab/proxies/`): extract `ProxyTree` into `tree.rs`, rendering into `render.rs`, key handling into `content.rs`, keeping tests in a separate file. This follows the existing multi-file module convention used elsewhere in the codebase.
- **Add user-configurable test URL** in the config file (`test_url` field under `proxies`), using `https://www.gstatic.com/generate_204` as the default (verified correct; matches metacubexd).

## Capabilities

### New Capabilities
- `proxy-delay-validation`: Zero-delay values (failed tests) are filtered out of history records and explicitly shown as "FAIL" in the UI. Users no longer see misleading `0ms` values.

### Modified Capabilities
- `proxy-speed-test`: Updated to cover zero-delay filtering, proper spinner state management for batch tests, and removal of unnecessary 2‑second sleep.

## Impact

- **Affected files**: `src/tui/tab/proxies.rs` → `src/tui/tab/proxies/{mod.rs, tree.rs, content.rs, render.rs}`, `src/functions/restful/proxies.rs`, `src/config/core.rs` (new `test_url` field), initialization in `src/tui/app.rs`
- **No API changes**: endpoints and request formats unchanged
- **Not breaking**: delay filtering improves data honesty; previous `0ms` display was misleading
- **Existing `proxy-speedtest` spec**: some requirements are now covered by `proxy-delay-validation` and the updated `proxy-speed-test`
