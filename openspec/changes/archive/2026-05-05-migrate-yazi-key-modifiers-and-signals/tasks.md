## 1. Key Type

- [x] 1.1 Create `src/tui/key.rs` with `Key` struct (`code: KeyCode, shift: bool, ctrl: bool, alt: bool, super_: bool`), `From<KeyEvent>` with cross-platform shift normalization, `FromStr` for `<C-x>` / `<A-x>` / `<S-Up>` / `<D-h>` / `<C-S-p>` and bare chars, `Display` for `<D-C-A-S-Key>`, `plain()` helper, and `PartialEq/Eq/Hash/Debug/Clone/Copy` derives
- [x] 1.2 Add `key` module to `src/tui/mod.rs` re-exports and prelude

## 2. Migrate Core Types to Key

- [x] 2.1 Update `KeyCombo` in `src/tui/widget/tab.rs`: change inner type from `Vec<KeyEvent>` to `Vec<Key>`, update `Deref<Target = [Key]>`
- [x] 2.2 Update `TuiTab::shortcuts()` return type to `&[(Vec<Key>, &str)]`, update `TuiTab::dispatch_shortcut()` to receive `seq: &[Key]`, update `Tab<C>::dispatch_shortcut()` internals and `DualTab` impls
- [x] 2.3 Update `ChordHandler` in `src/tui/widget/chord.rs`: use `Key` instead of `KeyEvent` throughout (`check_init`, `continue_`, `handle` methods), rename `key_event_to_str` to use `Key::Display`
- [x] 2.4 Update `App::handle_key_event` and `App::handle_global_kv` in `src/tui/app.rs`: convert `KeyEvent` to `Key` once at entry point, pass `Key` through all routing, update `handle_global_kv` match arms from `KeyCode::Char(..)` to `Key { code: KeyCode::Char('q'), ctrl: false, .. }`
- [x] 3.1 Update `mod_agent!` in `src/tui/tab/mod.rs`: change `Agent` HashMap key type from `KeyEvent` to `Key`, update `quick_map` to produce `Key` instead of `KeyEvent`, add `key_from_str` helper that parses `<X-name>` modifier syntax or falls through to `KeyCode` conversion, update shortcut generation to emit `Vec<Key>` instead of `Vec<KeyEvent>`
- [x] 3.2 Update `TryFrom<&KeyEvent>` → `TryFrom<&Key>` in all `Key` enums within `mod_agent!`, update `agent` module `AGENT` type
- [x] 4.1 Update `src/tui/tab/connections.rs`: add `<C-c>` → `Close` binding, convert `TryFrom<&KeyEvent>` → `TryFrom<&Key>`
- [x] 4.2 Update `src/tui/tab/files/profile.rs`: add `<C-c>` → `Back` or `Close` binding, convert `TryFrom<&KeyEvent>` → `TryFrom<&Key>`
- [x] 4.3 Update `src/tui/tab/files/template.rs`: add `<C-c>` → `Close` binding, convert `TryFrom<&KeyEvent>` → `TryFrom<&Key>`
- [x] 4.4 Update `src/tui/tab/proxies.rs`: add `<C-c>` → `Close` binding, convert `TryFrom<&KeyEvent>` → `TryFrom<&Key>`
- [x] 4.5 Update `src/tui/tab/srvctl.rs`: add `<C-c>` → `Close` binding, convert `TryFrom<&KeyEvent>` → `TryFrom<&Key>`
- [x] 4.6 Update `src/tui/tab/status.rs`: ensure empty `Key` enum uses `TryFrom<&Key>` → `Err(())` (no change needed, verify)
- [x] 5.1 Add `signal-hook-tokio` dependency to `Cargo.toml` (with `futures-v0_3` feature)
- [x] 5.2 Create `src/tui/signals.rs`: `Signals` struct with `start()` → spawn tokio task using `signal_hook_tokio::Signals`, biased `select!` loop (rx control channel, sys signals, term EventStream), `handle_sys()` for SIGINT/SIGQUIT/SIGHUP/SIGTERM/SIGTSTP/SIGCONT dispatch, `handle_term()` converting `crossterm::Event::Key` to `Event::Key(Key::from(key))`, `stop()`/`resume()` control methods
- [x] 5.3 Update `tui::init()` in `src/tui.rs` to call `Signals::start()` after raw mode setup, store `Signals` in `App`
- [x] 5.4 Update `App::serve()` in `src/tui/app.rs`: remove direct `crossterm::EventStream` usage, instead receive events from `Signals` task via the existing `events` channel; manage `Signals::stop()`/`resume()` on suspend
- [x] 6.1 Update `src/tui/agent.rs`: change `agent::init()` to use `Key` as HashMap key type, update YAML load/serialize format
- [x] 6.2 Update `src/config/util.rs` keymap file format description if needed
- [x] 6.3 Run `cargo check` and fix all compilation errors
- [x] 6.4 Run `cargo test` and fix all test failures
- [x] 6.5 Verify Ctrl-C works as keyboard binding in TUI, verify SIGTERM gracefully quits, verify Ctrl-Z suspends and `fg` resumes
