## ADDED Requirements

### Requirement: Profile delete with confirmation

The system SHALL require user confirmation via a Confirm PopUp before deleting a profile's YAML file and database record. The delete action SHALL be triggered by the `dd` chord (pressing `d` twice).

#### Scenario: Delete profile with confirmation

- **WHEN** the user presses `d` then `d` on a selected profile in the Profile tab
- **THEN** a Confirm PopUp SHALL appear asking "Delete profile <name>?"
- **AND** upon confirmation (Enter/y), the profile's YAML file SHALL be removed from `profile_yamls/`
- **AND** the profile database record SHALL be removed
- **AND** the profile list SHALL be refreshed

#### Scenario: Cancel profile delete

- **WHEN** the user presses `dd` then cancels the confirmation (Esc/n/q)
- **THEN** no files SHALL be deleted
- **AND** the profile SHALL remain in the list

#### Scenario: Delete profile fails gracefully

- **WHEN** the file removal fails (e.g., file in use or already deleted)
- **THEN** an error message SHALL be displayed via Confirm PopUp
- **AND** the database record removal SHALL still be attempted

### Requirement: Template delete with confirmation

The system SHALL require user confirmation via a Confirm PopUp before deleting a template file. The delete action SHALL be triggered by the `dd` chord.

#### Scenario: Delete template with confirmation

- **WHEN** the user presses `d` then `d` on a selected template in the Template tab
- **THEN** a Confirm PopUp SHALL appear asking "Delete template <name>?"
- **AND** upon confirmation, the template file SHALL be removed from `templates/`
- **AND** the template list SHALL be refreshed

#### Scenario: Cancel template delete

- **WHEN** the user presses `dd` then cancels the confirmation
- **THEN** no files SHALL be deleted
- **AND** the template SHALL remain in the list

### Requirement: No single-key delete conflict

The profile and template tabs SHALL NOT have a single-key `d` binding that would prevent the `dd` chord from being recognized.

#### Scenario: Single d does not trigger delete

- **WHEN** the user presses `d` once in the Profile or Template tab
- **THEN** the ChordHandler SHALL enter chord mode waiting for the second key
- **AND** no delete action SHALL be triggered until the second `d` is pressed
