## 1. Extend REST API Data Structs

- [x] 1.1 Add `rule` (Option<String>) and `rule_payload` (Option<String>, serde rename="rulePayload") to `Conn` struct in `src/functions/restful.rs`
- [x] 1.2 Add `destination_ip` (Option<String>, serde rename="destinationIP") and `sniff_host` (Option<String>, serde rename="sniffHost") to `ConnMetaData` struct
- [x] 1.3 Add `terminate_all_connections()` convenience function: DELETE `/connections` → Result<()>
- [x] 1.4 Verify `cargo check` passes on the extended structs

## 2. Connections Tab Content

- [x] 2.1 Create `src/tui/tab/connections.rs` with `Connections` struct: `conns: Vec<Conn>`, `row: Option<usize>` (selected row index), `error: Option<String>`, `is_loading: bool`, `tick_count: u64`, `last_bytes: HashMap<String, (u64, u64)>` (for rate calculation), `display_rows: Vec<DisplayRow>` (with computed speeds), `sort_state: SortState` (Default/ByDownload/ByUpload)
- [x] 2.2 Define `DisplayRow` struct: `conn: Conn`, `dl_speed: u64`, `ul_speed: u64` (computed from diff)
- [x] 2.3 Define `SortState` enum: Default, ByDownload, ByUpload
- [x] 2.4 Define `Key` enum: MoveUp, MoveDown, GoTop, GoBottom, Terminate, TerminateAll, SortByDownload, SortByUpload, SortReset
- [x] 2.5 Implement `TryFrom<&KeyEvent> for Key` with default single-key bindings (j/Down → MoveDown, k/Up → MoveUp, G → GoBottom)
- [x] 2.6 Define column name constants: `HOST_COL = "Host"`, `RULE_COL = "Rule"`, `CHAINS_COL = "Chains"`, `DL_COL = "Download"`, `UL_COL = "Upload"`, `DLSPD_COL = "DL Speed"`, `ULSPD_COL = "UL Speed"`
- [x] 2.7 Implement `BasicTabContent for Connections` (Key = Key, State = (), TITLE = "Connections")
- [x] 2.8 Implement `init()`: spawn get_connections task, set loading state, populate conns + display_rows on callback
- [x] 2.9 Implement `after_sync()`: 1-second interval auto-refresh (tokio::time::sleep + get_connections, update conns + compute rates + apply sort)
- [x] 2.10 Implement `handle_key_event()`: dispatch MoveUp/Down/GoBottom to update `row` index
- [x] 2.10a Implement `dispatch_shortcut()` for chord keys: GoTop (gg), Terminate (dd), TerminateAll (ac), SortByDownload (sd), SortByUpload (su), SortReset (sr)
- [x] 2.10b Implement rate calculation: on each refresh, diff current `download`/`upload` against `last_bytes` to compute `dl_speed`/`ul_speed`, update `display_rows`
- [x] 2.10c Implement sorting: on SortByDownload, sort display_rows by dl_speed desc; on SortByUpload, sort by ul_speed desc; on SortReset, restore conns order (by API return order)
- [x] 2.10d Implement terminate flow: spawn async task (from dispatch_shortcut for dd), get confirmation via AskConfirm popup, call DELETE `/connections/:id`, trigger refresh on success
- [x] 2.10e Implement terminate_all flow: spawn async task (from dispatch_shortcut for ac), get confirmation via AskConfirm popup, call DELETE `/connections`, trigger refresh
- [x] 2.11 Implement `render()`: ratatui Table with column headers (Host, Rule, Chains, Download, Upload, DL Speed, UL Speed) and data rows from display_rows, highlight selected row, display error text above table, show connection count + sort state indicator
- [x] 2.11a Add sort marker to column header: append `▼` to the sorted column's header text when sort is active
- [x] 2.11b Implement human-readable bytes formatting helper (e.g., `1.2 KB`, `3.4 MB`) for Download/Upload/DL Speed/UL Speed columns
- [x] 2.12 Add `mod_agent!` macro invocation with all chord and single-key shortcuts

## 3. Tab Registration

- [x] 3.1 Wrap with `newtype_tab!(ConnectionsTab(Tab<Connections>))` in `src/tui/tab/connections.rs`
- [x] 3.2 Add `ConnectionsTab` to `enum_dispatch!` in `src/tui/tab/mod.rs`
- [x] 3.3 Export `ConnectionsTab` from `pub mod prelude` in `src/tui/tab/mod.rs`
- [x] 3.4 Add `ConnectionsTab::default()` to `tabs` vec in `App::new()` (`src/tui/app.rs`)
- [x] 3.5 Update `TAB_COUNT` from 3 to 4 and number key range from `'1'..='3'` to `'1'..='4'` in `handle_global_kv`
- [x] 3.6 Register `agent_init` for ConnectionsTab in the prelude module

## 4. Integration Verification

- [x] 4.1 `cargo check` passes with all new code
- [ ] 4.2 Verify ConnectionsTab renders in TUI with correct tab order (1=Status, 2=File, 3=Proxies, 4=Connections)
- [ ] 4.3 Verify connections table displays active connections with correct column data
- [ ] 4.4 Verify navigation keys (j/k) work correctly
- [ ] 4.5 Verify chords work: gg (go top), G (go bottom), dd (close), ac (close all), sd/su/sr (sort)
- [ ] 4.6 Verify close single connection works via `dd` chord with confirmation
- [ ] 4.7 Verify close all connections works via `ac` chord with confirmation
- [ ] 4.8 Verify sort by download speed (sd) reorders table
- [ ] 4.9 Verify sort by upload speed (su) reorders table
- [ ] 4.10 Verify sort reset (sr) restores original order
- [ ] 4.11 Verify sort markers (▼) appear on the correct column header
- [ ] 4.12 Verify auto-refresh updates the table every 1 second
- [ ] 4.13 Verify error states (Mihomo unreachable, authentication failure) display gracefully
