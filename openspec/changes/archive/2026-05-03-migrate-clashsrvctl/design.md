## Context

Demotui already has:
- `src/config/core.rs` — `Service { clash_service_name, is_user }` and `ServiceController` enum (Systemd/OpenRc/Nssm) with `.args(work_type, service_name, is_user)` generating the right CLI flags
- `src/functions/command.rs` — `restart_service()` / `stop_service()` using `ServiceController`
- `src/functions/command/linux.rs` — `run_as_su_by_sudo()` that calls `tui::hold(true)` → `sudo <cmd>` → `tui::hold(false)` (restores terminal raw-mode, breaking TUI)
- `src/tui/popmsg/input.rs` — plain-text `Input` widget using `Msg`, `Route`, oneshot channel via `PAIR`

Clashtui's ClashSrvCtl uses pkexec for privilege elevation (GUI prompt, blocks the TUI). Neither project has a TUI-native password input.

The goal is a new ClashSrvCtl tab with service start/stop/restart + setcap operations, using a TUI-native masked password input when sudo is needed.

## Goals / Non-Goals

**Goals:**
- Provide a TUI tab listing service operations (Start, Stop, Restart, Set Permission)
- Implement TUI-native masked password input widget for collecting sudo passwords
- Execute service-control commands (systemctl/rc-service) respecting `is_user` flag
- Execute setcap via `sudo -S` with password from TUI input
- Register the tab into the existing tab system with key bindings

**Non-Goals:**
- Port `SwitchMode` or `CloseConnections` operations (clashtui-specific, unrelated to service control)
- Windows Nssm support (uses non-interactive elevation; out of scope)
- Modify `tui::hold()` or `run_as_su_by_sudo()` (leave existing infrastructure untouched)
- Add new config fields (use existing `service.is_user`, `service.clash_service_name`, `basic.clash_bin_path`, `hack.service_controller`)

## Decisions

### D1: New `InputMasked` widget type (NOT a boolean flag on `Input`)

**Rationale:** `Input` is a concrete type implementing `Msg<Result = String>`. Adding a boolean field would change its semantics (render/size behavior differs). Rust's type system rewards clarity. A separate `InputMasked` type implementing the same `Msg<Result = String>` trait is minimal code (mostly reuse via helper methods), avoids `if password_mode` branches everywhere, and makes the sender's intent explicit.

**Alternatives considered:**
- `Input { password_mode: bool }` — simpler but pollutes every method with conditionals; confusing for non-password callers
- Builder pattern: `Input::new().password()` — same issue as above

### D2: Pipe password to `sudo -S` stdin (NOT pkexec or terminal restore)

**Rationale:** The `sudo -S` flag reads password from stdin, which we control programmatically. This keeps the entire flow within the TUI:
1. User selects operation → password popup appears in TUI
2. User types password (masked) → presses Enter
3. Password is fed to `sudo -S` via `std::process::Command.stdin()`
4. Command executes, output captured

**Alternatives considered:**
- pkexec — requires GUI polkit agent, blocks TUI (clashtui approach)
- `tui::hold()` pattern — restores terminal raw-mode, user types in bare terminal (current demotui approach)
- Setuid binary — overkill for a TUI app

**Risk:** Password lives in process memory briefly. Acceptable — same as any `sudo` invocation. The password string is dropped after `Command::stdin()` call.

### D3: Async backend via `FutureSet` with `tri!` error handling

**Rationale:** Follow existing demotui patterns. When user selects an operation:
1. `handle_key_event` spawns an async block into `FutureSet`
2. Async block calls password popup if needed, then runs the command
3. Result/error stored in content state, rendered inline or via Confirm popup

This is non-blocking from the TUI's perspective — the UI keeps rendering at 50fps while the command runs.

### D4: Backend logic in `src/functions/command/srvctl.rs`

**Rationale:** Keep tab code focused on UI. The new module provides free functions:
- `start(service_name, is_user)` → `Result<String>`
- `stop(service_name, is_user)` → `Result<String>`
- `restart(service_name, is_user)` → `Result<String>` (stop + start)
- `set_permission(bin_path, password)` → `Result<String>` (if `is_user=false`, prompts for password; if `is_user=true`, runs directly without sudo)

The tab module imports these and orchestrates the user interaction (password popup, status display).

### D5: Service name cache in tab content

**Rationale:** The service name (`service.clash_service_name`) and bin path (`basic.clash_bin_path`) are read once at tab init from `crate::config::CONFIG` and stored in the content struct. No need to re-read config on every operation.

### D6: Key binding structure

Following the `mod_agent!` pattern from `connections.rs`:
- **Enter**: Execute selected operation
- **Up/k / Down/j**: Navigate operation list
- **Esc**: Cancel/pop up

Service name is displayed at the top of the render area; the list shows operations.

## Risks / Trade-offs

- [Risk] `sudo -S` may not work if `requiretty` is set in `/etc/sudoers` → Mitigation: document that `Defaults !requiretty` or `NOPASSWD` may be needed, same as any programmatic sudo usage
- [Risk] Invalid password produces sudo error output → Mitigation: parse sudo stderr for "incorrect password" messages, display user-friendly error via `Confirm::err()`; allow retry
- [Risk] OpenRc detection uses file check (`/sbin/rc-service`), which already exists in `ServiceController::default()` logic → No additional risk
- [Trade-off] Password stays in memory until command completes → Acceptable; `stdin` closure drops the password bytes after the write; memory may still retain them until overwritten. No different from other sudo-using tools
