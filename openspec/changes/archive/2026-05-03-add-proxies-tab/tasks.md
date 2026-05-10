## 1. REST API Module

- [x] 1.1 Create `src/functions/restful/proxies.rs` with `ProxiesResponse`, `Proxy`, `DelayRecord`, `DelayInfo` structs (serde Deserialize, kebab-case)
- [x] 1.2 Implement `fetch_proxies()` — GET `/proxies` → `Result<ProxiesResponse>`
- [x] 1.3 Implement `get_proxy(name)` — GET `/proxies/<name>` → `Result<Proxy>`
- [x] 1.4 Implement `select_proxy(group, node)` — PUT `/proxies/<group>` with `{"name":node}` body → `Result<()>`
- [x] 1.5 Implement `test_proxy_delay(name, url, timeout)` — GET `/proxies/<name>/delay` → `Result<u64>`
- [x] 1.6 Implement `test_group_delay(name, url, timeout)` — GET `/group/<name>/delay` → `Result<()>`
- [x] 1.7 Register `pub mod proxies;` in `src/functions/restful.rs`

## 2. ProxyTree Data Structure

- [x] 2.1 Define `ProxyTree` struct with `roots: Vec<TreeEntry>`, `flat: Vec<String>`
- [x] 2.2 Define `TreeEntry` with `name`, `proxy_type`, `alive`, `delay`, `now`, `children`, `expanded`, `depth`
- [x] 2.3 Implement `ProxyTree::build(ProxiesResponse)` — DFS construct tree from flat map, skip hidden proxies
- [x] 2.4 Implement `ProxyTree::flatten_to_vec()` — produce `Vec<String>` (names) for ListState rendering
- [x] 2.5 Implement `ProxyTree::toggle_expand(&mut self, name)` — toggle expansion of node
- [x] 2.6 Implement `ProxyTree::update_delay(&mut self, name, delay)` — update cached delay for a node
- [x] 2.7 Implement `ProxyTree::refresh(&mut self, ProxiesResponse)` — rebuild tree preserving expansion state

## 3. ProxiesTab Content

- [x] 3.1 Create `src/tui/tab/proxies.rs` with `Proxies` struct: `tree: ProxyTree`, `error: Option<String>`, `is_loading: bool`
- [x] 3.2 Define `Key` enum: MoveUp, MoveDown, Expand, Collapse, CollapseParent, Select, SpeedTest(Group), Search, Leave
- [x] 3.3 Implement `TryFrom<&KeyEvent> for Key` with default hardcoded key bindings (j/k/arrows/Enter/t/T/u/BackTab/esc)
- [x] 3.4 Implement `BasicTabContent for Proxies` (Key = Key, State = ListState, TITLE = "Proxies")
- [x] 3.5 Implement `init()`: spawn fetch_proxies task, set loading state, build tree on callback
- [x] 3.6 Implement `after_sync()`: 5-second interval auto-refresh (tokio::time::sleep + fetch_proxies)
- [x] 3.7 Implement `handle_key_event()`: dispatch MoveUp/Down/Expand/Collapse/CollapseParent/SpeedTest/Search/Select
- [x] 3.7a Implement `CollapseParent`: when cursor is on a file (leaf node), find nearest ancestor folder and collapse it
- [x] 3.8 Implement `render()`: ratatui List with depth prefix (▶/▼), type indicator, delay display, highlight selected
- [x] 3.9 Add mod_agent! macro invocation for configurable keymaps

## 4. Tab Registration

- [x] 4.1 Wrap with `newtype_tab!(ProxiesTab(Tab<Proxies>));`
- [x] 4.2 Add `ProxiesTab` to `enum_dispatch!` in `src/tui/tab/mod.rs`
- [x] 4.3 Export `ProxiesTab` from `pub mod prelude`
- [x] 4.4 Add `ProxiesTab::default()` to `tabs` vec in `App::new()` (`src/tui/app.rs`)
- [x] 4.5 Update `TAB_COUNT` and number key range in `handle_global_kv`
- [x] 4.6 Register `agent_init` for ProxiesTab in the prelude module

## 5. Selection & Speed Test Operations

- [x] 5.1 Implement node selection flow: spawn async task, open Choice PopUp with `all` list, PUT to `/proxies/<name>`, trigger refresh
- [x] 5.2 Implement single speed test flow: spawn async task, GET delay, update cached delay via wrapper, mark is_testing
- [x] 5.3 Implement batch speed test flow: spawn async task, GET group delay, trigger full refresh
- [x] 5.4 Add loading animation (`-/|\`) rendering for nodes with pending tests (rotating char based on frame counter)
- [x] 5.5 Wire `tri!` error handling for all API calls (or_cancel for user cancel, or_set for network errors)

## 6. Integration Verification

- [x] 6.1 `cargo check` passes with all new code
- [ ] 6.2 Verify ProxiesTab renders in TUI with correct tab order (1=Status, 3=Proxies, 4=File) — needs live Mihomo
- [ ] 6.3 Verify tree expand/collapse works with keyboard — needs live Mihomo
- [ ] 6.4 Verify node selection switches the active proxy on Mihomo — needs live Mihomo
- [ ] 6.5 Verify speed test displays delay values — needs live Mihomo
- [ ] 6.6 Verify auto-refresh updates the tree every 5 seconds — needs live Mihomo
- [ ] 6.7 Verify error states (Mihomo unreachable, authentication failure) display gracefully
