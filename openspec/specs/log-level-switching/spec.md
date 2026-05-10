# log-level-switching Specification

## Purpose
TBD - created by archiving change migrate-switch-mode. Update Purpose after archive.
## Requirements
### Requirement: Log level selector overlay appears on "Switch Log Level" selection
When the user selects "Switch Log Level" from the SrvCtl tab operation list, the system SHALL display a centered log-level-selector overlay listing the five log levels: Silent, Error, Warning, Info, and Debug. The main operation list SHALL remain visible behind the overlay.

#### Scenario: Selecting "Switch Log Level" opens the overlay
- **WHEN** the user highlights "Switch Log Level" in the operation list and presses Enter
- **THEN** a centered overlay appears showing five selectable items: "Silent", "Error", "Warning", "Info", "Debug"
- **AND** the first item ("Silent") is highlighted

#### Scenario: Overlay renders on top of main list
- **WHEN** the log level selector overlay is visible
- **THEN** the overlay is rendered as a bordered list centered in the tab area, with the background cleared so the main list does not show through

### Requirement: Log level selector navigation
While the log level selector overlay is visible, the system SHALL route Up/Down key events (and their k/j aliases) to move the selector cursor, Enter to confirm the selection, and Esc to dismiss the overlay.

#### Scenario: Navigate log level selector with Up/Down
- **WHEN** the log level selector is visible and user presses Up or k
- **THEN** the highlight moves to the previous item (saturating at the top)

#### Scenario: Navigate log level selector with Down
- **WHEN** the log level selector is visible and user presses Down or j
- **THEN** the highlight moves to the next item (saturating at the bottom)

#### Scenario: Confirm log level selection with Enter
- **WHEN** the log level selector is visible, a level is highlighted, and user presses Enter
- **THEN** the overlay closes and an async task is spawned to patch the clash config with the selected log level

#### Scenario: Dismiss log level selector with Esc
- **WHEN** the log level selector is visible and user presses Esc
- **THEN** the overlay closes without changing the log level

### Requirement: Log level switching via clash REST API
When the user confirms a log level selection, the system SHALL send a `PATCH /configs` request to the clash external controller with a JSON payload `{"log-level": "<lowercase-level-name>"}`.

#### Scenario: Successful log level switch to "Debug"
- **WHEN** user confirms "Debug" in the log level selector
- **THEN** the system issues `PATCH /configs` with body `{"log-level": "debug"}`
- **AND** on success, a Confirm popup displays a success message with the API response

#### Scenario: Failed log level switch
- **WHEN** the `PATCH /configs` request fails (connection error, non-2xx response, etc.)
- **THEN** the system shows an error popup via `Confirm::err()`

#### Scenario: Esc during log level selector discards without API call
- **WHEN** user presses Esc while the log level selector is visible
- **THEN** no API request is made and the current log level is unchanged

