## ADDED Requirements

### Requirement: File-level permission repair as SrvCtl operation
The ClashSrvCtl tab SHALL expose file-level permission check and repair as an additional operation alongside the existing Stop, Restart, and SetPermission operations.

#### Scenario: New operation appears in SrvCtl list
- **WHEN** the ClashSrvCtl tab is opened
- **THEN** the operations list includes "Fix File Permissions" in addition to "Stop Service", "Start Service", and "Set Permission"

#### Scenario: User executes Fix File Permissions
- **WHEN** the user selects "Fix File Permissions" and presses Enter
- **THEN** the system checks permissions on the mihomo config directory, and if repair is needed, prompts for sudo password and performs the repair

#### Scenario: Permissions already correct
- **WHEN** "Fix File Permissions" is executed and all permissions are already correct
- **THEN** the system reports "Permissions are correct, no repair needed" without requesting sudo

### Requirement: Startup permission check
The system SHALL check mihomo config directory file permissions during TUI startup and prompt the user if repair is needed.

#### Scenario: Permissions incorrect at startup
- **WHEN** TUI starts and file permissions are incorrect
- **THEN** the system shows a Confirm popup: "File permissions in <dir> need repair. Fix now?" with Yes/No options

#### Scenario: User declines startup repair
- **WHEN** the user selects No on the startup repair prompt
- **THEN** the system proceeds normally without repairing; the user can repair later via the SrvCtl tab

#### Scenario: User accepts startup repair
- **WHEN** the user selects Yes on the startup repair prompt
- **THEN** the system prompts for sudo password (if needed) and performs the repair before continuing

#### Scenario: Permissions correct at startup
- **WHEN** TUI starts and all file permissions are already correct
- **THEN** no prompt is shown; the system proceeds directly to the TUI

### Requirement: Umask set to 0o002
The system SHALL set the process umask to `0o002` at startup so that newly created files in the config directory have group-read and group-write permissions.

#### Scenario: Umask set at startup
- **WHEN** the process starts (in `main()` before config init)
- **THEN** the process umask is set to `0o002`

#### Scenario: Newly created file is group-readable/writable
- **WHEN** demotui creates a new file under the config directory
- **THEN** the file has `0664` permissions (read/write for owner and group, read for others) assuming a create mode of `0666`
