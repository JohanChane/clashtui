## Why

Demotui currently has no dedicated service-control tab for managing the clash core service. The existing `functions/command` module can restart/stop services but without a user-facing TUI. Sudo password handling uses `tui::hold()` to restore terminal raw-mode, forcing the user to type their password in the terminal beneath the TUI overlay — breaking the TUI experience. We need to bring clash service management into a native TUI tab with an integrated password input that keeps the user in the TUI.

## What Changes

- Add a **ClashSrvCtl tab** with an interactive list of service operations (Start, Stop, Restart, Set Permission)
- Add a **TUI-native password input widget** (masked text entry via PopUp) for sudo password collection, replacing pkexec/terminal-restore approaches
- Implement **service control logic** using systemctl/rc-service with `is_user` config flag to determine `--user` mode vs root
- Implement **setcap/set-permission logic** running setcap via `sudo -S` with password piped from the TUI input
- Register the tab in the app (enum, tabbar, key bindings, tab count)

## Capabilities

### New Capabilities
- `clashsrvctl-tab`: TUI tab with a list of service-control operations; selecting an operation dispatches async work with live status feedback via the Msg popup or inline state
- `password-input`: Masked-input PopUp widget that collects a password string (characters hidden as `*` or `●`) from the user and sends it via the existing oneshot channel pattern
- `service-control`: Start, stop, and restart the clash service via systemctl/rc-service, respecting the existing `service.is_user` config field for `--user` mode
- `set-permission`: Run `setcap cap_net_admin,cap_net_bind_service=+ep` on the clash binary path; if `is_user` is false, request sudo password via the TUI password input and pipe it to `sudo -S`

### Modified Capabilities
<!-- None: no existing specs to modify -->

## Impact

- **New files**: `src/tui/tab/srvctl.rs` (tab content + newtype wrapper)
- **New or modified files**: `src/tui/popmsg/input.rs` (add password/masked mode to existing `Input` widget)
- **Modified files**: `src/tui/tab/mod.rs` (register tab variant, enum dispatch, agent init), `src/tui/app.rs` (add tab to vec, update TAB_COUNT, digit-key arm)
- **New file**: `src/functions/command/srvctl.rs` (service control + setcap logic)
- **Modified file**: `src/functions/command.rs` (re-export srvctl module)
- **Config**: Uses existing `service.clash_service_name` and `service.is_user` from `ConfigFile` — no new config fields needed
- **Dependencies**: No new crate dependencies
