## 1. Password input widget

- [x] 1.1 Add `InputMasked` struct to `src/tui/popmsg/input.rs` — a masked text input that implements `Msg<Result = String>`, renders characters as `●`, supports full editing (insert, backspace, delete, cursor movement), sends buffer on Enter, drops on Esc
- [x] 1.2 Add `InputMasked::new()` and `InputMasked::with_title()` builder methods, matching the `Input` widget's API for integration with `MsgBuilder`

## 2. Set-permission backend

- [x] 2.1 Add `set_permission(bin_path: &str, password: Option<&str>) -> Result<String>` function to `src/functions/command.rs`: runs `setcap cap_net_admin,cap_net_bind_service=+ep` on the binary; if `password` is `Some`, wraps with `sudo -S` piped via stdin; returns command output or error
- [x] 2.2 Add `/usr/sbin` to PATH resolution in `set_permission` (setcap typically lives there)
- [x] 3.1 Create `src/tui/tab/srvctl.rs` with `SrvCtlContent` struct implementing `BasicTabContent<Key = SrvCtlKey, State = ListState>` and `TabContent`
- [x] 3.2 Define `SrvCtlKey` enum with `mod_agent!` macro — default bindings: Enter=Execute, Up/J=MoveUp, Down/K=MoveDown, Esc=Back; include help descriptions for the "?" overlay
- [x] 3.3 Define the operation enum `SrvCtlOp` with variants: Start, Stop, Restart, SetPermission; store as a `Vec<SrvCtlOp>` list with display strings
- [x] 3.4 Implement `init()` — read `clash_service_name`, `clash_bin_path`, `is_user` from `crate::config::CONFIG` and store in content struct
- [x] 3.5 Implement `handle_key_event()` for `Execute` — on Enter, spawn async block into FutureSet that dispatches the selected operation:
  - Start → call `restart_service()` (which does restart + status)
  - Stop → call `stop_service()`
  - Restart → call `restart_service()`
  - SetPermission → if `is_user`, call `set_permission(bin_path, None)`; if not `is_user`, first show `InputMasked` popup for password, then call `set_permission(bin_path, Some(&password))`
  - All use `tri!` and `wrapper()`/`Confirm` for result/error feedback
- [x] 3.6 Implement `render()` — bordered block with service name in title, a `List` of `SrvCtlOp` items with highlight styling, selected item highlighted
- [x] 3.7 Implement `handle_key_event()` for MoveUp/MoveDown — delegate to `ListState::select_previous()`/`select_next()`

## 4. Tab registration

- [x] 4.1 Add newtype wrapper `SrvCtlTab(Tab<SrvCtlContent>)` with `newtype_tab!` macro in `src/tui/tab/mod.rs`
- [x] 4.2 Register `SrvCtlTab` in the `enum_dispatch!` block and `prelude` re-exports in `src/tui/tab/mod.rs`
- [x] 4.3 Add `agent_init` call for `"srvctl"` keymap section in `src/tui/tab/mod.rs` `agent_init()` function
- [x] 4.4 Add `use` import for `srvctl` module via `mod srvctl;` in `src/tui/tab/mod.rs`
- [x] 4.5 Add `SrvCtlTab::default().into()` to the `tabs` vec in `src/tui/app.rs` `App::new()`
- [x] 4.6 Update `TAB_COUNT` and digit-key match arm in `src/tui/app.rs` to include the new tab
- [x] 4.7 Add `SrvCtlTab` to the `update_tabbar` visibility match in `src/tui/app.rs`

## 5. Verification

- [x] 5.1 Run `cargo check` and fix any type errors
- [x] 5.2 Run `cargo test` to ensure no regressions
- [x] 5.3 Run `cargo build` for a full compile check
