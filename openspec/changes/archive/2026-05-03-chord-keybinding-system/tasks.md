## 1. Core types

- [x] 1.1 Define `Key` struct (code + shift/ctrl/alt/super flags) in `src/tui/key.rs`, with `From<KeyEvent>`, `Display`, `PartialEq`, `Eq`, `Hash`
- [x] 1.2 Define `Layer` enum in `src/tui/layer.rs` with variants: `App`, `Status`, `File`, `Which`
- [x] 1.3 Define `Cmd` enum (or trait) for command dispatch in `src/tui/cmd.rs` — initially empty, extended per tab
- [x] 1.4 Define `Chord` struct `{ on: Vec<Key>, run: Vec<Cmd> }` in `src/tui/chord.rs`
- [x] 1.5 Register modules in `src/tui.rs`

## 2. Which panel

- [x] 2.1 Define `Which` struct in `src/tui/which.rs` with `times: usize`, `cands: Vec<Chord>`, `visible: bool`
- [x] 2.2 Implement `Which::show_with(key, layer)` — filter chords starting with key, sort, set visible
- [x] 2.3 Implement `Which::r#type(key) -> bool` — progressive filtering, auto-execute on single match or exact match, reset on empty
- [x] 2.4 Implement Which rendering — render candidates as lines with highlighted prefix, rest keys, separator, description
- [x] 2.5 Add `which` style section to `Theme` (cand, rest, separator, desc fields)

## 3. Router

- [x] 3.1 Define `Router` struct in `src/tui/router.rs` holding references to layer chord tables and Which state
- [x] 3.2 Implement `Router::route(key, layer) -> bool` — try Which first if visible, then match against layer chords
- [x] 3.3 Implement single-key match: if `on.len() == 1` and `on[0] == key`, execute commands, return true
- [x] 3.4 Implement multi-key prefix match: if `on.len() > 1` and `on[0] == key`, call `Which::show_with`, return true
- [x] 3.5 Implement non-match fallback: return false (key falls through to lower layers)

## 4. TuiWidget trait changes

- [x] 4.1 Remove `handle_key_event(&mut self, kv: &KeyEvent)` from `TuiWidget` trait in `src/tui.rs`
- [x] 4.2 Add `fn chords(&self) -> Vec<Chord>` to `TuiWidget` trait
- [x] 4.3 Add `fn execute(&mut self, cmd: &Cmd)` to `TuiWidget` trait
- [x] 4.4 Update `TuiTab` trait — add `fn layer() -> Layer` method
- [x] 4.5 Update `enum_dispatch!` macro in `src/tui/tab/mod.rs` to delegate `chords`, `execute`, `layer`
- [x] 4.6 Update `newtype_tab!` and `new_type_impl_tuiwidget!` macros accordingly

## 5. App routing integration

- [x] 5.1 Add `which: Which` field to `App` struct in `src/tui/app.rs`
- [x] 5.2 Add `Router` initialization in `App::new()` with chord references from all tabs
- [x] 5.3 Change `handle_key_event` dispatch order to: `KeyEvent → Key`, then Router → PopUp → App global keys
- [x] 5.4 Add Which rendering in `App::render()` — render Which panel when visible
- [x] 5.5 Add Which sync in `App::sync()` — handle Which state cleanup if needed

## 6. Migrate existing Tabs

- [x] 6.1 Migrate `StatusTab` — convert `handle_key_event` to chord registration + `execute` dispatch
- [x] 6.2 Migrate `FileTab` — convert `handle_key_event` to chord registration + `execute` dispatch
- [x] 6.3 Migrate `Profile` (inner content) — convert key handling to chords + commands
- [x] 6.4 Migrate `Template` (inner content) — convert key handling to chords + commands
- [x] 6.5 Migrate `DualTab` widget — forward `chords()`, `execute()` to active content
- [x] 6.6 Migrate `Tab<C>` widget — forward `chords()`, `execute()` to content

## 7. Cleanup and macro updates

- [x] 7.1 Remove `mod_agent!` macro and agent module (replaced by chord system)
- [x] 7.2 Remove `agent.rs` and keymap loading from `config` (no file-based keymap)
- [x] 7.3 Remove `keymap_path()` and related config functions
- [x] 7.4 Verify `cargo check` passes with no new warnings
- [x] 7.5 Verify `cargo test` passes all 3 existing tests
