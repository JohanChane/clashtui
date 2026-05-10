## 1. App State and Key Routing

- [x] 1.1 Define `HelpPanel` struct in `src/tui/widget/help.rs` with `visible: bool` field, plus `is_active()`, `toggle()`, `dismiss()` methods, and `Default` impl
- [x] 1.2 Add `help: HelpPanel` field to `App` struct in `src/tui/app.rs`, initialized via `HelpPanel::default()` in `App::new()`
- [x] 1.3 Add `KeyCode::Char('?')` to `handle_global_kv` that calls `self.help.toggle()` before the catch-all `_` arm
- [x] 1.4 Add help intercept check in `handle_key_event` after PopUp check and before Chord check: `if self.help.is_active() { self.help.dismiss(); return; }`
- [x] 1.5 Add conditional render in `App::render`: after `render_which()` and before PopUp render, add `if self.help.is_active() { self.render_help(f, &self.tabs[self.tab_index as usize]); }`

## 2. Help Panel Rendering

- [x] 2.1 Create `src/tui/widget/help.rs` with `HelpPanel` struct (see task 1.1) and `pub fn render_help(f, tab)` that renders the help overlay
- [x] 2.2 In `render_help`, compute tab shortcuts from `tab.shortcuts()`, compute global shortcuts as a hardcoded list, calculate popup size (width 60, height dynamic from total entries + sections + borders)
- [x] 2.3 Render the overlay: `Clear` widget, bordered block titled " Help " centered on screen
- [x] 2.4 Render upper section ("Tab Shortcuts — <tab_name>") with tab shortcuts in 2 columns if >4 entries, using `key_event_to_str()` from `chord.rs` for key display
- [x] 2.5 Render lower section ("Global Shortcuts") with global keys in 2 columns
- [x] 2.6 Register `help` module in `src/tui/widget/mod.rs`

## 3. Validation

- [x] 3.1 Run `cargo check` and fix any compilation errors
- [x] 3.2 Run `cargo test` and ensure all existing tests pass (2 pre-existing failures unrelated to this change)
