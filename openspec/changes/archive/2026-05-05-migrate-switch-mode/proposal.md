## Why

Clash proxy mode (Rule/Direct/Global) and log level (Silent/Error/Warning/Info/Debug) are frequently toggled runtime settings. Currently, demotui's SrvCtl tab only manages service lifecycle and permissions — users must switch to the config file or another tool to change these. Migrating the SwitchMode and SwitchLogLevel sub-actions from clashtui gives users a quick, in-TUI way to toggle both from the same operations list they already use for service control.

## What Changes

- Add **"Switch Mode"** and **"Switch Log Level"** entries to the SrvCtl tab's operation list
- Selecting "Switch Mode" opens a mode selector overlay (Rule/Direct/Global) rendered as a centered list
- Selecting "Switch Log Level" opens a log level selector overlay (Silent/Error/Warning/Info/Debug)
- User navigates and confirms the target value; on confirmation, the clash API is patched via `PATCH /configs` with `{"mode": "<selected>"}` or `{"log-level": "<selected>"}`
- Operation result is shown as a success Confirm popup or error popup
- The existing SrvCtl tab keybindings (Enter/Up/Down/Esc) apply to both the main list and the selectors

## Capabilities

### New Capabilities

- `mode-switching`: Switch the clash proxy mode (Rule, Direct, Global) from within the SrvCtl tab via a mode-selector overlay that issues a `PATCH /configs` request to the clash REST API
- `log-level-switching`: Switch the clash log level (Silent, Error, Warning, Info, Debug) from within the SrvCtl tab via a log-level-selector overlay that issues a `PATCH /configs` request to the clash REST API

### Modified Capabilities

- `clashsrvctl-tab`: The operation list requirement changes to include "Switch Mode" and "Switch Log Level" items which open sub-selectors instead of dispatching immediate backend operations

## Impact

- `src/tui/tab/srvctl.rs` — add `SwitchMode` and `SwitchLogLevel` variants to `SrvCtlOp`, selector rendering/state logic, and the `PATCH /configs` async tasks
- `src/functions/restful.rs` — `config::patch()` is already available; used directly
- `src/functions/restful/config_struct.rs` — `Mode` and `LogLevel` enums already exist; reused for selector items
