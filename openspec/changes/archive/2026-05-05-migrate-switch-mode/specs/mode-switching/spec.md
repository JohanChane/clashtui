## ADDED Requirements

### Requirement: Mode selector overlay appears on "Switch Mode" selection
When the user selects "Switch Mode" from the SrvCtl tab operation list, the system SHALL display a centered mode-selector overlay listing the three proxy modes: Rule, Direct, and Global. The main operation list SHALL remain visible behind the overlay.

#### Scenario: Selecting "Switch Mode" opens the overlay
- **WHEN** the user highlights "Switch Mode" in the operation list and presses Enter
- **THEN** a centered overlay appears showing three selectable items: "Rule", "Direct", "Global"
- **AND** the first item ("Rule") is highlighted

#### Scenario: Overlay renders on top of main list
- **WHEN** the mode selector overlay is visible
- **THEN** the overlay is rendered as a bordered list centered in the tab area, with the background cleared so the main list does not show through

### Requirement: Mode selector navigation
While the mode selector overlay is visible, the system SHALL route Up/Down key events (and their k/j aliases) to move the selector cursor, Enter to confirm the selection, and Esc to dismiss the overlay.

#### Scenario: Navigate mode selector with Up/Down
- **WHEN** the mode selector is visible and user presses Up or k
- **THEN** the highlight moves to the previous item (wrapping or saturating at the top)

#### Scenario: Navigate mode selector with Down
- **WHEN** the mode selector is visible and user presses Down or j
- **THEN** the highlight moves to the next item (saturating at the bottom)

#### Scenario: Confirm mode selection with Enter
- **WHEN** the mode selector is visible, a mode is highlighted, and user presses Enter
- **THEN** the overlay closes and an async task is spawned to patch the clash config with the selected mode

#### Scenario: Dismiss mode selector with Esc
- **WHEN** the mode selector is visible and user presses Esc
- **THEN** the overlay closes without changing the proxy mode

### Requirement: Mode switching via clash REST API
When the user confirms a mode selection, the system SHALL send a `PATCH /configs` request to the clash external controller with a JSON payload `{"mode": "<lowercase-mode-name>"}`.

#### Scenario: Successful mode switch to "Rule"
- **WHEN** user confirms "Rule" in the mode selector
- **THEN** the system issues `PATCH /configs` with body `{"mode": "rule"}`
- **AND** on success, a Confirm popup displays a success message with the API response

#### Scenario: Failed mode switch
- **WHEN** the `PATCH /configs` request fails (connection error, non-2xx response, etc.)
- **THEN** the system shows an error popup via `Confirm::err()`

#### Scenario: Esc during mode selector discards without API call
- **WHEN** user presses Esc while the mode selector is visible
- **THEN** no API request is made and the current proxy mode is unchanged
