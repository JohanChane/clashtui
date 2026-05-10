## 1. Refactor into multi-file module

- [x] 1.1 Create `src/tui/tab/proxies/` directory: `proxies.rs` (hub + re-exports + `newtype_tab!` + `mod_agent!` + `Key` + tests), `proxies/tree.rs` (`ProxyTree`, `NodeItem`, `NodeType`), `proxies/content.rs` (`Proxies` struct, `dispatch_key`, trait impls, `spawn_select_inline`), `proxies/render.rs` (`render` function)
- [x] 1.2 Tests kept inline in `proxies.rs` as `#[cfg(test)] mod tests`
- [x] 1.3 `src/tui/tab/mod.rs`: no change needed — `mod proxies;` picks up `proxies.rs` which declares submodules via `mod tree; mod content; mod render;`
- [x] 1.4 `cargo check` passes after split

## 2. Fix zero-delay filtering (API layer)

- [x] 2.1 In `src/functions/restful/proxies.rs`: `test_group_delay()` — filter entries where `value == 0` from the returned `HashMap`
- [x] 2.2 In `src/functions/restful/proxies.rs`: `test_proxy_delay()` — map `Some(0)` to `None` via `filter(|&d| d > 0)` so callers see failed tests as absence of delay
- [x] 2.3 In `content.rs`: `TestDelay` and `TestAllDelay` handlers — add `if d > 0` / `if *d > 0` guards before pushing to history (double guard)

## 3. Fix UI display of failed delays

- [x] 3.1 In `tree.rs`: `push_entry` for File children — passes delay through (0 preserved); `sort_by_delay` maps 0 → `u64::MAX` for bottom-of-list ordering
- [x] 3.2 In `render.rs`: `delay == 0` → display `"FAIL"`; `delay > 0` → display `"{d}ms"`; `None` → empty
- [x] 3.3 In `tree.rs`: `sort_by_delay` — maps 0 to `u64::MAX` (already correct, verified)

## 4. Fix spinner hang on TestAllDelay

- [x] 4.1 In `content.rs`: `Key::TestAllDelay` completion wrapper — added `content.testing_since = None;`

## 5. Remove unnecessary sleep

- [x] 5.1 In `content.rs`: `Key::TestDelay` for `NodeType::Folder` — removed `tokio::time::sleep(2s)`

## 6. Add user-configurable test URL

- [x] 6.1 In `src/config/core.rs`: added `pub test_url: Option<String>` field to `ConfigFile` struct with `#[serde(default)]`
- [x] 6.2 In `content.rs`: read `CONFIG.cfg_file.test_url` as fallback when proxy has no per-proxy `test_url`
- [x] 6.3 Default config: serde auto-generates, users add `test_url:` key to their `config.yaml` manually. Default behavior uses `DEFAULT_TEST_URL` when not set.

## 7. Verify

- [x] 7.1 `cargo check` — passes (0 errors, only pre-existing warnings)
- [x] 7.2 `cargo test` — 66 passed, 1 ignored, 0 failed
- [x] 7.3 `cargo build` — clean debug build succeeds
