## MODIFIED Requirements

### Requirement: Tab displays operation list
The ClashSrvCtl tab SHALL render a vertically scrollable list of service-control operations (Start Service, Stop Service, Set Permission, Fix File Permissions, Switch Mode, Switch Log Level) in a bordered block with the tab title.

#### Scenario: Tab becomes visible
- **WHEN** user switches to the ClashSrvCtl tab
- **THEN** the tab renders a bordered list containing "Start Service", "Stop Service", "Set Permission", "Fix File Permissions", "Switch Mode", and "Switch Log Level" as selectable items

#### Scenario: Navigating the list
- **WHEN** user presses Up/k or Down/j while the tab is focused
- **THEN** the highlight moves to the previous or next item in the list

### Requirement: Execute selected operation on Enter
When the user presses Enter on a highlighted operation, the system SHALL dispatch the corresponding action. For service control operations (Start, Stop, Set Permission, Fix File Permissions), an async task is spawned. For "Switch Mode" and "Switch Log Level", a selector overlay is displayed.

#### Scenario: Select and execute "Start Service"
- **WHEN** user highlights "Start Service" and presses Enter
- **THEN** an async task is spawned that calls the `start()` backend function with `clash_service_name` and `is_user` from config

#### Scenario: Select and execute "Stop Service"
- **WHEN** user highlights "Stop Service" and presses Enter
- **THEN** an async task is spawned that calls the `stop()` backend function

#### Scenario: Select and execute "Set Permission"
- **WHEN** user highlights "Set Permission" and presses Enter
- **THEN** an async task is spawned that calls `set_permission()` with `clash_bin_path` from config; if `is_user` is false, the task first requests a password via the TUI password input

#### Scenario: Select "Switch Mode" opens mode selector
- **WHEN** user highlights "Switch Mode" and presses Enter
- **THEN** a centered overlay appears showing "Rule", "Direct", and "Global" as selectable mode options

#### Scenario: Select "Switch Log Level" opens log level selector
- **WHEN** user highlights "Switch Log Level" and presses Enter
- **THEN** a centered overlay appears showing "Silent", "Error", "Warning", "Info", and "Debug" as selectable log level options

### Requirement: Key bindings configurable via keymap
The tab SHALL support user-customizable key bindings via the `keymap.yaml` file, using the `mod_agent!` macro pattern. The default bindings SHALL include Esc to dismiss the mode or log level selector.

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
