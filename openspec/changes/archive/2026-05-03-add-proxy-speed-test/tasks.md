## 1. Link Node Type/Delay

- [x] 1.1 In `ProxyTree::rebuild_from_proxies()`, look up the target proxy from `proxies` IndexMap when creating Link `NodeItem`, and set `proxy_type` and `delay` from the target's `proxy_type` and `history.last().delay`

## 2. Key Bindings

- [x] 2.1 Add `([KeyCode::Char('t')], Key::TestDelay, "Test delay")` to `mod_agent!` in `proxies.rs`
- [x] 2.2 Add `([KeyCode::Char('a'), KeyCode::Char('t')], Key::TestAllDelay, "Test all delay")` to `mod_agent!` in `proxies.rs`
- [x] 2.3 Add `TestDelay` and `TestAllDelay` variants to the `Key` enum

## 3. Per-Node Delay Test (t on File or Link)

- [x] 3.1 In `dispatch_key()` `Key::TestDelay` arm, resolve the node type and name from the selected index
- [x] 3.2 For File/Link: spawn a `FutureSet` task that calls `proxies::test_proxy_delay()`, re-fetches `/proxies` and rebuilds tree via `wrapper` callback
- [x] 3.3 Set `error` status to `"Testing {name}..."` before spawning, clear on completion or on error via `or_cancel`

## 4. Per-Group Delay Test (t on Folder)

- [x] 4.1 In `dispatch_key()` `Key::TestDelay` arm for Folder: spawn a `FutureSet` task that calls `proxies::test_group_delay()`, sleeps briefly (2s), then re-fetches `/proxies` and rebuilds the tree
- [x] 4.2 Set `error` status to `"Testing group {name}..."` before spawning, clear on completion

## 5. Global Delay Test (a t chord)

- [x] 5.1 In `dispatch_key()` `Key::TestAllDelay` arm: spawn a single `FutureSet` task that iterates all Folders and root-level Files in the tree
- [x] 5.2 For each Folder, call `test_group_delay()`; for each root-level File, call `test_proxy_delay()`
- [x] 5.3 Set `error` status to `"Testing all (N groups/nodes)..."` before batch starts; individual failures silently ignored via `.ok()`
- [x] 5.4 On batch completion, re-fetch `/proxies` and rebuild the tree, then clear the `error` status

## 6. Docs

- [x] 6.1 Update `docs/proxies_selection.md` to document `t` and `a t` shortcuts and their behavior
