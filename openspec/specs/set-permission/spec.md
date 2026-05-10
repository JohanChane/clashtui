# set-permission Specification

## Purpose
TBD - created by archiving change migrate-clashsrvctl. Update Purpose after archive.
## Requirements
### Requirement: Set capabilities on clash binary
The system SHALL run `setcap cap_net_admin,cap_net_bind_service=+ep` on the clash binary path from config.

#### Scenario: is_user=true (no sudo needed)
- **WHEN** `service.is_user` is `true`
- **THEN** executes `setcap cap_net_admin,cap_net_bind_service=+ep <clash_bin_path>` directly without sudo

#### Scenario: is_user=false (needs sudo)
- **WHEN** `service.is_user` is `false`
- **THEN** first requests a sudo password via the TUI password input popup, then executes `sudo -S setcap cap_net_admin,cap_net_bind_service=+ep <clash_bin_path>` with the password piped to stdin

### Requirement: Password piped to sudo -S
When sudo is needed, the system SHALL pass the user's password to `sudo -S` via `std::process::Command.stdin()`.

#### Scenario: Valid password
- **WHEN** the correct sudo password is provided via the TUI input
- **THEN** `sudo -S` accepts the password, `setcap` runs successfully, and the output is returned

#### Scenario: Invalid password
- **WHEN** an incorrect sudo password is provided
- **THEN** `sudo -S` rejects the password, the function returns an error, and the user sees a "sudo: incorrect password" error message via `Confirm::err()`

### Requirement: setcap binary path resolution
The system SHALL resolve the `setcap` binary from the standard PATH (including `/usr/sbin`).

#### Scenario: setcap found in /usr/sbin
- **WHEN** `setcap` is at `/usr/sbin/setcap`
- **THEN** the command executes successfully

#### Scenario: setcap not found
- **WHEN** `setcap` is not available in PATH
- **THEN** the function returns an error indicating `setcap` could not be found

### Requirement: Report operation output
The system SHALL return the stdout and stderr of the setcap command for display to the user.

#### Scenario: Successful setcap with no output
- **WHEN** `setcap` succeeds and produces no output
- **THEN** the function returns an empty or minimal success string (e.g., "OK")

#### Scenario: Successful setcap with verbose output
- **WHEN** `setcap` succeeds and writes to stdout/stderr
- **THEN** the combined output is returned in the result

### Requirement: Use configured clash binary path
The system SHALL use `basic.clash_bin_path` from the config file as the target for setcap.

#### Scenario: Configured binary path
- **WHEN** `config.yaml` contains `basic.clash_bin_path: "/usr/bin/mihomo"`
- **THEN** setcap targets `/usr/bin/mihomo`

#### Scenario: Default binary path
- **WHEN** no `basic.clash_bin_path` is configured
- **THEN** setcap targets the default path `/usr/bin/mihomo`

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

