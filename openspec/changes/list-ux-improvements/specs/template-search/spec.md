## ADDED Requirements

### Requirement: Template search via PopUp

The system SHALL provide a `/` key-triggered search PopUp in the Template tab that filters the template list by substring match.

#### Scenario: Trigger template search

- **WHEN** the user presses `/` while the Template tab is focused
- **THEN** an Input PopUp SHALL appear with title "Filter"
- **AND** the current filter value (if any) SHALL be pre-filled in the input field

#### Scenario: Apply template filter

- **WHEN** the user enters a filter string "my-tpl" and confirms
- **THEN** the template list SHALL display only templates whose names contain "my-tpl"
- **AND** the filter text SHALL appear in the bottom-right of the template pane's border

#### Scenario: Clear template filter

- **WHEN** the user submits an empty filter string
- **THEN** the filter SHALL be cleared
- **AND** all templates SHALL be displayed

#### Scenario: Cancel template search

- **WHEN** the user presses Esc or cancels the PopUp
- **THEN** the filter SHALL remain unchanged
