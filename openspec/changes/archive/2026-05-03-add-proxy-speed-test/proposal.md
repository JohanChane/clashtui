## Why

Proxies tab currently shows no type or delay for Link entries, and has no latency testing shortcuts. Users need to see the type/delay of linked nodes at a glance and trigger speed tests from the keyboard without leaving the TUI. Mihomo's REST API already supports `/proxies/{name}/delay` and `/group/{name}/delay` — the UI just needs wiring.

## What Changes

- Link nodes in the proxy tree display the `type` and `delay` of the proxy they point to (currently both are blank for Links)
- `t` on a Folder (group): triggers delay test for all nodes inside that group, fetching results and updating the tree view
- `t` on a File (leaf node): tests only that single node
- `t` on a Link: tests the single node the Link points to
- `a t` multi-key chord: triggers delay test for ALL nodes across all groups, with visual progress indicator
- Delay column in the tree visually updates after each test completes (polling or batch refresh)

## Capabilities

### New Capabilities

- `proxy-speed-test`: Keyboard-driven latency/speed testing of proxy nodes, with per-node (`t`), per-group (`t` on folder), per-link (`t` on Link), and global (`a t`) scopes. Delay results display inline in the proxy tree alongside each node's type tag.

### Modified Capabilities

<!-- None existing -->

## Impact

- `src/tui/tab/proxies.rs` — Link `NodeItem` construction (add type + delay lookup), new `Key::TestDelay` + `Key::TestAllDelay` chords in `mod_agent!`, delay-test spawning logic in `handle_key_event`/`sync`, Link rendering updated to show type tag and delay
- `src/functions/restful/proxies.rs` — `test_proxy_delay()` and `test_group_delay()` already exist; may need a batch wrapper or result parser
- `docs/proxies_selection.md` — add shortcuts `t` and `a t` to documented bindings
