## 1. Extend SrvCtlOp enum

- [x] 1.1 Add `SwitchMode` and `SwitchLogLevel` variants to `SrvCtlOp` enum in `src/tui/tab/srvctl.rs`
- [x] 1.2 Add `"Switch Mode"` and `"Switch Log Level"` display strings in `as_str()`
- [x] 1.3 Include both new variants in `SrvCtlOp::all()` return vec (append at end)

## 2. Add Esc key binding

- [x] 2.1 Add `Esc` variant to `SrvCtlKey` enum
- [x] 2.2 Add `([KeyCode::Esc], SrvCtlKey::Esc, "")` entry to `mod_agent!` macro invocation
- [x] 2.3 Add `KeyCode::Esc => Self::Esc` fallback arm in `TryFrom<&Key>` impl

## 3. Add selector state fields to content

- [x] 3.1 Add fields to `SrvCtlContent`:
  - `mode_selector_state: ListState`, `mode_selector_visible: bool`, `modes: Vec<crate::functions::restful::config_struct::Mode>`
  - `log_level_selector_state: ListState`, `log_level_selector_visible: bool`, `log_levels: Vec<crate::functions::restful::config_struct::LogLevel>`
- [x] 3.2 Update `SrvCtlContent::init()` to populate `self.modes` with `Mode::VARIANTS` and `self.log_levels` with `LogLevel::VARIANTS` (from strum), select index 0 in both selector states
- [x] 3.3 Verify `#[derive(Default)]` still works — all new bool fields default to `false`, `Vec` to empty, `ListState` to unselected

## 4. Key routing for selectors

- [x] 4.1 In `handle_key_event`, at the top: check `self.mode_selector_visible` and `self.log_level_selector_visible`, route keys to the active selector:
  - `MoveUp`: decrement selected index (saturating at 0)
  - `MoveDown`: increment selected index (guard against overflow)
  - `Esc`: set visibility to `false`, return
  - `Execute`: get selected value, set visibility to `false`, spawn async task to call `config::patch()` with the appropriate JSON payload, show success/error popup
  - Return early from all these branches
- [x] 4.2 In `handle_key_event`, in the `Execute` branch for the main list: add cases for `SrvCtlOp::SwitchMode` (set `mode_selector_visible = true`) and `SrvCtlOp::SwitchLogLevel` (set `log_level_selector_visible = true`), return early

## 5. Async tasks for PATCH /configs

- [x] 5.1 Mode switch task: call `config::patch(serde_json::json!({"mode": mode.to_string()}).to_string())`, show Confirm popup on success, `Confirm::err()` on failure
- [x] 5.2 Log level switch task: call `config::patch(serde_json::json!({"log-level": level.to_string()}).to_string())`, same success/error pattern

## 6. Render selector overlays

- [x] 6.1 In `render()`, after the main list: if `mode_selector_visible`, render a centered overlay (e.g. 60% w × 30% h of `area`) with `Clear` background, a bordered "Mode" list from `self.modes`, and `&mut self.mode_selector_state`
- [x] 6.2 Similarly for `log_level_selector_visible`: render centered overlay with bordered "Log Level" list from `self.log_levels` and `&mut self.log_level_selector_state`
- [x] 6.3 Use `Theme::get().tab.item_highlighted` for selector highlight style, matching the main list

## 7. Build and verify

- [x] 7.1 Run `cargo check` to verify compilation
- [x] 7.2 Run `cargo build` to verify full build
- [x] 7.3 Run `cargo test` to verify no regressions
