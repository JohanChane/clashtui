## ADDED Requirements

### Requirement: Tab displays operation list
The ClashSrvCtl tab SHALL render a vertically scrollable list of service-control operations (Start Service, Stop Service, Restart Service, Set Permission) in a bordered block with the tab title.

#### Scenario: Tab becomes visible
- **WHEN** user switches to the ClashSrvCtl tab
- **THEN** the tab renders a bordered list containing "Start Service", "Stop Service", "Restart Service", and "Set Permission" as selectable items

#### Scenario: Navigating the list
- **WHEN** user presses Up/k or Down/j while the tab is focused
- **THEN** the highlight moves to the previous or next item in the list

### Requirement: Execute selected operation on Enter
When the user presses Enter on a highlighted operation, the system SHALL dispatch the corresponding backend operation via an async task spawned into the FutureSet.

#### Scenario: Select and execute "Start Service"
- **WHEN** user highlights "Start Service" and presses Enter
- **THEN** an async task is spawned that calls the `start()` backend function with `clash_service_name` and `is_user` from config

#### Scenario: Select and execute "Stop Service"
- **WHEN** user highlights "Stop Service" and presses Enter
- **THEN** an async task is spawned that calls the `stop()` backend function

#### Scenario: Select and execute "Set Permission"
- **WHEN** user highlights "Set Permission" and presses Enter
- **THEN** an async task is spawned that calls `set_permission()` with `clash_bin_path` from config; if `is_user` is false, the task first requests a password via the TUI password input

### Requirement: Operation result feedback
The system SHALL display operation results to the user via inline status text or a Confirm popup.

#### Scenario: Successful operation
- **WHEN** a service operation completes successfully
- **THEN** a success status message is shown (inline or via Confirm popup with the command's stdout)

#### Scenario: Failed operation
- **WHEN** a service operation fails
- **THEN** the error message is displayed via `Confirm::err()` or inline error state

### Requirement: Key bindings configurable via keymap
The tab SHALL support user-customizable key bindings via the `keymap.yaml` file, using the `mod_agent!` macro pattern.

#### Scenario: Custom keymap overrides default bindings
- **WHEN** `keymap.yaml` contains a `srvctl` section with remapped keys
- **THEN** the remapped keys take effect and hardcoded defaults are ignored

#### Scenario: No custom keymap
- **WHEN** no `srvctl` section exists in `keymap.yaml`
- **THEN** hardcoded default keys (Enter, Up/k, Down/j, Esc) are used

### Requirement: Tab registered in application
The tab SHALL be registered as a new variant in the `Tab` enum, added to the `tabs` vec in `App::new()`, and integrated into the tab-switching key handler (digit keys).

#### Scenario: Tab appears in tab bar
- **WHEN** the application starts
- **THEN** the ClashSrvCtl tab is present in the tab bar and can be selected via the corresponding digit key

#### Scenario: Switched away from tab stops rendering
- **WHEN** user switches to a different tab
- **THEN** the ClashSrvCtl tab is marked non-visible and does not receive key events or render
